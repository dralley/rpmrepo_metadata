# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Changed

- `UpdateReference` fields (`href`, `title`, `reftype`) now preserve empty string values from XML instead of treating them as required.
  - Real-world metadata like RHEL6 and AmaLinux 8, can contain e.g. thousands of `type="other"` documentation references with empty `id=""` and `title=""` attributes.
  - When writing: empty string attributes are omitted from the XML output
- Updateinfo XML writer now matches createrepo_c output order
- Package-level suggested flags (`reboot_suggested`, `restart_suggested`, `relogin_suggested`) now write `"True"` instead of `"1"` to match createrepo_c

## 0.6.1

- Add some additional functions to the Python bindings necessary for authoring of comps metadata.

## 0.6.0

### Added

- Parsing and writing support for `supportinfo.xml` (DNF plugin package support information), via `SupportInfoXmlReader`, `SupportInfoXmlWriter`, and the `SupportInfoVisitor` trait. Support is gated by the "supportinfo" feature flag, which is disabled by default.
  - NOTE: this is unofficial, experimental and intended for further experimentation, not something which ought to be used for any serious purpose.

### Fixed

- A number of inconsistencies related to comps and updateinfo parsing.

## 0.5.0

### Added

- A visitor API is now provided to allow parsing without allocations, useful for e.g. loading a dependency resolver which uses its own string internment.

### Changed

- `Package` now stores file lists in a `FileList` struct which performs interning of base paths, dramatically reducing memory requirements for large repos. The internal string pool is shared when parsing an entire repository or consecutive packages of the same name. File name itself is now stored in a `compact_str` to prevent extreme numbers of small String allocations.
- Various other optimizations.
- Fields on `Requirement` are now private and use getters/setters.

### Fixed

- Lots of compatibility nitpicks found by parsing in-the-wild repos.
