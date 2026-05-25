# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## 0.5.0

### Added

- A visitor API is now provided to allow parsing without allocations, useful for e.g. loading a dependency resolver which uses its own string internment.

### Changed

- `Package` now stores file lists in a `FileList` struct which performs interning of base paths, dramatically reducing memory requirements for large repos. The internal string pool is shared when parsing an entire repository or consecutive packages of the same name. File name itself is now stored in a `compact_str` to prevent extreme numbers of small String allocations.
