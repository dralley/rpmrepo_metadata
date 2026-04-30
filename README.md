# rpmrepo_metadata

[![crates.io](https://img.shields.io/crates/v/rpmrepo_metadata.svg)](https://crates.io/crates/rpmrepo_metadata)
[![docs.rs](https://docs.rs/rpmrepo_metadata/badge.svg)](https://docs.rs/rpmrepo_metadata)
[![PyPI](https://img.shields.io/pypi/v/rpmrepo-metadata.svg)](https://pypi.org/project/rpmrepo-metadata/)

A library for reading, writing, and managing RPM repository metadata.

RPM repository metadata consists of several XML files — `primary.xml`, `filelists.xml`, `other.xml`, `repomd.xml`, `updateinfo.xml`, and `comps.xml` — that together describe the packages available in a repository. This crate provides both high-level APIs (`Repository`, `RepositoryReader`, `RepositoryWriter`) and lower-level streaming readers/writers for each metadata type.

This is **not** a reimplementation of `createrepo_c` — it is a library for working with the metadata files themselves, suitable for building tools on top of.

## Features

- [x] Read and write all standard RPM repository metadata types
- [x] High-level `Repository` API for in-memory manipulation
- [x] Streaming `RepositoryReader` / `RepositoryWriter` for large repositories
- [x] Read RPM package files directly (with `read_rpm` feature)
- [x] Advisory (updateinfo) and comps (group/category/environment) support
- [x] Python bindings (available on [PyPI](https://pypi.org/project/rpmrepo-metadata/))

## Examples

---

### Read a repository and iterate packages

Use `RepositoryReader` to stream through packages without loading everything into memory.

```rust
use rpmrepo_metadata::RepositoryReader;
use std::path::Path;

let reader = RepositoryReader::new_from_directory(Path::new("tests/assets/external_repos/centos7/"))?;

println!("Revision: {:?}", reader.repomd().revision());

for pkg in reader.iter_packages()? {
    let pkg = pkg?;
    println!("{} - {}", pkg.nevra(), pkg.summary());
}
```

### Read advisories and comps data

```rust
use rpmrepo_metadata::RepositoryReader;
use std::path::Path;

let reader = RepositoryReader::new_from_directory(Path::new("repo/"))?;

// Iterate advisory records from updateinfo.xml
for advisory in reader.iter_advisories()? {
    let advisory = advisory?;
    println!("[{}] {} - {}", advisory.update_type, advisory.id, advisory.title);
    for reference in &advisory.references {
        println!("  {} {}", reference.reftype, reference.href);
    }
}

// Read comps (group) metadata if present
if let Some(comps) = reader.read_comps()? {
    for group in &comps.groups {
        println!("Group: {} ({})", group.name, group.id);
    }
    for env in &comps.environments {
        println!("Environment: {} ({})", env.name, env.id);
    }
}
```

### Read an RPM file directly

With the `read_rpm` feature (enabled by default), you can extract metadata from `.rpm` files.

```rust
use rpmrepo_metadata::{Package, PackageOptions, ChecksumType};

// Using defaults (SHA-256 checksum, 10 changelog entries)
let pkg = Package::from_file("packages/foo-1.0-1.el9.x86_64.rpm")?;
println!("{} {} files", pkg.nevra(), pkg.files().len());

// With custom options
let options = PackageOptions {
    checksum_type: ChecksumType::Sha512,
    location_href: Some("Packages/f/foo-1.0-1.el9.x86_64.rpm".to_string()),
    changelog_limit: 5,
    ..Default::default()
};
let pkg = Package::from_file_with_options("packages/foo-1.0-1.el9.x86_64.rpm", options)?;
```

### Build a repository with RepositoryWriter

Use `RepositoryWriter` for streaming writes — packages are written to disk as they are added, keeping memory usage low.

```rust
use rpmrepo_metadata::{
    RepositoryWriter, RepositoryOptions, Package, ChecksumType,
    CompressionType, UpdateRecord,
};
use std::path::Path;

let options = RepositoryOptions::default()
    .compression_type(CompressionType::Zstd)
    .checksum_type(ChecksumType::Sha256);

let mut writer = RepositoryWriter::new_with_options(
    Path::new("output/repo/"),
    num_packages,
    options,
)?;

// Add packages one at a time
for rpm_path in rpm_files {
    let pkg = Package::from_file(&rpm_path)?;
    writer.add_package(&pkg)?;
}

// Add advisories
let mut advisory = UpdateRecord::default();
advisory.id = "EXAMPLE-2024:001".to_string();
advisory.title = "Important security fix".to_string();
advisory.update_type = "security".to_string();
advisory.severity = "Important".to_string();
writer.add_advisory(&advisory)?;

// Finalize — writes repomd.xml and closes all files
writer.finish()?;
```

### Work with Repository in-memory

`Repository` loads all metadata into memory, which is convenient for smaller repositories or when you need random access.

```rust
use rpmrepo_metadata::{Repository, RepositoryOptions, CompressionType};
use std::path::Path;

// Load
let mut repo = Repository::load_from_directory(Path::new("repo/"))?;

println!("{} packages", repo.packages().len());
println!("{} advisories", repo.advisories().len());

// Modify
repo.sort();

// Write with options
let options = RepositoryOptions::default()
    .compression_type(CompressionType::Xz)
    .simple_metadata_filenames(true);
repo.write_to_directory_with_options(Path::new("output/repo/"), options)?;
```

### Parse and compare EVR version strings

```rust
use rpmrepo_metadata::EVR;

let evr1 = EVR::parse("1:2.3.4-5.el9");
let evr2 = EVR::parse("2.3.4-6.el9");

assert!(evr1 > evr2); // epoch 1 beats no epoch

println!("{} vs {}", evr1, evr2);
println!("epoch={}, version={}, release={}", evr1.epoch(), evr1.version(), evr1.release());
```

### Read and write individual metadata types

You can work with specific metadata files without going through the full repository API.

```rust
use rpmrepo_metadata::{Repository, PrimaryXml, RepomdXml};

// Parse a single metadata type from a string
let mut repo = Repository::new();
repo.load_metadata_str::<PrimaryXml>(primary_xml_content)?;

// Write a single metadata type to a string
let xml_output = repo.write_metadata_string::<PrimaryXml>()?;
```

## Python bindings

Python bindings are available on [PyPI](https://pypi.org/project/rpmrepo-metadata/). See the [Python README](README_PYTHON.md) for installation and usage.

```sh
pip install rpmrepo_metadata
```

## License

[Mozilla Public License 2.0](LICENSE)
