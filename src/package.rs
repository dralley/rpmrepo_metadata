// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::io::BufReader;
use std::path::Path;

use crate::constants::mdrecord;
use crate::filelist::FilelistsXmlReader;
use crate::other::OtherXmlReader;
use crate::primary::PrimaryXmlReader;
use crate::{Checksum, ChecksumType, FilelistsXml, MetadataError, OtherXml, Package, PrimaryXml};
use crate::{RepomdData, utils};

/// Options for parsing an RPM package into a [`Package`].
pub struct PackageOptions {
    /// Checksum algorithm used to hash the RPM file. Default: SHA-256.
    pub checksum_type: ChecksumType,
    /// Maximum number of changelog entries to keep (most recent first). Default: 10.
    pub changelog_limit: usize,
}

/// File-specific options for reading an RPM package into a [`Package`].
#[derive(Default)]
pub struct PackageFileOptions {
    /// Options shared by all RPM package parsing APIs.
    pub package_options: PackageOptions,
    /// Override for the `location_href` field. If `None`, it defaults to the RPM filename.
    pub location_href: Option<String>,
    /// Optional base URL prepended to `location_href` when resolving the package location.
    pub location_base: Option<String>,
}

/// File-derived data needed to create repository metadata from RPM headers.
///
/// RPM headers do not contain the full-file checksum, on-disk size, modification time, or
/// repository location. Callers that parse headers independently provide those values here.
/// [`Package::from_package_metadata`] requires `checksum` and `size_package` to be present.
pub struct PackageSource {
    /// Checksum of the complete RPM file.
    ///
    /// [`Package::from_buffer`] derives this using [`PackageOptions::checksum_type`] when it and
    /// `size_package` are both `None`.
    pub checksum: Option<Checksum>,
    /// Size of the complete RPM file in bytes.
    ///
    /// [`Package::from_buffer`] derives this from the buffer length when it and
    /// `checksum` are both `None`.
    pub size_package: Option<u64>,
    /// Modification time of the RPM file as seconds since the Unix epoch.
    pub time_file: u64,
    /// Package location relative to the repository root.
    pub location_href: String,
    /// Optional base URL for the package location.
    pub location_base: Option<String>,
}

#[cfg(feature = "read_rpm")]
impl PackageSource {
    /// Reports whether both full-file values are present and rejects a partial pair.
    fn has_file_values(&self) -> Result<bool, MetadataError> {
        match (&self.checksum, self.size_package) {
            (Some(_), Some(_)) => Ok(true),
            (None, None) => Ok(false),
            _ => Err(MetadataError::InconsistentMetadataError(
                "checksum and size_package must be supplied together".to_owned(),
            )),
        }
    }
}

impl Default for PackageOptions {
    fn default() -> Self {
        Self {
            checksum_type: ChecksumType::Sha256,
            changelog_limit: 10,
        }
    }
}

#[cfg(feature = "read_rpm")]
pub mod rpm_parsing {
    use std::time::SystemTime;
    use std::{cmp::Ordering, collections::HashSet, fs::File};

    use indexmap::IndexSet;

    use crate::{Changelog, Evr, Requirement, RequirementType};

    use super::*;
    use rpm;

    /// Raw dependency fields used to recognize requirements satisfied by the package itself.
    ///
    /// Keeping the original version avoids treating distinct EVRs as equal after normalization.
    #[derive(Hash, PartialEq, Eq)]
    struct DependencyKey {
        name: String,
        flags: Option<RequirementType>,
        version: String,
    }

    /// Converts RPM's low-nibble comparator into the repository XML representation.
    ///
    /// Non-comparison bits must not affect the XML comparator.
    fn dependency_flags(flags: rpm::DependencyFlags) -> Option<RequirementType> {
        let comparison = flags
            & (rpm::DependencyFlags::LESS
                | rpm::DependencyFlags::GREATER
                | rpm::DependencyFlags::EQUAL);

        if comparison == rpm::DependencyFlags::LESS {
            Some(RequirementType::LT)
        } else if comparison == rpm::DependencyFlags::GREATER {
            Some(RequirementType::GT)
        } else if comparison == rpm::DependencyFlags::EQUAL {
            Some(RequirementType::EQ)
        } else if comparison == (rpm::DependencyFlags::LESS | rpm::DependencyFlags::EQUAL) {
            Some(RequirementType::LE)
        } else if comparison == (rpm::DependencyFlags::GREATER | rpm::DependencyFlags::EQUAL) {
            Some(RequirementType::GE)
        } else {
            None
        }
    }

    /// Builds the raw dependency key used for self-provided requirement filtering.
    fn dependency_key(dependency: &rpm::Dependency) -> DependencyKey {
        DependencyKey {
            name: dependency.name.clone(),
            flags: dependency_flags(dependency.flags),
            version: dependency.version.clone(),
        }
    }

    /// Reports whether RPM marks a requirement as needed before installation.
    ///
    /// Repository metadata serializes these scriptlet phases with the `pre` attribute.
    fn is_preinstall(flags: rpm::DependencyFlags) -> bool {
        flags.intersects(
            rpm::DependencyFlags::PREREQ
                | rpm::DependencyFlags::SCRIPT_PRE
                | rpm::DependencyFlags::SCRIPT_POST
                | rpm::DependencyFlags::PRETRANS
                | rpm::DependencyFlags::POSTTRANS,
        )
    }

    /// Reports whether an explicitly supplied dependency epoch is a nonnegative integer.
    fn has_valid_epoch(version: &str) -> bool {
        version
            .split_once(':')
            .is_none_or(|(epoch, _)| epoch.parse::<u64>().is_ok())
    }

    /// The first capability form used to reduce `libc.so.6` requirements.
    enum LibcCapability<'a> {
        NoParenthesis,
        Unterminated,
        Empty,
        NonNumeric,
        Version(&'a str),
    }

    impl LibcCapability<'_> {
        /// Returns the precedence of a capability form.
        fn rank(&self) -> u8 {
            match self {
                Self::NoParenthesis => 0,
                Self::Unterminated => 1,
                Self::Empty => 2,
                Self::NonNumeric => 3,
                Self::Version(_) => 4,
            }
        }
    }

    /// Classifies the first parenthesized component of a libc capability.
    ///
    /// Accepted forms include bare `libc.so.6`, an empty symbol version such as
    /// `libc.so.6()(64bit)`, a nonnumeric symbol such as `libc.so.6(GLIBC_ABI_DT_RELR)(64bit)`,
    /// and a numeric symbol such as `libc.so.6(GLIBC_2.38)(64bit)`. Version extraction is
    /// limited to the first component; `(64bit)` is architecture information, not a version.
    fn libc_capability(name: &str) -> LibcCapability<'_> {
        let Some((_, rest)) = name.split_once('(') else {
            return LibcCapability::NoParenthesis;
        };
        let Some((version, _)) = rest.split_once(')') else {
            return LibcCapability::Unterminated;
        };
        if version.is_empty() {
            return LibcCapability::Empty;
        }
        match version.find(|character: char| character.is_ascii_digit()) {
            Some(index) => LibcCapability::Version(&version[index..]),
            None => LibcCapability::NonNumeric,
        }
    }

    /// Orders `libc.so.6` capabilities when reducing requirements.
    ///
    /// A binary can require several cumulative GLIBC symbol versions. Publishing the highest
    /// numeric version omits requirements that glibc's provides already imply. This deliberately
    /// applies only to `libc.so.6`; it is not a general shared-library dependency rule.
    fn compare_libc_requirements(first: &str, second: &str) -> Ordering {
        let first = libc_capability(first);
        let second = libc_capability(second);

        match (&first, &second) {
            (LibcCapability::Version(first), LibcCapability::Version(second)) => {
                // Empty epoch and release isolate RPM's version-component comparison.
                Evr::new("", first, "").cmp(&Evr::new("", second, ""))
            }
            _ => first.rank().cmp(&second.rank()),
        }
    }

    /// Filters requirements that repository clients do not need to resolve.
    ///
    /// - skip rpmlib() deps (internal RPM feature tracking)
    /// - skip file-path requires for primary files the package itself contains
    /// - skip deps that the package itself provides (self-satisfied dependencies)
    ///
    /// `MISSINGOK` requirements remain in `requires` - createrepo_c with LEGACY_WEAKDEPS_ENABLED
    /// reinterprets that flag as a weak recommendation;  that is not implemented here.
    fn filter_requires(
        dependencies: Vec<rpm::Dependency>,
        provided: &HashSet<DependencyKey>,
        files: &crate::FileList,
    ) -> Result<Vec<Requirement>, MetadataError> {
        let mut requires = IndexSet::new();
        let mut libc_requirement: Option<(String, Requirement)> = None;

        for dependency in dependencies {
            // RPM feature requirements describe the package format, not an installable package.
            if dependency.name.starts_with("rpmlib(") {
                continue;
            }

            // A package satisfies a primary-path requirement when it installs that path itself.
            if dependency.name.starts_with('/')
                && files.contains(&dependency.name)
                && utils::is_primary_file(&dependency.name)
            {
                continue;
            }

            let dependency_key = dependency_key(&dependency);
            // Self-provided requirements do not constrain a repository transaction.
            if provided.contains(&dependency_key) {
                continue;
            }

            // An explicit epoch must be numeric; otherwise repository clients cannot compare it.
            if !has_valid_epoch(&dependency.version) {
                continue;
            }

            let preinstall = is_preinstall(dependency.flags);
            let requirement = Requirement::try_from(dependency)?.set_preinstall(preinstall);
            // GLIBC symbol versions are cumulative, so retain only the highest libc capability.
            if requirement.name().starts_with("libc.so.6") {
                if libc_requirement.as_ref().is_none_or(|(name, _)| {
                    compare_libc_requirements(name, requirement.name()) == Ordering::Less
                }) {
                    libc_requirement = Some((requirement.name().to_owned(), requirement));
                }
            } else {
                // Removes duplicates through the use of IndexSet.
                requires.insert(requirement);
            }
        }

        let mut requires: Vec<_> = requires.into_iter().collect();
        if let Some((_, requirement)) = libc_requirement {
            // Keep the legacy libc reduction after normal requirements for stable output order.
            requires.push(requirement);
        }
        Ok(requires)
    }

    /// Maps RPM file metadata to the file types supported by repository metadata.
    ///
    /// RPM metadata has no representation for device nodes, FIFOs, or sockets, so those entries
    /// are represented as ordinary files. A directory type takes precedence over a ghost flag.
    fn repository_file_type(file_type: rpm::FileType, flags: rpm::FileFlags) -> crate::FileType {
        if matches!(file_type, rpm::FileType::Dir) {
            crate::FileType::Dir
        } else if flags.contains(rpm::FileFlags::GHOST) {
            crate::FileType::Ghost
        } else {
            crate::FileType::File
        }
    }

    impl TryFrom<rpm::Dependency> for Requirement {
        type Error = MetadataError;

        fn try_from(d: rpm::Dependency) -> Result<Self, Self::Error> {
            let flags = dependency_flags(d.flags);

            let evr = Evr::parse(&d.version);

            let epoch: Option<&str> = if evr.epoch().is_empty() {
                if d.version.is_empty() {
                    None
                } else {
                    Some("0")
                }
            } else {
                Some(evr.epoch())
            };
            let version: Option<&str> = if evr.version().is_empty() && d.version.is_empty() {
                None
            } else {
                Some(evr.version())
            };
            let release: Option<&str> = if evr.release().is_empty() {
                None
            } else {
                Some(evr.release())
            };

            Ok(Requirement::new(d.name)
                .set_flags(flags)
                .set_epoch(epoch)
                .set_version(version)
                .set_release(release))
        }
    }

    impl From<rpm::ChangelogEntry> for Changelog {
        fn from(value: rpm::ChangelogEntry) -> Self {
            Changelog {
                author: value.name,
                timestamp: value.timestamp,
                description: value.description,
            }
        }
    }

    impl Package {
        /// Read an RPM file from disk using default [`PackageFileOptions`].
        pub fn from_file<A: AsRef<Path>>(path: A) -> Result<Package, MetadataError> {
            Self::from_file_with_options(path, PackageFileOptions::default())
        }

        /// Read an RPM file from disk using the provided [`PackageFileOptions`].
        pub fn from_file_with_options<A: AsRef<Path>>(
            path: A,
            options: PackageFileOptions,
        ) -> Result<Package, MetadataError> {
            let file = File::open(&path)?;
            let file_metadata = file.metadata()?;
            let pkg = rpm::PackageMetadata::parse(&mut BufReader::new(&file))?;

            let href = options.location_href.unwrap_or_else(|| {
                path.as_ref()
                    .file_name()
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.as_ref().to_string_lossy().into_owned())
            });
            let source = PackageSource {
                checksum: Some(utils::checksum_file(
                    path.as_ref(),
                    options.package_options.checksum_type,
                )?),
                size_package: Some(file_metadata.len()),
                time_file: file_metadata
                    .modified()?
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                location_href: href,
                location_base: options.location_base,
            };

            Self::from_package_metadata(&pkg, source, options.package_options)
        }

        /// Parse RPM headers from a buffer and create repository package metadata.
        ///
        /// If [`PackageSource::checksum`] and [`PackageSource::size_package`] are supplied, the
        /// buffer needs only the RPM lead, signature header, and main header. If both are absent,
        /// the buffer is assumed to contain the complete RPM and both values are derived from it
        /// using [`PackageOptions::checksum_type`] and its byte length. Supplying only one is an
        /// error. Supplied values are trusted and are not verified against the buffer.
        pub fn from_buffer(
            buffer: impl AsRef<[u8]>,
            mut source: PackageSource,
            options: PackageOptions,
        ) -> Result<Package, MetadataError> {
            let buffer = buffer.as_ref();
            if !source.has_file_values()? {
                source.checksum = Some(utils::checksum_bytes(buffer, options.checksum_type)?);
                source.size_package = Some(buffer.len() as u64);
            }
            let pkg = rpm::PackageMetadata::parse(&mut BufReader::new(buffer))?;
            Self::from_package_metadata(&pkg, source, options)
        }

        /// Create repository package metadata from previously parsed RPM headers.
        ///
        /// This avoids reparsing headers when the caller also needs RPM metadata such as
        /// signatures. [`PackageSource::checksum`] and [`PackageSource::size_package`] must be
        /// present because there is no source buffer from which to derive them.
        pub fn from_package_metadata(
            pkg: &rpm::PackageMetadata,
            source: PackageSource,
            options: PackageOptions,
        ) -> Result<Package, MetadataError> {
            if !source.has_file_values()? {
                return Err(MetadataError::MissingFieldError(
                    "checksum and size_package",
                ));
            }
            let mut pkg_metadata = Package::default();

            pkg_metadata.set_name(pkg.get_name()?);

            let arch = if pkg.is_source_package() {
                "src"
            } else {
                pkg.get_arch()?
            };

            pkg_metadata.set_arch(arch);
            pkg_metadata.set_evr(Evr::new(
                pkg.get_epoch().unwrap_or_default().to_string(),
                pkg.get_version()?.to_owned(),
                pkg.get_release()?.to_owned(),
            ));

            // These tags are optional in the RPM spec and default to empty when absent,
            // matching createrepo_c which always emits the XML element with empty content
            pkg_metadata.set_summary(pkg.get_summary().unwrap_or_default());
            pkg_metadata.set_description(pkg.get_description().unwrap_or_default());
            pkg_metadata.set_packager(pkg.get_packager().unwrap_or_default());
            pkg_metadata.set_url(pkg.get_url().unwrap_or_default());
            pkg_metadata.set_time_build(pkg.get_build_time().unwrap_or_default());
            pkg_metadata.set_rpm_license(pkg.get_license().unwrap_or_default());
            pkg_metadata.set_rpm_vendor(pkg.get_vendor().unwrap_or_default());
            pkg_metadata.set_rpm_group(pkg.get_group().unwrap_or_default());
            pkg_metadata.set_rpm_buildhost(pkg.get_build_host().unwrap_or_default());
            pkg_metadata.set_rpm_sourcerpm(pkg.get_source_rpm().unwrap_or_default());

            let archive_size = pkg
                .signature
                .get_entry_data_as_u64(rpm::IndexSignatureTag::RPMSIGTAG_LONGARCHIVESIZE)
                .unwrap_or_else(|_| {
                    pkg.signature
                        .get_entry_data_as_u32(rpm::IndexSignatureTag::RPMSIGTAG_PAYLOADSIZE)
                        .unwrap_or(0) as u64
                });
            pkg_metadata.set_size_archive(archive_size);
            pkg_metadata.set_size_installed(pkg.get_installed_size()?);

            fn convert_deps(
                requirements: Vec<rpm::Dependency>,
            ) -> Result<Vec<Requirement>, MetadataError> {
                let mut out = Vec::new();
                for r in requirements.into_iter() {
                    out.push(r.try_into()?)
                }
                Ok(out)
            }

            // Build a set of provided deps so we can filter self-provided entries from requires.
            let rpm_provides = pkg.get_provides()?;
            let provided: HashSet<_> = rpm_provides.iter().map(dependency_key).collect();
            let provides = convert_deps(rpm_provides)?;

            // All files are stored; the primary/filelists split happens at write time
            pkg.for_each_file_entry(|f| {
                let filetype = repository_file_type(f.file_type(), f.flags());
                pkg_metadata.add_file_split(filetype, f.dirname(), f.basename());
                Ok(())
            })?;

            // Omit redundant requirements so primary metadata matches createrepo_c output.
            let requires = filter_requires(pkg.get_requires()?, &provided, pkg_metadata.files())?;

            pkg_metadata.set_requires(requires);
            pkg_metadata.set_provides(provides);
            pkg_metadata.set_conflicts(convert_deps(pkg.get_conflicts()?)?);
            pkg_metadata.set_obsoletes(convert_deps(pkg.get_obsoletes()?)?);
            pkg_metadata.set_suggests(convert_deps(pkg.get_suggests()?)?);
            pkg_metadata.set_enhances(convert_deps(pkg.get_enhances()?)?);
            pkg_metadata.set_recommends(convert_deps(pkg.get_recommends()?)?);
            pkg_metadata.set_supplements(convert_deps(pkg.get_supplements()?)?);

            // Keep only the N most recent entries, sorted oldest-first
            let mut changelogs: Vec<Changelog> = Vec::new();
            for f in pkg.iter_changelog_entries()?.take(options.changelog_limit) {
                let mut entry: Changelog = f.into();
                // trim leading/trailing whitespace
                let trimmed_author = entry.author.trim();
                if trimmed_author.len() != entry.author.len() {
                    entry.author = trimmed_author.to_owned();
                }
                changelogs.push(entry);
            }
            changelogs.reverse();
            pkg_metadata.set_changelogs(changelogs);

            pkg_metadata.set_checksum(
                source
                    .checksum
                    .ok_or(MetadataError::MissingFieldError("checksum"))?,
            );
            pkg_metadata.set_location_href(source.location_href);
            if let Some(base) = source.location_base {
                pkg_metadata.set_location_base(Some(base));
            }
            pkg_metadata.set_size_package(
                source
                    .size_package
                    .ok_or(MetadataError::MissingFieldError("size_package"))?,
            );
            pkg_metadata.set_time_file(source.time_file);

            let offsets = pkg.get_package_segment_offsets();
            pkg_metadata.set_rpm_header_range(offsets.header, offsets.payload);

            Ok(pkg_metadata)
        }
    }

    impl crate::Repository {
        /// Read an RPM file from disk and add it to the repository.
        pub fn add_package_from_file<A: AsRef<Path>>(
            &mut self,
            path: A,
        ) -> Result<(), MetadataError> {
            let pkg = Package::from_file(&path)?;
            self.packages_mut().insert(pkg.pkgid().to_owned(), pkg);
            Ok(())
        }
    }

    impl crate::RepositoryWriter {
        /// Read an RPM file from disk and add it to the repository metadata.
        pub fn add_package_from_file<A: AsRef<Path>>(
            &mut self,
            path: A,
        ) -> Result<(), MetadataError> {
            let pkg = Package::from_file(&path)?;
            self.add_package(&pkg)?;
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// Creates an RPM dependency for require-filtering tests.
        fn dependency(name: &str, flags: rpm::DependencyFlags, version: &str) -> rpm::Dependency {
            rpm::Dependency {
                name: name.to_owned(),
                flags,
                version: version.to_owned(),
            }
        }

        /// Returns requirement names to keep filtering assertions focused on selection and order.
        fn requirement_names(requires: Vec<Requirement>) -> Vec<String> {
            requires
                .into_iter()
                .map(|requirement| requirement.name().to_owned())
                .collect()
        }

        /// Preserves valid comparator bits and drops combinations that XML cannot represent.
        #[test]
        fn preserves_only_valid_rpm_comparison_operators() {
            // Other dependency bits must not change the comparator, while contradictory bounds
            // have no representation in repository metadata.
            assert_eq!(
                dependency_flags(
                    rpm::DependencyFlags::GREATER
                        | rpm::DependencyFlags::EQUAL
                        | rpm::DependencyFlags::PREREQ,
                ),
                Some(RequirementType::GE)
            );
            assert_eq!(
                dependency_flags(rpm::DependencyFlags::LESS | rpm::DependencyFlags::GREATER,),
                None
            );
        }

        /// Accepts only nonnegative integer epochs.
        #[test]
        fn rejects_invalid_dependency_epochs() {
            assert!(has_valid_epoch("1:1"));
            assert!(!has_valid_epoch("-1:1"));
            assert!(!has_valid_epoch("999999999999999999999:1"));
            assert!(!has_valid_epoch("invalid:1"));
        }

        /// Filters RPM internals, self-provides, and malformed epochs without conflating EVRs.
        #[test]
        fn filters_redundant_requires() {
            // Raw keys prevent distinct EVRs from colliding while RPM internals and malformed
            // epochs remain excluded.
            let provided = HashSet::from([dependency_key(&dependency(
                "provided",
                rpm::DependencyFlags::EQUAL,
                "1:23",
            ))]);
            let requires = filter_requires(
                vec![
                    dependency("rpmlib(PayloadIsXz)", rpm::DependencyFlags::LE, "5.2"),
                    dependency("provided", rpm::DependencyFlags::EQUAL, "1:23"),
                    dependency("provided", rpm::DependencyFlags::EQUAL, "12:3"),
                    dependency("bad-epoch", rpm::DependencyFlags::ANY, "not-an-epoch:1"),
                    dependency(
                        "large-epoch",
                        rpm::DependencyFlags::ANY,
                        "999999999999999999999:1",
                    ),
                    dependency("numeric-epoch", rpm::DependencyFlags::ANY, "1:1"),
                    dependency("kept", rpm::DependencyFlags::ANY, ""),
                ],
                &provided,
                &crate::FileList::new(),
            )
            .unwrap();

            assert_eq!(
                requirement_names(requires),
                ["provided", "numeric-epoch", "kept"]
            );
        }

        /// Removes exact duplicate requirements regardless of where they occur in the header.
        #[test]
        fn removes_duplicate_requires_regardless_of_order() {
            // The intervening, different constraint must remain while the final duplicate is dropped.
            let requires = filter_requires(
                vec![
                    dependency("duplicate", rpm::DependencyFlags::EQUAL, "1"),
                    dependency("duplicate", rpm::DependencyFlags::EQUAL, "1"),
                    dependency("duplicate", rpm::DependencyFlags::GREATER, "2"),
                    dependency("duplicate", rpm::DependencyFlags::EQUAL, "1"),
                ],
                &HashSet::new(),
                &crate::FileList::new(),
            )
            .unwrap();

            assert_eq!(requires.len(), 2);
            assert_eq!(requires[0].flags(), Some(RequirementType::EQ));
            assert_eq!(requires[0].version(), Some("1"));
            assert_eq!(requires[1].flags(), Some(RequirementType::GT));
            assert_eq!(requires[1].version(), Some("2"));
        }

        /// Retains the greatest libc symbol-version capability after ordinary requirements.
        #[test]
        fn retains_only_the_highest_libc_requirement() {
            // libc capabilities are condensed, and the retained capability follows other requires.
            let requires = filter_requires(
                vec![
                    dependency("other", rpm::DependencyFlags::ANY, ""),
                    dependency("libc.so.6(GLIBC_2.3)", rpm::DependencyFlags::ANY, ""),
                    dependency("libc.so.6(GLIBC_2.4)", rpm::DependencyFlags::ANY, ""),
                ],
                &HashSet::new(),
                &crate::FileList::new(),
            )
            .unwrap();

            assert_eq!(
                requirement_names(requires),
                ["other", "libc.so.6(GLIBC_2.4)"]
            );
        }

        /// Orders libc capability forms before numeric symbol versions.
        #[test]
        fn orders_libc_capability_forms() {
            let forms = [
                "libc.so.6",
                "libc.so.6(GLIBC_2",
                "libc.so.6()(64bit)",
                "libc.so.6(GLIBC_ABI_DT_RELR)(64bit)",
                "libc.so.6(GLIBC_2.38)(64bit)",
            ];

            // Each form is less specific than the following form.
            for pair in forms.windows(2) {
                assert_eq!(compare_libc_requirements(pair[0], pair[1]), Ordering::Less);
            }
        }

        /// Marks transaction scriptlet requirements as installation-ordering requirements.
        #[test]
        fn marks_transaction_scriptlet_requires_as_preinstall() {
            // Transaction scriptlets participate in installation ordering just like %pre and %post.
            let requires = filter_requires(
                vec![
                    dependency("pretrans", rpm::DependencyFlags::PRETRANS, ""),
                    dependency("posttrans", rpm::DependencyFlags::POSTTRANS, ""),
                ],
                &HashSet::new(),
                &crate::FileList::new(),
            )
            .unwrap();

            assert!(requires.iter().all(Requirement::preinstall));
        }

        /// Does not apply requirement-only ordering flags to weak dependencies.
        #[test]
        fn does_not_mark_weak_dependencies_preinstall() {
            let requirement = Requirement::try_from(dependency(
                "group(example)",
                rpm::DependencyFlags::SCRIPT_PRE | rpm::DependencyFlags::SCRIPT_POSTUN,
                "",
            ))
            .unwrap();

            assert!(!requirement.preinstall());
        }

        /// Matches createrepo_c when an RPM entry is both a directory and ghost.
        #[test]
        fn maps_ghost_directories_to_directory_entries() {
            assert_eq!(
                repository_file_type(rpm::FileType::Dir, rpm::FileFlags::GHOST),
                crate::FileType::Dir
            );
        }

        /// Preserves the ghost marker for non-directory file entries.
        #[test]
        fn maps_ghost_files_to_ghost_entries() {
            assert_eq!(
                repository_file_type(rpm::FileType::Regular, rpm::FileFlags::GHOST),
                crate::FileType::Ghost
            );
        }

        /// Represents RPM file types unsupported by RPM-MD without failing parsing.
        #[test]
        fn maps_unrepresentable_rpm_file_types_to_files() {
            assert_eq!(
                repository_file_type(rpm::FileType::Other, rpm::FileFlags::empty()),
                crate::FileType::File
            );
        }

        /// Keeps normal RPM directory and regular-file metadata unchanged.
        #[test]
        fn preserves_directory_and_regular_file_types() {
            assert_eq!(
                repository_file_type(rpm::FileType::Dir, rpm::FileFlags::empty()),
                crate::FileType::Dir
            );
            assert_eq!(
                repository_file_type(rpm::FileType::Regular, rpm::FileFlags::empty()),
                crate::FileType::File
            );
        }
    }
}

/// Iterator over packages in a repository, merging data from primary, filelists, and other XML.
pub struct PackageIterator {
    primary_xml: PrimaryXmlReader<BufReader<Box<dyn std::io::Read + Send>>>,
    filelists_xml: FilelistsXmlReader<BufReader<Box<dyn std::io::Read + Send>>>,
    other_xml: OtherXmlReader<BufReader<Box<dyn std::io::Read + Send>>>,

    num_packages: usize,
    num_remaining: usize,
    in_progress_package: Option<Package>,
}

impl PackageIterator {
    /// Create an iterator from repodata on disk, using paths from the given [`RepomdData`].
    pub fn from_repodata(base: &Path, repomd: &RepomdData) -> Result<Self, MetadataError> {
        let primary_path = base.join(
            &repomd
                .get_record(mdrecord::MD_PRIMARY)
                .unwrap()
                .location_href,
        );
        let filelists_path = base.join(
            &repomd
                .get_record(mdrecord::MD_FILELISTS)
                .unwrap()
                .location_href,
        );
        let other_path = base.join(&repomd.get_record(mdrecord::MD_OTHER).unwrap().location_href);
        Self::from_files(&primary_path, &filelists_path, &other_path)
    }

    /// Create an iterator from explicit primary, filelists, and other XML file paths.
    pub fn from_files(
        primary_path: &Path,
        filelists_path: &Path,
        other_path: &Path,
    ) -> Result<Self, MetadataError> {
        let primary_xml = PrimaryXml::new_reader(utils::xml_reader_from_file(primary_path)?);
        let filelists_xml = FilelistsXml::new_reader(utils::xml_reader_from_file(filelists_path)?);
        let other_xml = OtherXml::new_reader(utils::xml_reader_from_file(other_path)?);

        Self::from_readers(primary_xml, filelists_xml, other_xml)
    }

    /// Create an iterator from pre-constructed XML readers.
    pub fn from_readers(
        primary_xml: PrimaryXmlReader<BufReader<Box<dyn std::io::Read + Send>>>,
        filelists_xml: FilelistsXmlReader<BufReader<Box<dyn std::io::Read + Send>>>,
        other_xml: OtherXmlReader<BufReader<Box<dyn std::io::Read + Send>>>,
    ) -> Result<Self, MetadataError> {
        let mut parser = Self {
            primary_xml,
            filelists_xml,
            other_xml,
            num_packages: 0,
            num_remaining: 0,
            in_progress_package: None,
        };
        parser.parse_headers()?;

        Ok(parser)
    }

    fn parse_headers(&mut self) -> Result<(), MetadataError> {
        let primary_pkg_count = self.primary_xml.read_header()?;
        let _filelists_pkg_count = self.filelists_xml.read_header()?;
        let _other_pkg_count = self.other_xml.read_header()?;

        self.num_packages = primary_pkg_count;
        self.num_remaining = self.num_packages;

        Ok(())
    }

    /// Parse the next package from the XML streams, or `None` if exhausted.
    pub fn parse_package(&mut self) -> Result<Option<Package>, MetadataError> {
        self.primary_xml
            .read_package(&mut self.in_progress_package)?;
        self.filelists_xml
            .read_package(&mut self.in_progress_package)?;
        self.other_xml.read_package(&mut self.in_progress_package)?;

        let package = self.in_progress_package.take();

        // TODO: re-enable this with actual error handling instead of panics - RHEL6 for example will fail
        // because the header lies about the number of packages
        if package.is_some() {
            self.num_remaining = self.num_remaining.saturating_sub(1);
        }

        Ok(package)
    }

    /// Returns the number of packages not yet yielded.
    pub fn remaining_packages(&self) -> usize {
        self.num_remaining
    }

    /// Returns the total number of packages declared in the metadata headers.
    pub fn total_packages(&self) -> usize {
        self.num_packages
    }
}

impl Iterator for PackageIterator {
    type Item = Result<Package, MetadataError>;
    fn next(&mut self) -> Option<Self::Item> {
        self.parse_package().transpose()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.remaining_packages()))
    }
}
