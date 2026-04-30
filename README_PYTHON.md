# rpmrepo_metadata

[![PyPI](https://img.shields.io/pypi/v/rpmrepo-metadata.svg)](https://pypi.org/project/rpmrepo-metadata/)
[![Python](https://img.shields.io/pypi/pyversions/rpmrepo-metadata.svg)](https://pypi.org/project/rpmrepo-metadata/)

A Python library for reading, writing, and managing RPM repository metadata. Built on a Rust core for performance.

RPM repository metadata consists of several XML files — `primary.xml`, `filelists.xml`, `other.xml`, `repomd.xml`, `updateinfo.xml`, and `comps.xml` — that together describe the packages available in a repository. This library provides high-level APIs for working with all of these metadata types.

## Installation

```sh
pip install rpmrepo_metadata
```

Requires Python >= 3.10. Pre-built wheels are available for Linux, macOS, and Windows.

## Examples

---

### Read a repository and iterate packages

Use `RepositoryReader` to stream through packages without loading everything into memory.

```python
from rpmrepo_metadata import RepositoryReader

reader = RepositoryReader("path/to/repo/")

packages = reader.iter_packages()
print(f"Total packages: {packages.total_packages}")

for pkg in packages:
    print(f"{pkg.nevra()} - {pkg.summary}")
    print(f"  Size: {pkg.size_package} bytes")
    print(f"  Checksum: {pkg.checksum}")
    print(f"  Location: {pkg.location_href}")
```

### Read advisories (updateinfo)

```python
from rpmrepo_metadata import RepositoryReader

reader = RepositoryReader("path/to/repo/")

for advisory in reader.iter_advisories():
    print(f"[{advisory.update_type}] {advisory.id} - {advisory.title}")
    print(f"  Severity: {advisory.severity}")
    print(f"  Issued: {advisory.issued_date}")

    for ref in advisory.references:
        print(f"  {ref.reftype}: {ref.href}")

    for collection in advisory.pkglist:
        for pkg in collection.packages:
            print(f"  Package: {pkg.name}-{pkg.version}-{pkg.release}.{pkg.arch}")
```

### Read comps data (groups, categories, environments)

```python
from rpmrepo_metadata import RepositoryReader

reader = RepositoryReader("path/to/repo/")

comps = reader.read_comps()
if comps is not None:
    for group in comps.groups:
        print(f"Group: {group.name} ({group.id})")
        for pkg_req in group.packages:
            print(f"  {pkg_req.reqtype}: {pkg_req.name}")

    for category in comps.categories:
        print(f"Category: {category.name}")
        for group_id in category.group_ids:
            print(f"  Group: {group_id}")

    for env in comps.environments:
        print(f"Environment: {env.name} ({env.id})")
        for group_id in env.group_ids:
            print(f"  Group: {group_id}")
        for option in env.option_ids:
            print(f"  Optional: {option.group_id} (default={option.default})")
```

### Create and populate a Package

```python
from rpmrepo_metadata import Package

pkg = Package()
pkg.name = "my-package"
pkg.epoch = 0
pkg.version = "1.2.3"
pkg.release = "4.el9"
pkg.arch = "x86_64"

pkg.summary = "An example package"
pkg.description = "A longer description of the package"
pkg.url = "https://example.com"
pkg.rpm_license = "MIT"

pkg.checksum = ("sha256", "a" * 64)
pkg.location_href = "Packages/m/my-package-1.2.3-4.el9.x86_64.rpm"

pkg.size_package = 12345
pkg.size_installed = 67890
pkg.time_build = 1700000000

# Dependencies are tuples of (name, flags, epoch, version, release, preinstall)
pkg.requires = [
    ("glibc", "GE", "0", "2.17", "", False),
    ("bash", None, None, None, None, False),
]
pkg.provides = [("my-package", "EQ", "0", "1.2.3", "4.el9", False)]

# Files are tuples of (type, path) where type is None, "dir", or "ghost"
pkg.files = [
    (None, "/usr/bin/my-package"),
    ("dir", "/etc/my-package"),
]

# Changelogs are tuples of (author, timestamp, description)
pkg.changelogs = [
    ("John Doe <john@example.com>", 1700000000, "- Initial release"),
]

print(pkg.nevra())       # "my-package-0:1.2.3-4.el9.x86_64"
print(pkg.nevra_short())  # "my-package-1.2.3-4.el9.x86_64" (omits epoch 0)
print(pkg.pkgid)          # "aaaa...aaaa"
```

### Read an RPM file directly

Extract metadata from `.rpm` files on disk.

```python
from rpmrepo_metadata import Package, ChecksumType

# Using defaults (SHA-256 checksum, 10 changelog entries)
pkg = Package.from_file("packages/foo-1.0-1.el9.x86_64.rpm")
print(f"{pkg.nevra()} - {len(pkg.files)} files")

# With custom options
pkg = Package.from_file_with_options(
    "packages/foo-1.0-1.el9.x86_64.rpm",
    checksum_type=ChecksumType.Sha512,
    location_href="Packages/f/foo-1.0-1.el9.x86_64.rpm",
    location_base="https://example.com/repo/",
    changelog_limit=5,
)
print(f"Checksum type: {pkg.checksum_type}")  # "sha512"
print(f"Location: {pkg.location_href}")
```

### Write a repository with RepositoryWriter

Stream packages to disk one at a time, keeping memory usage low.

```python
from rpmrepo_metadata import RepositoryWriter, Package

writer = RepositoryWriter("output/repo/", num_pkgs=100)

# Add packages
for rpm_path in rpm_files:
    pkg = Package.from_file(rpm_path)
    writer.add_package(pkg)

# Add advisories
advisory = UpdateRecord()
advisory.id = "EXAMPLE-2024:001"
advisory.title = "Important security fix"
advisory.update_type = "security"
advisory.severity = "Important"
writer.add_advisory(advisory)

# Finalize — writes repomd.xml and closes all files
writer.finish()
```

### Work with Repository in-memory

`Repository` loads all metadata into memory, convenient for smaller repositories.

```python
from rpmrepo_metadata import Repository

# Load from disk
repo = Repository.load_from_directory("path/to/repo/")

# Write to a new location
repo.write_to_directory("output/repo/")
```

### Parse and compare EVR version strings

```python
from rpmrepo_metadata import EVR

evr1 = EVR.parse("1:2.3.4-5.el9")
evr2 = EVR.parse("2.3.4-6.el9")

print(f"{evr1} vs {evr2}")
print(f"epoch={evr1.epoch}, version={evr1.version}, release={evr1.release}")

# Comparison operators
assert evr1 > evr2  # epoch 1 beats no epoch
assert EVR.parse("1.0-1") == EVR.parse("0:1.0-1")  # epoch 0 is the default
assert EVR.parse("1.0-2") > EVR.parse("1.0-1")

# Destructure into components
epoch, version, release = evr1.components()
```

### Parse comps XML from a string

```python
from rpmrepo_metadata import CompsData

xml = """<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE comps PUBLIC "-//Red Hat, Inc.//DTD Comps info//EN" "comps.dtd">
<comps>
  <group>
    <id>core</id>
    <name>Core</name>
    <description>Minimal system</description>
    <packagelist>
      <packagereq type="mandatory">bash</packagereq>
      <packagereq type="mandatory">coreutils</packagereq>
    </packagelist>
  </group>
</comps>"""

comps = CompsData.from_xml(xml)
print(f"{len(comps.groups)} groups")

# Serialize back to XML
output_xml = comps.to_xml()
```

### Work with UpdateRecord (advisories)

```python
from rpmrepo_metadata import (
    UpdateRecord, UpdateReference, UpdateCollection,
    UpdateCollectionPackage,
)

record = UpdateRecord()
record.id = "RHSA-2024:1234"
record.title = "Critical: kernel security update"
record.update_type = "security"
record.severity = "Critical"
record.issued_date = "2024-03-15"
record.summary = "An update for kernel is now available."
record.description = "The kernel packages contain the Linux kernel."

# Add a reference
ref = UpdateReference(
    href="https://bugzilla.redhat.com/show_bug.cgi?id=12345",
    id="12345",
    title="kernel vulnerability",
    reftype="bugzilla",
)
record.references = [ref]

# Add affected packages
pkg = UpdateCollectionPackage()
pkg.name = "kernel"
pkg.version = "5.14.0"
pkg.release = "362.24.1.el9_3"
pkg.arch = "x86_64"
pkg.epoch = "0"
pkg.filename = "kernel-5.14.0-362.24.1.el9_3.x86_64.rpm"

collection = UpdateCollection(name="Red Hat Enterprise Linux 9", shortname="RHEL-9")
collection.packages = [pkg]
record.pkglist = [collection]
```

## Rust library

This package is built on a Rust library of the same name, also available on [crates.io](https://crates.io/crates/rpmrepo_metadata). See the [Rust README](https://github.com/dralley/rpmrepo_metadata/blob/main/README.md) and [API documentation](https://docs.rs/rpmrepo_metadata/) for Rust usage.

## License

[Mozilla Public License 2.0](https://github.com/dralley/rpmrepo_metadata/blob/main/LICENSE)
