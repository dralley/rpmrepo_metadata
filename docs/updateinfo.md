# Updateinfo: Security and Bug Advisory Metadata in RPM Repositories

Updateinfo is the metadata format RPM-based distributions use to describe
security advisories, bug fixes, and feature enhancements that affect packages
in a repository. The metadata lives in a file called `updateinfo.xml`
(referenced as the `updateinfo` record type in `repomd.xml`) and contains a
list of advisory records (also called errata), each linking one or more
external references (CVEs, Bugzilla tickets) to one or more affected packages.

This document is cross-referenced against createrepo_c (the canonical
repository creation tool), libsolv (the solver used by DNF), and DNF5's
documented behavior.

## Purpose

Updateinfo metadata serves three core functions:

1. **Security visibility**: users can query which advisories affect their
   installed packages and whether updates are available that resolve them.
2. **Targeted upgrades**: `dnf5 upgrade --advisory=RHSA-2024:1234` or
   `dnf5 upgrade --security` upgrades only the packages referenced by matching
   advisories, rather than upgrading everything.
3. **Compliance reporting**: tools can enumerate which CVEs are remediated by
   which updates, enabling automated vulnerability scanning and audit trails.

## File location

The updateinfo data is stored as a compressed XML file in the repository's
`repodata/` directory. Its location is declared in `repomd.xml` under a
`<data type="updateinfo">` record:

```xml
<data type="updateinfo">
  <location href="repodata/abc123-updateinfo.xml.gz"/>
  <checksum type="sha256">abc123...</checksum>
  ...
</data>
```

Unlike primary.xml, filelists.xml, and other.xml which declare XML namespaces,
updateinfo.xml has no namespace declaration. The root element is simply
`<updates>`.

Loading updateinfo is optional. DNF5's `optional_metadata_types` configuration
option (which defaults to `{comps, updateinfo}`) controls whether this metadata
is downloaded. Advisory CLI commands force it on regardless of configuration.

## XML structure overview

```xml
<?xml version="1.0" encoding="UTF-8"?>
<updates>
  <update from="security@redhat.com" status="final" type="security" version="1">
    <id>RHSA-2024:1234</id>
    <title>Important: kernel security update</title>
    <issued date="2024-03-15 00:00:00"/>
    <updated date="2024-03-16 12:00:00"/>
    <rights>Copyright 2024 Red Hat, Inc.</rights>
    <release>Red Hat Enterprise Linux 9</release>
    <pushcount>2</pushcount>
    <severity>Important</severity>
    <summary>An update for kernel is now available.</summary>
    <description>The kernel packages contain the Linux kernel...

Security Fix(es):
* kernel: use-after-free in net/sched (CVE-2024-0001)</description>
    <solution>Before applying this update, make sure all previously released
errata relevant to your system have been applied.</solution>
    <message>A reboot is required to apply this update.</message>
    <reboot_suggested>True</reboot_suggested>
    <references>
      <reference href="https://access.redhat.com/errata/RHSA-2024:1234"
                 id="RHSA-2024:1234" type="self"
                 title="RHSA-2024:1234"/>
      <reference href="https://bugzilla.redhat.com/show_bug.cgi?id=2261234"
                 id="2261234" type="bugzilla"
                 title="kernel: use-after-free in net/sched"/>
      <reference href="https://access.redhat.com/security/cve/CVE-2024-0001"
                 id="CVE-2024-0001" type="cve"
                 title="CVE-2024-0001"/>
    </references>
    <pkglist>
      <collection short="rhel-9">
        <name>Red Hat Enterprise Linux 9</name>
        <package name="kernel" version="5.14.0" release="362.24.1.el9_3"
                 epoch="0" arch="x86_64"
                 src="kernel-5.14.0-362.24.1.el9_3.src.rpm">
          <filename>kernel-5.14.0-362.24.1.el9_3.x86_64.rpm</filename>
          <sum type="sha256">8e214681...</sum>
          <reboot_suggested>True</reboot_suggested>
        </package>
      </collection>
    </pkglist>
  </update>
</updates>
```

## Update records

Each `<update>` element represents a single advisory. It carries four
attributes and a number of child elements.

### Update attributes

| Attribute | Description |
|-----------|-------------|
| `from` | The issuing authority (typically an email address like `security@redhat.com` or `updates@fedoraproject.org`). Has been seen missing in the wild (per libzypp). |
| `type` | Advisory type. Red Hat/Fedora values: `security`, `bugfix`, `enhancement`, `newpackage`. SUSE values: `security`, `recommended`, `optional`, `feature`. |
| `status` | Release status: `final`, `stable`, `testing`, `retracted`, or `pending`. |
| `version` | Version of the advisory metadata itself (an integer string, incremented when the advisory is revised). |

### Update child elements

Elements are listed in the order createrepo_c generates them. All text-content
elements are optional unless noted.

| Element | Required | Description |
|---------|----------|-------------|
| `<id>` | Yes | Unique advisory identifier (e.g. `RHSA-2024:1234`, `FEDORA-2024-abc123`). |
| `<title>` | Yes | Human-readable title. |
| `<issued date="..."/>` | No | Self-closing element with a `date` attribute. Date is typically `"YYYY-MM-DD HH:MM:SS"` but may also be a Unix epoch timestamp or contain trailing ` UTC`. Some SUSE repositories use the timestamp as element text content instead of the `date` attribute (see date format notes below). |
| `<updated date="..."/>` | No | Same format as `<issued>`. |
| `<rights>` | No | Copyright string. |
| `<release>` | No | Target distribution release name. |
| `<pushcount>` | No | Number of times the advisory has been pushed. Largely deprecated. |
| `<severity>` | No | Severity level for security advisories: `Critical`, `Important`, `Moderate`, `Low`, or `None`. Free-form string, not validated by parsers. |
| `<summary>` | No | Brief summary. |
| `<description>` | No | Detailed description. Often multi-line, may include CVE identifiers and fix descriptions. |
| `<solution>` | No | Recommended remediation steps. |
| `<message>` | No | Informative message (e.g. reboot instructions). See implementation notes below. |
| `<reboot_suggested>` | No | Record-level reboot flag. See implementation notes below. |
| `<references>` | No | Container for `<reference>` elements. |
| `<pkglist>` | No | Container for `<collection>` elements. |

### Date format

The `date` attribute on `<issued>` and `<updated>` is a free-form string. In
practice, three formats appear in the wild:

| Format | Example |
|--------|---------|
| Date-time string | `"2024-03-15 00:00:00"` |
| Date-time with timezone | `"2024-03-15 06:00:01 UTC"` |
| Unix epoch timestamp | `"1710460800"` |

Parsers should handle all three. createrepo_c stores the raw string and
defers parsing to consumers. libsolv's `datestr2timestamp()` parses both
numeric timestamps and `"YYYY-MM-DD HH:MM:SS"` formatted strings.

Additionally, the date has been seen in the wild as element text content
rather than in a `date` attribute:

```xml
<!-- Standard (Red Hat, Fedora, CentOS) -->
<issued date="2024-03-15 00:00:00"/>

<!-- Variant seen in the wild (per libzypp) -->
<issued>1710460800</issued>
```

libzypp's parser handles both forms. Parsers aiming for broad compatibility
should check for text content when the `date` attribute is absent.

## References

Each `<reference>` within `<references>` is a self-closing element with four
attributes linking the advisory to external tracking systems.

```xml
<reference href="https://bugzilla.redhat.com/show_bug.cgi?id=2261234"
           id="2261234" type="bugzilla"
           title="kernel: use-after-free in net/sched"/>
```

| Attribute | Description |
|-----------|-------------|
| `href` | URL to the external resource. |
| `id` | Identifier within that system (bug number, CVE ID, errata ID). May be absent for `type="self"` references. |
| `type` | Reference type. Common values: `bugzilla`, `cve`, `self` (link back to the errata page), `vendor`. |
| `title` | Human-readable title or description of the reference. |

A single advisory typically has one `type="self"` reference pointing to the
errata page, plus one `type="bugzilla"` and/or `type="cve"` reference per
tracked issue.

## Package lists and collections

The `<pkglist>` element contains one or more `<collection>` elements. Each
collection groups packages that belong together — typically by distribution
release or channel.

### Collections

```xml
<collection short="rhel-9-server">
  <name>Red Hat Enterprise Linux 9 Server</name>
  <!-- optional module element -->
  <package ...>...</package>
  <package ...>...</package>
</collection>
```

| Field | Description |
|-------|-------------|
| `short` attribute | Short identifier for the collection (e.g. `rhel-9-server`, `F35`). |
| `<name>` | Full human-readable name. |

A single advisory may have multiple collections — for example, one per
architecture or one per distribution variant. In modular content, each module
stream gets its own collection.

### Packages

Each `<package>` within a collection describes a specific RPM affected by the
advisory.

```xml
<package name="kernel" version="5.14.0" release="362.24.1.el9_3"
         epoch="0" arch="x86_64"
         src="kernel-5.14.0-362.24.1.el9_3.src.rpm">
  <filename>kernel-5.14.0-362.24.1.el9_3.x86_64.rpm</filename>
  <sum type="sha256">8e214681104e4ba73726e0ce11d21b963ec0390fd70458d439ddc72372082034</sum>
  <reboot_suggested/>
  <restart_suggested/>
  <relogin_suggested/>
</package>
```

#### Package attributes

| Attribute | Description |
|-----------|-------------|
| `name` | Package name. |
| `epoch` | Package epoch (typically `"0"`). |
| `version` | Package version string. |
| `release` | Package release string. |
| `arch` | Package architecture (e.g. `x86_64`, `noarch`, `aarch64`). |
| `src` | Source RPM filename or URL. Optional. |

#### Package child elements

| Element | Description |
|---------|-------------|
| `<filename>` | RPM filename (e.g. `kernel-5.14.0-362.24.1.el9_3.x86_64.rpm`). |
| `<sum type="...">` | Checksum of the RPM file. The `type` attribute specifies the algorithm (`sha256`, `sha1`, etc.) and the text content is the hex digest. Optional. |
| `<reboot_suggested>` | Suggests a system reboot after installing this package. |
| `<restart_suggested>` | Suggests restarting affected services after installing this package. |
| `<relogin_suggested>` | Suggests a user session re-login after installing this package. |

#### Suggested-action flags

The three `*_suggested` elements are boolean flags. Their presence or text
content indicates `true`. Implementations vary on what constitutes a true
value:

| Implementation | True if... |
|---------------|------------|
| createrepo_c (parser) | Element is present (content ignored) |
| createrepo_c (writer) | Writes `True` as content |
| libsolv | Content starts with `T`, `t`, or `1` |
| rpmrepo_metadata | Content is `"1"` or `"True"` (exact match) |

These flags exist at two levels:

- **Package-level** (inside `<package>`): applies to a specific RPM. All
  implementations support this.
- **Record-level** (inside `<update>`): applies to the advisory as a whole.
  createrepo_c and libsolv support this. libsolv stores the record-level flag
  on the advisory solvable and includes a FIXME noting that the global flag
  should ideally be computed at runtime from the per-package flags.

### Modules

Collections may contain an optional `<module>` element for modular content
(Fedora Modularity / RHEL modules). This identifies the module stream that
the packages belong to.

```xml
<module name="perl-DBI" stream="1.641" version="8010020190322130042"
        context="16b3ab4d" arch="x86_64"/>
```

| Attribute | Description |
|-----------|-------------|
| `name` | Module name. |
| `stream` | Module stream. |
| `version` | Module version (unsigned 64-bit integer). |
| `context` | Module context hash. |
| `arch` | Module architecture. |

Module information is used by DNF5 to match advisory packages against
installed module streams. The `Advisory::is_applicable()` method in libdnf5
checks whether an advisory's module matches any active module stream.

## How libsolv represents advisories

libsolv creates a solvable for each advisory, using the `patch:` name prefix.
The advisory `RHSA-2024:1234` becomes a solvable named
`patch:RHSA-2024:1234`.

### Field mapping

| XML source | libsolv keyname | Notes |
|-----------|----------------|-------|
| `<id>` | `solvable->name` | Prefixed with `patch:` |
| `<update version="...">` | `solvable->evr` | |
| `<update from="...">` | `solvable->vendor` | |
| (hardcoded) | `solvable->arch` = `ARCH_NOARCH` | |
| `<update type="...">` | `SOLVABLE_PATCHCATEGORY` | |
| `<update status="...">` | `UPDATE_STATUS` | |
| `<title>` | `SOLVABLE_SUMMARY` | Trailing newlines stripped |
| `<description>` | `SOLVABLE_DESCRIPTION` | |
| `<severity>` | `UPDATE_SEVERITY` | |
| `<rights>` | `UPDATE_RIGHTS` | |
| `<message>` | `UPDATE_MESSAGE` | |
| max(`<issued>`, `<updated>`) | `SOLVABLE_BUILDTIME` | Later of the two dates |
| `<reboot_suggested>` | `UPDATE_REBOOT` | Void flag |
| `<restart_suggested>` | `UPDATE_RESTART` | Void flag |
| `<relogin_suggested>` | `UPDATE_RELOGIN` | Void flag |

### Elements libsolv discards

| Element | Notes |
|---------|-------|
| `<release>` | Parsed but not stored. |
| `<collection><name>` | Parsed but not stored. |
| `<collection short="...">` | Attribute not read. |
| `<package src="...">` | Attribute not read. |

libsolv does not parse `<summary>`, `<solution>`, or `<pushcount>` at all —
these elements are silently skipped.

### Reference and package storage

References and packages are stored as flexarray entries:

- `UPDATE_REFERENCE` contains `UPDATE_REFERENCE_HREF`, `UPDATE_REFERENCE_ID`,
  `UPDATE_REFERENCE_TYPE`, and `UPDATE_REFERENCE_TITLE`.
- `UPDATE_COLLECTION` (acknowledged as misnamed — should be
  `UPDATE_PACKAGE`) contains `UPDATE_COLLECTION_NAME`,
  `UPDATE_COLLECTION_EVR`, `UPDATE_COLLECTION_ARCH`, and
  `UPDATE_COLLECTION_FILENAME`.
- `UPDATE_MODULE` contains `UPDATE_MODULE_NAME`, `UPDATE_MODULE_STREAM`,
  `UPDATE_MODULE_VERSION`, `UPDATE_MODULE_CONTEXT`, and
  `UPDATE_MODULE_ARCH`.
- `UPDATE_COLLECTIONLIST` groups `UPDATE_COLLECTION` and `UPDATE_MODULE`
  entries by their parent `<collection>` element.

### Conflict generation

For each package in an advisory, libsolv generates a conflict of the form
`name < evr` on the advisory solvable. If the package architecture is not
`noarch`, two conflicts are generated: one with the specific arch and one
with `ARCH_NOARCH`. This is the mechanism that lets the solver know which
installed packages are superseded by the advisory.

### Retracted advisories (SUSE)

libsolv includes SUSE-specific logic (`repo_mark_retracted_packages()`) that
finds advisories with `UPDATE_STATUS == "retracted"` and marks the
corresponding binary packages by adding a retracted-package marker to their
provides. This is a post-processing step, not part of XML parsing.

## Advisory types

| Type | Description | DNF5 CLI filter |
|------|-------------|----------------|
| `security` | Security vulnerability fix. Usually carries a severity. | `--security` |
| `bugfix` | Bug fix. | `--bugfix` |
| `enhancement` | New feature or improvement. | `--enhancement` |
| `newpackage` | New package added to the distribution. | `--newpackage` |

The type is stored as a free-form string in the `type` attribute of
`<update>`. These four values are conventional but not enforced by parsers.

## Advisory severities

Severity applies primarily to `security`-type advisories:

| Severity | Description |
|----------|-------------|
| `Critical` | Remote code execution, privilege escalation, or similar with widespread impact. |
| `Important` | Significant security impact — denial of service, information disclosure, privilege escalation with some mitigating factors. |
| `Moderate` | More limited impact or harder to exploit. |
| `Low` | Minimal security impact. |
| `None` | No severity assigned (used for non-security advisories that happen to carry a severity field). |

DNF5's `--advisory-severities` filter accepts these values case-insensitively.
The `advisory summary` command groups security advisories into
Critical/Important/Moderate/Low/Other buckets, with "Other" catching
non-standard severity strings.

## DNF5 CLI commands

The `advisory` command (aliased as `updateinfo` for compatibility) has three
subcommands:

| Command | What it does |
|---------|-------------|
| `dnf5 advisory list` | Tabular list: advisory ID, type, severity, package NEVRA, date. |
| `dnf5 advisory info <spec>` | Detailed advisory info: title, severity, type, status, vendor, dates, description, message, rights, references, and package collections. |
| `dnf5 advisory summary` | Counts by type and severity. |

### Availability modes

| Flag | What it shows |
|------|---------------|
| `--available` (default) | Advisories with packages newer than the installed latest EVR. |
| `--installed` | Advisories with packages at or below installed versions. |
| `--updates` | Like `--available`, but further limited to upgradable packages. |
| `--all` | Both available and installed. |

### Filtering options

| Option | Description |
|--------|-------------|
| `--contains-pkgs=NAME,...` | Filter by package name (glob supported). |
| `--security` / `--bugfix` / `--enhancement` / `--newpackage` | Filter by type. |
| `--advisory-severities=SEV,...` | Filter by severity. |
| `--bzs=ID,...` | Filter by Bugzilla IDs. |
| `--cves=ID,...` | Filter by CVE IDs. |
| `--with-bz` / `--with-cve` | Only show advisories that have bugzilla/CVE references. |

### Advisory-scoped upgrades

Advisory filters are also available on `upgrade`, `install`, `repoquery`, and
`check-upgrade`. This enables targeted upgrades:

```bash
dnf5 upgrade --security                    # only security fixes
dnf5 upgrade --advisory=RHSA-2024:1234     # specific advisory
dnf5 upgrade --security --minimal          # lowest version that fixes
dnf5 upgrade --cves=CVE-2024-0001          # specific CVE
```

The `--minimal` flag limits upgrades to the lowest package version that
resolves the matching advisories, rather than upgrading to the absolute
latest.

### Running kernel handling

DNF5 adds the running kernel and its source package siblings to the installed
packages set when evaluating advisories. This ensures advisories for the
running kernel are shown even if a newer kernel RPM is already installed but
not yet booted.

## Real-world example

A minimal real-world advisory from Fedora:

```xml
<update from="updates@fedoraproject.org" status="stable"
        type="bugfix" version="2">
  <id>FEDORA-2024-abc123def4</id>
  <title>nano-7.2-3.fc40</title>
  <issued date="2024-01-15 04:10:31"/>
  <updated date="2024-01-16 01:23:45"/>
  <severity>None</severity>
  <description>Update to nano 7.2 with bug fixes.</description>
  <references>
    <reference href="https://bugzilla.redhat.com/show_bug.cgi?id=2261234"
               id="2261234" type="bugzilla"
               title="nano-7.2 is available"/>
  </references>
  <pkglist>
    <collection short="F40">
      <name>Fedora 40</name>
      <package name="nano" version="7.2" release="3.fc40"
               epoch="0" arch="x86_64"
               src="nano-7.2-3.fc40.src.rpm">
        <filename>nano-7.2-3.fc40.x86_64.rpm</filename>
        <sum type="sha256">29be985e1f652cd0a29ceed6a1c49964d3618bddd22f0be3292421c8777d26c8</sum>
      </package>
    </collection>
  </pkglist>
</update>
```

## Cross-implementation comparison

### Elements supported by each implementation

| Element | createrepo_c | libsolv | rpmrepo_metadata |
|---------|:----------:|:------:|:---------------:|
| `<id>` | Yes | Yes | Yes |
| `<title>` | Yes | Yes | Yes |
| `<issued>` | Yes | Yes | Yes |
| `<updated>` | Yes | Yes | Yes |
| `<rights>` | Yes | Yes | Yes |
| `<release>` | Yes | Parsed, discarded | Yes |
| `<pushcount>` | Yes | No | Yes |
| `<severity>` | Yes | Yes | Yes |
| `<summary>` | Yes | No | Yes |
| `<description>` | Yes | Yes | Yes |
| `<solution>` | Yes | No | Yes |
| `<message>` | Parsed, discarded | Yes | Yes |
| Record-level `<reboot_suggested>` | Yes | Yes | Yes |
| `<references>` | Yes | Yes | Yes |
| `<pkglist>` | Yes | Yes | Yes |
| `<collection short="...">` | Yes | Ignored | Yes |
| `<collection><name>` | Yes | Parsed, discarded | Yes |
| `<module>` (NSVCA) | Yes | Yes | Yes |
| `<package>` (NEVRA) | Yes | Yes | Yes |
| `<package src="...">` | Yes | Ignored | Yes |
| `<filename>` | Yes | Yes | Yes |
| `<sum>` (checksum) | Yes | No | Yes |
| Package `<reboot_suggested>` | Yes | Yes | Yes |
| Package `<restart_suggested>` | Yes | Yes | Yes |
| Package `<relogin_suggested>` | Yes | Yes | Yes |

### Strictness comparison

| Behavior | createrepo_c | libsolv | rpmrepo_metadata |
|----------|:----------:|:------:|:---------------:|
| `<update>` attributes required | No (NULL if absent) | No | **Yes** (error) |
| `<reference>` attributes required | No (NULL if absent) | No | **Partially** (`href`, `type`, `title` required; `id` optional) |
| `<package>` attributes required | No (NULL if absent) | No | **Yes** (error) |
| Unknown elements | Silently skipped | Silently skipped | Silently skipped |

createrepo_c's test suite includes a fixture (`updateinfo_02.xml`) with an
`<update>` element that has no attributes at all, an attribute-less
`<reference/>`, and an attribute-less `<package>`. rpmrepo_metadata would
return an error on such input, while createrepo_c and libsolv accept it
gracefully. This is unlikely to be a problem in practice since real-world
advisory metadata from Fedora, RHEL, and other distributions always includes
these attributes.

## Hierarchy summary

```
updateinfo.xml
└── <updates>
    └── <update>* ──── from, type, status, version
        ├── <id>
        ├── <title>
        ├── <issued date="..."/>
        ├── <updated date="..."/>
        ├── <rights>
        ├── <release>
        ├── <pushcount>
        ├── <severity>
        ├── <summary>
        ├── <description>
        ├── <solution>
        ├── <message>
        ├── <reboot_suggested>
        ├── <references>
        │   └── <reference>* ── href, id, type, title
        └── <pkglist>
            └── <collection>* ── short
                ├── <name>
                ├── <module/>? ── name, stream, version, context, arch
                └── <package>* ── name, epoch, version, release, arch, src
                    ├── <filename>
                    ├── <sum type="...">
                    ├── <reboot_suggested>
                    ├── <restart_suggested>
                    └── <relogin_suggested>
```

## Implementation notes for rpmrepo_metadata

### What rpmrepo_metadata supports that libsolv does not

rpmrepo_metadata preserves several fields that libsolv discards, making it
suitable for advisory round-tripping (read, modify, write back):

- `<release>`, `<pushcount>`, `<summary>`, `<solution>` — all stored.
- `<collection short="...">` and `<collection><name>` — preserved.
- `<package src="...">` — preserved.
- `<sum>` (package checksum) — preserved.

## Updateinfo across repositories

A single repository contains at most one `updateinfo.xml` file. When a system
is configured with multiple repositories, advisories from all repos are loaded
into the same pool. Advisory IDs are globally unique — the same advisory ID
appearing in multiple repos (e.g. mirrored repos) represents the same
advisory.

## Updateinfo in Fedora vs RHEL

Both Fedora and RHEL/CentOS use the same updateinfo.xml format. The primary
differences are in content and process:

- **RHEL** advisories are formally structured errata (RHSA for security,
  RHBA for bug fixes, RHEA for enhancements) with consistent severity
  ratings, CVE cross-references, and solution text.
- **Fedora** advisories are generated from Bodhi (Fedora's update system)
  and tend to be simpler — often just a title matching the package NEVRA,
  with bugzilla references but less detailed description text.
- **CentOS Stream / AlmaLinux / Rocky Linux** import or mirror RHEL-derived
  advisories with their own ID namespaces.

## SUSE-specific extensions

SUSE uses the `<update status="retracted">` status value to indicate
advisories that have been withdrawn (e.g. because the update caused
regressions). libsolv includes SUSE-specific post-processing
(`repo_mark_retracted_packages()`) that marks binary packages referenced by
retracted advisories, allowing the solver to avoid installing them.

## Implementation-specific quirks and behavior in the wild

This section documents behaviors observed across implementations and
real-world metadata that may surprise implementors.

### No XML namespace

Unlike primary.xml (`http://linux.duke.edu/metadata/common`), filelists.xml,
and other.xml, updateinfo.xml has no namespace declaration. The root element
is simply `<updates>` with no `xmlns` attribute. This has been stable since
the format's inception.

### Date format variations

The `date` attribute on `<issued>` and `<updated>` is a free-form string
with no enforced format. Real-world metadata contains at least three
formats:

| Format | Example | Sources |
|--------|---------|---------|
| Date-time | `"2024-03-15 00:00:00"` | Fedora, RHEL |
| Date-time with timezone | `"2024-03-15 06:00:01 UTC"` | RHEL |
| Unix epoch timestamp | `"1710460800"` | Various |

createrepo_c stores date values as raw strings in C and does no parsing.
Its Python bindings attempt `strptime` with `"%Y-%m-%d %H:%M:%S"`, then
`"%Y-%m-%d"`, then treat the value as an epoch integer, raising an error if
none work. createrepo_c's test suite includes deliberately malformed dates
like `"15mangled2"` to test tolerance.

libsolv's `datestr2timestamp()` handles numeric timestamps and
`"YYYY-MM-DD HH:MM:SS"` format strings. It stores the later of the issued
and updated dates as `SOLVABLE_BUILDTIME`.

### Boolean format for `*_suggested` flags

All real-world metadata examined (RHEL 6/8/9, AlmaLinux 8, Fedora 42,
EPEL 9, Oracle Linux 9, VMware Photon) uses the exact string `True`
(capital T, Python-style) as the text content for `<reboot_suggested>`,
`<restart_suggested>`, and `<relogin_suggested>`. No instances of `true`,
`1`, `False`, `0`, or empty self-closing elements were found in production
metadata.

Despite this uniformity, implementations disagree on parsing:

| Implementation | True if... |
|---------------|------------|
| createrepo_c | Element is present (content ignored entirely) |
| libsolv | First character is `T`, `t`, or `1` |
| rpmrepo_metadata | First character is `T`, `t`, or `1` (matches libsolv) |

createrepo_c's presence-based parsing means `<reboot_suggested>False</reboot_suggested>`
would be interpreted as true — arguably a bug. Its test fixture
`updateinfo_01.xml` includes `<reboot_suggested/>` (empty self-closing),
which createrepo_c treats as true but libsolv would treat as false.

### Record-level vs package-level `<reboot_suggested>`

The `<reboot_suggested>` element can appear at two levels:

1. **Record-level**: as a direct child of `<update>`, indicating the
   advisory as a whole suggests a reboot.
2. **Package-level**: inside `<package>`, indicating a specific RPM
   suggests a reboot.

RHEL 8/9 and AlmaLinux 8 use record-level `<reboot_suggested>True</reboot_suggested>`
extensively — typically for kernel, systemd, and glibc updates. These
advisories usually also set the flag at the package level. Fedora and EPEL
only use the package-level flag.

Bugzilla [#1772466](https://bugzilla.redhat.com/show_bug.cgi?id=1772466)
("Createrepo_c UpdateRecord ignores reboot_suggested at advisory level")
documents that createrepo_c historically had issues with this field.

libsolv stores the record-level flag with a FIXME comment: "this is
per-package, the global flag should be computed at runtime."

### `<message>` element

The `<message>` element is defined in the format and stored by libsolv
(`UPDATE_MESSAGE`). DNF5 displays it in `advisory info` output. However:

- createrepo_c recognizes it during parsing (avoiding unknown-element
  warnings) but silently discards the content. There is no field in the
  `cr_UpdateRecord` struct, no XML dump code, and no Python binding. The
  parser state is marked `// NI` (not implemented).
- No real-world updateinfo.xml files examined (across RHEL, Fedora,
  AlmaLinux, Oracle Linux, EPEL, VMware Photon) contain a `<message>`
  element.

Despite its absence in the wild, rpmrepo_metadata parses and preserves it
for completeness.

### `<restart_suggested>` rarity

While `<reboot_suggested>` and `<relogin_suggested>` appear in real-world
metadata, `<restart_suggested>` was not found in any of the examined
fixtures (RHEL 6/8/9, AlmaLinux 8, Fedora 42, EPEL 9, Oracle Linux 9,
VMware Photon). The element is defined by all implementations and appears
in createrepo_c's synthetic test fixtures, but appears to be unused in
production metadata from major distributions.

### `<update>` with missing attributes

createrepo_c's test fixture `updateinfo_02.xml` includes an `<update>`
element with no attributes at all, a `<reference/>` with no attributes,
and a `<package>` with no attributes. This tests that createrepo_c
handles NULL values gracefully. libsolv also accepts this. rpmrepo_metadata
requires the `from`, `type`, `status`, and `version` attributes on
`<update>` and returns an error if any are missing. This matches the
behavior of all real-world metadata producers.

### Ampersand double-encoding

createrepo_c includes a workaround (`unescape_ampersand_from_values()`)
for `&#38;` appearing in XML attribute values, which can result from
double-encoding of `&` characters. The parser strips `#38;` sequences
from ampersands. A dedicated test fixture (`updateinfo_ampersand.xml`)
exercises this. This is not a property of the format itself but an artifact
of how some metadata producers encode special characters.

### libsolv internal naming

libsolv uses `UPDATE_COLLECTION` as the keyname for individual package
entries within an advisory. The source code contains an explicit comment:
"UPDATE_COLLECTION is misnamed, it should have been UPDATE_PACKAGE." Each
flexarray entry under `UPDATE_COLLECTION` represents a single package,
not a collection. The actual collection grouping is handled separately by
`UPDATE_COLLECTIONLIST`.

### libsolv title newline stripping

libsolv strips trailing newlines from `<title>` text content before
storing it as `SOLVABLE_SUMMARY`. This stripping is not applied to
`<description>` or other text elements.

### createrepo_c API typo

The public C function for adding advisory records to an updateinfo
object is named `cr_updateinfo_apped_record` (note: "apped" instead of
"append"). This typo is preserved in the public API for backwards
compatibility.
