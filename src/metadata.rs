// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::convert::TryInto;
use std::hash::{Hash, Hasher};
use std::io::{BufRead, Write};
use std::os::unix::prelude::MetadataExt;
use std::path::{Path, PathBuf};

// use bitflags;
use niffler;
use quick_xml;
use quick_xml::{Reader, Writer};
#[cfg(feature = "read_rpm")]
use rpm;
use thiserror::Error;

use crate::{EVR, Repository, utils};

pub struct RepomdXml;
pub struct PrimaryXml;
pub struct FilelistsXml;
pub struct OtherXml;
pub struct UpdateinfoXml;

pub const METADATA_PRIMARY: &str = "primary";
pub const METADATA_FILELISTS: &str = "filelists";
pub const METADATA_OTHER: &str = "other";
// pub const METADATA_PRIMARY_ZCK: &str = "primary_zck";
// pub const METADATA_FILELISTS_ZCK: &str = "filelists_zck";
// pub const METADATA_OTHER_ZCK: &str = "other_zck";
pub const METADATA_UPDATEINFO: &str = "updateinfo";

// TODO: probably this can / should be broken up better rather than being a kitchen sink
#[derive(Error, Debug)]
pub enum MetadataError {
    #[cfg(feature = "read_rpm")]
    #[error(transparent)]
    RpmReadError(#[from] rpm::Error),
    #[error(transparent)]
    XmlParseError(#[from] quick_xml::Error),
    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    IntFieldParseError(#[from] std::num::ParseIntError),
    #[error(transparent)]
    UnsupportedCompressionTypeError(#[from] niffler::Error),
    #[error("Checksum type {0} is not supported")]
    UnsupportedChecksumTypeError(String),
    #[error("\"{0}\" is not a valid checksum of type \"{1:?}\"")]
    InvalidChecksumError(String, ChecksumType),
    #[error("\"{0}\" is not a valid flag value")]
    InvalidFlagsError(String),
    #[error("\"{0}\" is not a valid EVR string: {1}")]
    InvalidEvrError(String, String),
    #[error("Metadata files are inconsistent: {0}")]
    InconsistentMetadataError(String),
    #[error("Missing metadata field: {0}")]
    MissingFieldError(&'static str),
    #[error("Missing metadata attribute: {0}")]
    MissingAttributeError(&'static str),
    #[error("Unknown metadata attribute: {0}")]
    UnknownAttributeError(String),
    #[error("Missing metadata header")]
    MissingHeaderError,
}

// #[derive(Error, Debug)]
// pub enum RpmrepoError {

// }

/// Default namespace for primary.xml
pub const XML_NS_COMMON: &str = "http://linux.duke.edu/metadata/common";
/// Default namespace for filelists.xml
pub const XML_NS_FILELISTS: &str = "http://linux.duke.edu/metadata/filelists";
/// Default namespace for other.xml
pub const XML_NS_OTHER: &str = "http://linux.duke.edu/metadata/other";
/// Default namespace for repomd.xml
pub const XML_NS_REPO: &str = "http://linux.duke.edu/metadata/repo";
/// Namespace for rpm (used in primary.xml and repomd.xml)
pub const XML_NS_RPM: &str = "http://linux.duke.edu/metadata/rpm";

pub trait RpmMetadata {
    fn filename() -> &'static str;

    fn load_metadata<R: BufRead>(
        repository: &mut Repository,
        buffer: Reader<R>,
    ) -> Result<(), MetadataError>;

    fn write_metadata<W: Write>(
        repository: &Repository,
        buffer: Writer<W>,
    ) -> Result<(), MetadataError>;
}

// TODO: Trait impl tests https://github.com/rust-lang/rfcs/issues/616

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompressionType {
    None,
    Gzip,
    Xz,
    Bz2,
    Zstd,
}

impl CompressionType {
    pub fn to_file_extension(&self) -> &str {
        match self {
            CompressionType::None => "",
            CompressionType::Gzip => ".gz",
            CompressionType::Xz => ".xz",
            CompressionType::Bz2 => ".bz2",
            CompressionType::Zstd => ".zst",
        }
    }
}

impl TryInto<CompressionType> for &str {
    type Error = MetadataError;

    fn try_into(self) -> Result<CompressionType, Self::Error> {
        match self {
            "gzip" => Ok(CompressionType::Gzip),
            "bz2" => Ok(CompressionType::Bz2),
            "xz" => Ok(CompressionType::Xz),
            "zstd" => Ok(CompressionType::Zstd),
            "none" => Ok(CompressionType::None),
            _ => Err(MetadataError::UnsupportedChecksumTypeError(self.to_owned())),
        }
    }
}

// impl Ord for Package {
//     #[inline]
//     fn cmp(&self, other: &Package) -> Ordering {
//         other.0.cmp(&self.0)
//     }
// }

// bitflags::bitflags! {
//     #[derive(Default)]
//     pub struct ParseState: u8 {
//         const NONE = 0b00000000;
//         const PRIMARY = 0b00000001;
//         const FILELISTS = 0b00000010;
//         const OTHER = 0b00000100;
//         const PRIMARY_WITH_FILES = 0b00001001;
//     }
// }

#[derive(Clone, Default, Debug, PartialEq, Hash)]
pub struct Package {
    // pub(crate) parse_state: ParseState,
    pub name: String,
    pub arch: String,
    pub evr: EVR,
    pub checksum: Checksum,
    pub location_href: String,
    pub location_base: Option<String>,
    pub summary: String,
    pub description: String,
    pub packager: String,
    pub url: String,
    pub time_file: u64,
    pub time_build: u64,
    pub size_package: u64,
    pub size_installed: u64,
    pub size_archive: u64,

    pub rpm_license: String,           // rpm:license
    pub rpm_vendor: String,            // rpm:vendor
    pub rpm_group: String,             // rpm:group
    pub rpm_buildhost: String,         // rpm:buildhost
    pub rpm_sourcerpm: String,         // rpm:sourcerpm
    pub rpm_header_range: HeaderRange, // rpm:header-range

    pub rpm_requires: Vec<Requirement>,    // rpm:provides
    pub rpm_provides: Vec<Requirement>,    // rpm:requires
    pub rpm_conflicts: Vec<Requirement>,   // rpm:conflicts
    pub rpm_obsoletes: Vec<Requirement>,   // rpm:obsoletes
    pub rpm_suggests: Vec<Requirement>,    // rpm:suggests
    pub rpm_enhances: Vec<Requirement>,    // rpm:enhances
    pub rpm_recommends: Vec<Requirement>,  // rpm:recommends
    pub rpm_supplements: Vec<Requirement>, // rpm:supplements

    pub rpm_changelogs: Vec<Changelog>,
    pub rpm_files: Vec<PackageFile>,
}

impl Package {
    pub fn new(
        name: &str,
        version: &EVR,
        arch: &str,
        checksum: &Checksum,
        location_href: &str,
    ) -> Package {
        Package {
            name: name.to_owned(),
            arch: arch.to_owned(),
            // TODO: https://github.com/rust-lang/rust/issues/107115
            evr: EVR::new(
                version.epoch().to_owned(),
                version.version().to_owned(),
                version.release().to_owned(),
            ),
            checksum: checksum.clone(),
            location_href: location_href.to_owned(),
            ..Package::default()
        }
    }

    pub fn set_name(&mut self, name: impl Into<String>) -> &mut Self {
        self.name = name.into();
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_epoch(&mut self, epoch: u32) -> &mut Self {
        self.evr.epoch = epoch.to_string().into();
        self
    }

    pub fn epoch(&self) -> u32 {
        self.evr.epoch.parse().expect("TODO: don't do this")
    }

    pub fn set_version(&mut self, version: impl Into<String>) -> &mut Self {
        self.evr.version = version.into().into();
        self
    }

    pub fn version(&self) -> &str {
        &self.evr.version
    }

    pub fn set_release(&mut self, release: impl Into<String>) -> &mut Self {
        self.evr.release = release.into().into();
        self
    }

    pub fn release(&self) -> &str {
        &self.evr.release
    }

    pub fn set_arch(&mut self, arch: impl Into<String>) -> &mut Self {
        self.arch = arch.into();
        self
    }

    pub fn arch(&self) -> &str {
        &self.arch
    }

    // TODO: signature
    // TODO: https://github.com/rust-lang/rust/issues/107115
    pub fn set_evr(&mut self, evr: EVR) -> &mut Self {
        let evr = EVR::new(
            evr.epoch().to_owned(),
            evr.version().to_owned(),
            evr.release().to_owned(),
        );
        self.evr = evr;
        self
    }

    pub fn evr(&self) -> &EVR {
        &self.evr
    }

    pub fn nvra(&self) -> String {
        format!(
            "{}-{}-{}.{}",
            self.name, self.evr.version, self.evr.release, self.arch
        )
    }

    pub fn nevra_short(&self) -> String {
        if self.evr.epoch == "0" {
            self.nvra()
        } else {
            self.nevra()
        }
    }

    pub fn nevra(&self) -> String {
        format!(
            "{}-{}:{}-{}.{}",
            self.name, self.evr.epoch, self.evr.version, self.evr.release, self.arch
        )
    }
    // TODO: signature
    pub fn set_checksum(&mut self, checksum: Checksum) -> &mut Self {
        self.checksum = checksum;
        self
    }

    pub fn checksum(&self) -> &Checksum {
        &self.checksum
    }

    pub fn pkgid(&self) -> &str {
        // TODO: better way to do this
        &self.checksum.to_values().unwrap().1
    }

    pub fn set_location_href(&mut self, location_href: impl Into<String>) -> &mut Self {
        self.location_href = location_href.into();
        self
    }

    pub fn location_href(&self) -> &str {
        &self.location_href
    }

    pub fn set_location_base(&mut self, location_base: Option<impl Into<String>>) -> &mut Self {
        self.location_base = location_base.and_then(|a| Some(a.into()));
        self
    }

    pub fn location_base(&self) -> Option<&str> {
        self.location_base.as_ref().and_then(|a| Some(a.as_ref()))
    }

    pub fn set_summary(&mut self, summary: impl Into<String>) -> &mut Self {
        self.summary = summary.into();
        self
    }

    pub fn summary(&self) -> &str {
        &self.summary
    }

    pub fn set_description(&mut self, description: impl Into<String>) -> &mut Self {
        self.description = description.into();
        self
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn set_packager(&mut self, packager: impl Into<String>) -> &mut Self {
        self.packager = packager.into();
        self
    }

    pub fn packager(&self) -> &str {
        &self.packager
    }

    pub fn set_url(&mut self, url: impl Into<String>) -> &mut Self {
        self.url = url.into();
        self
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn set_time_file(&mut self, time_file: u64) -> &mut Self {
        self.time_file = time_file;
        self
    }

    pub fn time_file(&self) -> u64 {
        self.time_file
    }

    pub fn set_time_build(&mut self, time_build: u64) -> &mut Self {
        self.time_build = time_build;
        self
    }

    pub fn time_build(&self) -> u64 {
        self.time_build
    }

    pub fn set_size_package(&mut self, size_package: u64) -> &mut Self {
        self.size_package = size_package;
        self
    }

    pub fn size_package(&self) -> u64 {
        self.size_package
    }

    pub fn set_size_installed(&mut self, size_installed: u64) -> &mut Self {
        self.size_installed = size_installed;
        self
    }

    pub fn size_installed(&self) -> u64 {
        self.size_installed
    }

    pub fn set_size_archive(&mut self, size_archive: u64) -> &mut Self {
        self.size_archive = size_archive;
        self
    }

    pub fn size_archive(&self) -> u64 {
        self.size_archive
    }

    pub fn set_rpm_license(&mut self, license: impl Into<String>) -> &mut Self {
        self.rpm_license = license.into();
        self
    }

    pub fn rpm_license(&self) -> &str {
        &self.rpm_license
    }

    pub fn set_rpm_vendor(&mut self, vendor: impl Into<String>) -> &mut Self {
        self.rpm_vendor = vendor.into();
        self
    }

    pub fn rpm_vendor(&self) -> &str {
        &self.rpm_vendor
    }

    pub fn set_rpm_group(&mut self, group: impl Into<String>) -> &mut Self {
        self.rpm_group = group.into();
        self
    }

    pub fn rpm_group(&self) -> &str {
        &self.rpm_group
    }

    pub fn set_rpm_buildhost(&mut self, rpm_buildhost: impl Into<String>) -> &mut Self {
        self.rpm_buildhost = rpm_buildhost.into();
        self
    }

    pub fn rpm_buildhost(&self) -> &str {
        &self.rpm_buildhost
    }

    pub fn set_rpm_sourcerpm(&mut self, rpm_sourcerpm: impl Into<String>) -> &mut Self {
        self.rpm_sourcerpm = rpm_sourcerpm.into();
        self
    }

    pub fn rpm_sourcerpm(&self) -> &str {
        &self.rpm_sourcerpm
    }

    pub fn set_rpm_header_range(&mut self, start: u64, end: u64) -> &mut Self {
        self.rpm_header_range = HeaderRange { start, end };
        self
    }

    pub fn rpm_header_range(&self) -> &HeaderRange {
        &self.rpm_header_range
    }

    // TODO: probably adjust the signatures on all of these w/ builder pattern or something
    pub fn set_requires(&mut self, requires: Vec<Requirement>) -> &mut Self {
        self.rpm_requires = requires;
        self
    }

    pub fn requires(&self) -> &[Requirement] {
        &self.rpm_requires
    }

    pub fn set_provides(&mut self, provides: Vec<Requirement>) -> &mut Self {
        self.rpm_provides = provides;
        self
    }

    pub fn provides(&self) -> &[Requirement] {
        &self.rpm_provides
    }

    pub fn set_conflicts(&mut self, conflicts: Vec<Requirement>) -> &mut Self {
        self.rpm_conflicts = conflicts;
        self
    }

    pub fn conflicts(&self) -> &[Requirement] {
        &self.rpm_conflicts
    }

    pub fn set_obsoletes(&mut self, obsoletes: Vec<Requirement>) -> &mut Self {
        self.rpm_obsoletes = obsoletes;
        self
    }

    pub fn obsoletes(&self) -> &[Requirement] {
        &self.rpm_obsoletes
    }

    pub fn set_suggests(&mut self, suggests: Vec<Requirement>) -> &mut Self {
        self.rpm_suggests = suggests;
        self
    }

    pub fn suggests(&self) -> &[Requirement] {
        &self.rpm_suggests
    }

    pub fn set_enhances(&mut self, enhances: Vec<Requirement>) -> &mut Self {
        self.rpm_enhances = enhances;
        self
    }

    pub fn enhances(&self) -> &[Requirement] {
        &self.rpm_enhances
    }

    pub fn set_recommends(&mut self, recommends: Vec<Requirement>) -> &mut Self {
        self.rpm_recommends = recommends;
        self
    }

    pub fn recommends(&self) -> &[Requirement] {
        &self.rpm_recommends
    }

    pub fn set_supplements(&mut self, supplements: Vec<Requirement>) -> &mut Self {
        self.rpm_supplements = supplements;
        self
    }

    pub fn supplements(&self) -> &[Requirement] {
        &self.rpm_supplements
    }

    pub fn add_file(&mut self, filetype: FileType, path: &str) -> &mut Self {
        self.rpm_files.push(PackageFile {
            filetype,
            path: path.to_owned(),
        });
        self
    }

    pub fn set_files(&mut self, files: Vec<PackageFile>) -> &mut Self {
        self.rpm_files = files;
        self
    }

    pub fn files(&self) -> &[PackageFile] {
        &self.rpm_files
    }

    pub fn add_changelog(&mut self, author: &str, description: &str, date: u64) -> &mut Self {
        self.rpm_changelogs.push(Changelog {
            author: author.to_owned(),
            timestamp: date,
            description: description.to_owned(),
        });
        self
    }

    pub fn set_changelogs(&mut self, changelogs: Vec<Changelog>) -> &mut Self {
        self.rpm_changelogs = changelogs;
        self
    }

    pub fn changelogs(&self) -> &[Changelog] {
        &self.rpm_changelogs
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChecksumType {
    Md5,
    Sha1,
    Sha224,
    Sha256,
    Sha384,
    Sha512,
    Unknown,
}

impl Default for ChecksumType {
    fn default() -> Self {
        ChecksumType::Sha256
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Checksum {
    Md5(String),
    Sha1(String),
    Sha224(String),
    Sha256(String),
    Sha384(String),
    Sha512(String),
    Unknown(String),
    Empty,
}

impl Default for Checksum {
    fn default() -> Self {
        Checksum::Empty
    }
}

impl Hash for Checksum {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Md5(hash) => format!("md5:{}", hash).hash(state),
            Self::Sha1(hash) => format!("sha1:{}", hash).hash(state),
            Self::Sha224(hash) => format!("sha224:{}", hash).hash(state),
            Self::Sha256(hash) => format!("sha256:{}", hash).hash(state),
            Self::Sha384(hash) => format!("sha384:{}", hash).hash(state),
            Self::Sha512(hash) => format!("sha512:{}", hash).hash(state),
            // TODO: adjust this representation. Currently these exist because of reuse of these enums
            // to represent intermediate parsing states, but those probably ought to be pulled out somehow
            Self::Unknown(hash) => unimplemented!(),
            Self::Empty => unimplemented!(),
        }
    }
}

impl Checksum {
    pub fn try_create<N: AsRef<[u8]> + Sized>(
        checksum_type: N,
        checksum: N,
    ) -> Result<Self, MetadataError> {
        let bytes_to_str = |value| std::str::from_utf8(value).unwrap().to_owned();

        match checksum_type.as_ref() {
            b"md5" => {
                let digest = bytes_to_str(checksum.as_ref());
                if digest.len() != 32 {
                    Err(MetadataError::InvalidChecksumError(
                        digest,
                        ChecksumType::Md5,
                    ))
                } else {
                    Ok(Checksum::Md5(digest))
                }
            }
            b"sha" => {
                let digest = bytes_to_str(checksum.as_ref());
                if digest.len() != 40 {
                    Err(MetadataError::InvalidChecksumError(
                        digest,
                        ChecksumType::Sha1,
                    ))
                } else {
                    Ok(Checksum::Sha1(digest))
                }
            }
            b"sha1" => {
                let digest = bytes_to_str(checksum.as_ref());
                if digest.len() != 40 {
                    Err(MetadataError::InvalidChecksumError(
                        digest,
                        ChecksumType::Sha1,
                    ))
                } else {
                    Ok(Checksum::Sha1(digest))
                }
            }
            b"sha224" => {
                let digest = bytes_to_str(checksum.as_ref());
                if digest.len() != 56 {
                    Err(MetadataError::InvalidChecksumError(
                        digest,
                        ChecksumType::Sha224,
                    ))
                } else {
                    Ok(Checksum::Sha224(digest))
                }
            }
            b"sha256" => {
                let digest = bytes_to_str(checksum.as_ref());
                if digest.len() != 64 {
                    Err(MetadataError::InvalidChecksumError(
                        digest,
                        ChecksumType::Sha256,
                    ))
                } else {
                    Ok(Checksum::Sha256(digest))
                }
            }
            b"sha384" => {
                let digest = bytes_to_str(checksum.as_ref());
                if digest.len() != 96 {
                    Err(MetadataError::InvalidChecksumError(
                        digest,
                        ChecksumType::Sha384,
                    ))
                } else {
                    Ok(Checksum::Sha384(digest))
                }
            }
            b"sha512" => {
                let digest = bytes_to_str(checksum.as_ref());
                if digest.len() != 128 {
                    Err(MetadataError::InvalidChecksumError(
                        digest,
                        ChecksumType::Sha512,
                    ))
                } else {
                    Ok(Checksum::Sha512(digest))
                }
            }
            _ => {
                return Err(MetadataError::UnsupportedChecksumTypeError(bytes_to_str(
                    checksum_type.as_ref(),
                )));
            }
        }
    }

    pub fn to_values<'a>(&'a self) -> Result<(&str, &'a str), MetadataError> {
        let values = match self {
            Checksum::Md5(c) => ("md5", c.as_str()),
            Checksum::Sha1(c) => ("sha1", c.as_str()),
            Checksum::Sha224(c) => ("sha224", c.as_str()),
            Checksum::Sha256(c) => ("sha256", c.as_str()),
            Checksum::Sha384(c) => ("sha384", c.as_str()),
            Checksum::Sha512(c) => ("sha512", c.as_str()),
            Checksum::Unknown(c) => ("unknown", c.as_str()), // TODO: need to fix this - if filelists is loaded w/o metadata the pkgid is known but the type is not
            Checksum::Empty => panic!("Cannot take value of empty checksum"),
        };
        Ok(values)
    }
}

#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct Changelog {
    pub author: String,
    pub timestamp: u64,
    pub description: String,
}

#[derive(Copy, Clone, Debug, Default, Hash, PartialEq)]
pub struct HeaderRange {
    pub start: u64,
    pub end: u64,
}

// Requirement (Provides, Conflicts, Obsoletes, Requires).
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct Requirement {
    pub name: String,
    pub flags: Option<String>,
    pub epoch: Option<String>,
    pub version: Option<String>,
    pub release: Option<String>,
    pub preinstall: bool,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq)]
pub enum RequirementType {
    LT,
    GT,
    EQ,
    LE,
    GE,
}

impl From<RequirementType> for &str {
    fn from(rtype: RequirementType) -> &'static str {
        match rtype {
            RequirementType::LT => "LT",
            RequirementType::GT => "GT",
            RequirementType::EQ => "EQ",
            RequirementType::LE => "LE",
            RequirementType::GE => "GE",
        }
    }
}

impl TryFrom<&str> for RequirementType {
    type Error = MetadataError;

    fn try_from(flags: &str) -> Result<Self, Self::Error> {
        let reqtype = match flags {
            "LT" => RequirementType::LT,
            "GT" => RequirementType::GT,
            "EQ" => RequirementType::EQ,
            "LE" => RequirementType::LE,
            "GE" => RequirementType::GE,
            t @ _ => return Err(MetadataError::InvalidFlagsError(t.to_owned())),
        };

        Ok(reqtype)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Hash)]
pub enum FileType {
    File,
    Dir,
    Ghost,
}

// TODO: this is unnecessary / not the best way
impl FileType {
    pub fn try_create<N: AsRef<[u8]> + Sized>(val: N) -> Result<Self, MetadataError> {
        let ftype = match val.as_ref() {
            b"dir" => FileType::Dir,
            b"ghost" => FileType::Ghost,
            b"file" => FileType::File,
            _ => panic!(),
        };
        Ok(ftype)
    }

    pub fn to_values(&self) -> &[u8] {
        match self {
            FileType::File => b"file",
            FileType::Dir => b"dir",
            FileType::Ghost => b"ghost",
        }
    }
}

impl Default for FileType {
    fn default() -> Self {
        FileType::File
    }
}

#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct PackageFile {
    pub filetype: FileType,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MetadataType {
    Primary,
    Filelists,
    Other,

    // PrimaryZck,
    // FilelistsZck,
    // OtherZck,

    // PrimaryDb,
    // FilelistsDb,
    // OtherDb,
    Unknown,
}

impl From<&str> for MetadataType {
    fn from(name: &str) -> Self {
        match name {
            METADATA_PRIMARY => MetadataType::Primary,
            METADATA_FILELISTS => MetadataType::Filelists,
            METADATA_OTHER => MetadataType::Other,

            // METADATA_PRIMARY_DB => MetadataType::PrimaryDb,
            // METADATA_FILELISTS_DB => MetadataType::FilelistsDb,
            // METADATA_OTHER_DB => MetadataType::OtherDb,

            // METADATA_PRIMARY_ZCK => MetadataType::PrimaryZck,
            // METADATA_FILELISTS_ZCK => MetadataType::FilelistsZck,
            // METADATA_OTHER_ZCK => MetadataType::OtherZck,
            _ => MetadataType::Unknown,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct DistroTag {
    pub cpeid: Option<String>,
    pub name: String,
}

impl DistroTag {
    pub fn new(name: String, cpeid: Option<String>) -> Self {
        DistroTag { name, cpeid }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct RepomdData {
    revision: Option<String>,
    metadata_files: Vec<RepomdRecord>,

    // checksum_type: ChecksumType,
    repo_tags: Vec<String>,
    content_tags: Vec<String>,
    distro_tags: Vec<DistroTag>,
}

impl RepomdData {
    pub fn add_record(&mut self, record: RepomdRecord) {
        self.metadata_files.push(record);
    }

    pub fn get_record(&self, rectype: &str) -> Option<&RepomdRecord> {
        self.metadata_files
            .iter()
            .find(|r| &r.metadata_name == rectype)
    }

    pub fn records(&self) -> &Vec<RepomdRecord> {
        &self.metadata_files
    }

    // pub fn records(&self) -> &BTreeMap<String, RepomdRecord> {
    //     &self.metadata_files
    // }

    // pub fn records_mut(&self) -> &mut BTreeMap<String, RepomdRecord> {
    //     &mut self.metadata_files
    // }

    // pub fn remove_record(&mut self, rectype: &str) {
    //     self.metadata_files.retain(|r| &r.mdtype != rectype);
    // }

    pub fn add_repo_tag(&mut self, repo: String) {
        self.repo_tags.push(repo)
    }

    pub fn repo_tags(&self) -> &Vec<String> {
        &self.repo_tags
    }

    pub fn add_content_tag(&mut self, content: String) {
        self.content_tags.push(content)
    }

    pub fn content_tags(&self) -> &Vec<String> {
        &self.content_tags
    }

    pub fn add_distro_tag(&mut self, name: String, cpeid: Option<String>) {
        let distro = DistroTag { name, cpeid };
        self.distro_tags.push(distro)
    }

    pub fn distro_tags(&self) -> &Vec<DistroTag> {
        &self.distro_tags
    }

    pub fn set_revision(&mut self, revision: &str) {
        self.revision = Some(revision.to_owned());
    }

    pub fn revision(&self) -> Option<&str> {
        self.revision.as_deref()
    }

    pub fn sort_records(&mut self) {
        fn value(item: &RepomdRecord) -> u32 {
            let mdtype = MetadataType::from(item.metadata_name.as_str());
            match mdtype {
                MetadataType::Primary => 1,
                MetadataType::Filelists => 2,
                MetadataType::Other => 3,
                // MetadataType::PrimaryDb => 4,
                // MetadataType::FilelistsDb => 5,
                // MetadataType::OtherDb => 6,
                // MetadataType::PrimaryZck => 7,
                // MetadataType::FilelistsZck => 8,
                // MetadataType::OtherZck => 9,
                MetadataType::Unknown => 10,
            }
        }
        self.metadata_files.sort_by(|a, b| value(a).cmp(&value(b)));
    }

    // TODO error handling
    pub fn get_primary_data(&self) -> &RepomdRecord {
        self.get_record(METADATA_PRIMARY)
            .expect("Cannot find primary metadata")
    }

    pub fn get_filelist_data(&self) -> &RepomdRecord {
        self.get_record(METADATA_FILELISTS)
            .expect("Cannot find filelists metadata")
    }

    pub fn get_other_data(&self) -> &RepomdRecord {
        self.get_record(METADATA_OTHER)
            .expect("Cannot find other metadata")
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RepomdRecord {
    base_path: Option<PathBuf>,

    /// Record type
    pub metadata_name: String,
    /// Relative location of the file in a repository
    pub location_href: PathBuf,
    /// URL at which the location_href is relative - if it is not the current one
    pub location_base: Option<String>,
    /// Mtime of the file
    pub timestamp: i64,
    /// Size of the file
    pub size: Option<u64>,
    /// Checksum of the file
    pub checksum: Checksum,

    /// Size of the archive content
    pub open_size: Option<u64>,
    /// Checksum of the archive content
    pub open_checksum: Option<Checksum>,

    /// Size of the Zchunk header
    pub header_size: Option<u64>,
    /// Checksum of the Zchunk header
    pub header_checksum: Option<Checksum>,

    /// Database version (used only for sqlite databases like primary.sqlite etc.)
    pub database_version: Option<u32>,
}

impl RepomdRecord {
    pub fn new(
        name: &str,
        href: &Path,
        base: &Path,
        checksum_type: ChecksumType,
    ) -> Result<Self, MetadataError> {
        let mut record = RepomdRecord::default();
        record.metadata_name = name.to_owned();
        record.location_href = {
            // let href = href
            //     .strip_prefix(href.ancestors().nth(2).unwrap())
            //     .unwrap()
            //     .to_owned();
            assert!(href.starts_with("repodata/"));
            href.to_owned()
        };
        record.base_path = Some(base.to_owned());
        record.fill(checksum_type)?;
        Ok(record)
    }

    pub fn fill(&mut self, checksum_type: ChecksumType) -> Result<(), MetadataError> {
        let file_path = self
            .base_path
            .as_ref()
            .expect("cannot fill metadata if path not on disk")
            .join(&self.location_href);
        let file_metadata = file_path.metadata()?;
        self.timestamp = file_metadata.mtime();
        self.size = Some(file_metadata.size());
        self.checksum = utils::checksum_file(&file_path, checksum_type)?;
        self.open_checksum = utils::checksum_inner_file(&file_path, checksum_type)?;
        self.open_size = utils::size_inner_file(&file_path)?;

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct UpdateRecord {
    pub from: String,
    pub update_type: String,
    pub status: String,
    pub version: String,
    pub id: String,
    pub title: String,
    pub issued_date: Option<String>,
    pub updated_date: Option<String>,
    pub rights: String,
    pub release: String,
    pub pushcount: Option<String>, // deprecated?
    pub severity: String,
    pub summary: String,
    pub description: String,
    pub solution: String,
    // It's not clear that any metadata actually uses this
    // pub reboot_suggested: bool,
    pub references: Vec<UpdateReference>,
    pub pkglist: Vec<UpdateCollection>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct UpdateCollection {
    pub name: String,
    pub shortname: String,
    pub packages: Vec<UpdateCollectionPackage>,
    pub module: Option<UpdateCollectionModule>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct UpdateReference {
    pub href: String,
    pub id: String,
    pub title: String,
    pub reftype: String,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct UpdateCollectionPackage {
    pub epoch: String,
    pub filename: String,
    pub name: String,
    pub reboot_suggested: bool,
    pub restart_suggested: bool,
    pub relogin_suggested: bool,
    pub release: String,
    pub src: String,
    pub arch: String,
    pub checksum: Option<Checksum>,
    pub version: String,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct UpdateCollectionModule {
    pub name: String,
    pub stream: String,
    pub version: u64,
    pub context: String,
    pub arch: String,
}
