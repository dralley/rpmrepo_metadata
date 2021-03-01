use niffler;
use quick_xml;
use quick_xml::{Reader, Writer};

use std::io::{BufRead, Write};
use thiserror::Error;

use crate::RpmRepository;

pub struct RepomdXml;
pub struct PrimaryXml;
pub struct FilelistsXml;
pub struct OtherXml;

pub struct UpdateInfoXml;

pub const METADATA_PRIMARY: &str = "primary";
pub const METADATA_FILELISTS: &str = "filelists";
pub const METADATA_OTHER: &str = "other";
pub const METADATA_PRIMARY_DB: &str = "primary_db";
pub const METADATA_FILELISTS_DB: &str = "filelists_db";
pub const METADATA_OTHER_DB: &str = "other_db";
pub const METADATA_PRIMARY_ZCK: &str = "primary_zck";
pub const METADATA_FILELISTS_ZCK: &str = "filelists_zck";
pub const METADATA_OTHER_ZCK: &str = "other_zck";

#[derive(Error, Debug)]
pub enum MetadataError {
    #[error(transparent)]
    MetadataParseError(#[from] quick_xml::Error),
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
    #[error("Missing metadata fields: {0}")]
    MissingFieldError(&'static str), // TODO: support multiple missing fields?
    #[error("Missing metadata attributes: {0}")]
    MissingAttributeError(&'static str), // TODO: support multiple missing attributes?
}

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

pub(crate) trait RpmMetadata {
    const NAME: &'static str;

    fn load_metadata<R: BufRead>(
        repository: &mut RpmRepository,
        reader: &mut Reader<R>,
    ) -> Result<(), MetadataError>;

    fn write_metadata<W: Write>(
        repository: &RpmRepository,
        writer: &mut Writer<W>,
    ) -> Result<(), MetadataError>;
}

// TODO: Trait impl tests https://github.com/rust-lang/rfcs/issues/616

pub enum Compression {
    None,
    Gzip,
    Xz,
}

#[derive(Debug, PartialEq, Default)]
pub struct Package {
    pub name: String,
    pub arch: String,
    pub evr: EVR,
    pub checksum: Checksum,
    pub location_href: String,
    pub summary: String,
    pub description: String,
    pub packager: String,
    pub url: String,
    pub time: Time,
    pub size: Size,

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
            evr: version.clone(), // TODO
            checksum: checksum.clone(),
            location_href: location_href.to_owned(),
            ..Package::default()
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Checksum {
    SHA224(String),
    SHA256(String),
    SHA384(String),
    SHA512(String),
    Unknown,
}

impl Default for Checksum {
    fn default() -> Self {
        Checksum::Unknown
    }
}

impl Checksum {
    pub fn try_create<N: AsRef<[u8]> + Sized>(
        checksum_type: N,
        checksum: N,
    ) -> Result<Self, MetadataError> {
        let bytes_to_str = |value| std::str::from_utf8(value).unwrap().to_owned();

        let checksum = match checksum_type.as_ref() {
            b"sha224" => Checksum::SHA224(bytes_to_str(checksum.as_ref())),
            b"sha256" => Checksum::SHA256(bytes_to_str(checksum.as_ref())),
            b"sha384" => Checksum::SHA384(bytes_to_str(checksum.as_ref())),
            b"sha512" => Checksum::SHA512(bytes_to_str(checksum.as_ref())),
            _ => {
                return Err(MetadataError::UnsupportedChecksumTypeError(bytes_to_str(
                    checksum_type.as_ref(),
                )))
            }
        };
        Ok(checksum)
    }

    pub fn to_values<'a>(&'a self) -> Result<(&str, &'a str), MetadataError> {
        let values = match self {
            Checksum::SHA224(c) => ("sha224", c.as_str()),
            Checksum::SHA256(c) => ("sha256", c.as_str()),
            Checksum::SHA384(c) => ("sha384", c.as_str()),
            Checksum::SHA512(c) => ("sha512", c.as_str()),
            Checksum::Unknown => panic!("Cannot take value of a checksum of unknown type"),
        };
        Ok(values)
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct EVR {
    pub epoch: String,
    pub version: String, // ver
    pub release: String, // rel
}

impl EVR {
    pub fn new(epoch: &str, version: &str, release: &str) -> EVR {
        EVR {
            epoch: epoch.to_owned(),
            version: version.to_owned(),
            release: release.to_owned(),
        }
    }

    pub fn values(&self) -> (&str, &str, &str) {
        (&self.epoch, &self.version, &self.release)
    }
}

#[derive(Debug, PartialEq, Default)]
pub struct Changelog {
    pub author: String,
    pub date: u64,
    pub description: String,
}

#[derive(Debug, PartialEq, Default)]
pub struct Time {
    pub file: u64,
    pub build: u64,
}

#[derive(Debug, PartialEq, Default)]
pub struct Size {
    pub package: u64,
    pub installed: u64,
    pub archive: u64,
}

#[derive(Debug, PartialEq, Default)]
pub struct HeaderRange {
    pub start: u64,
    pub end: u64,
}

// Requirement (Provides, Conflicts, Obsoletes, Requires).
#[derive(Debug, PartialEq, Default)]
pub struct Requirement {
    pub name: String,
    pub flags: Option<String>,
    pub epoch: Option<String>,
    pub version: Option<String>,
    pub release: Option<String>,
    pub preinstall: Option<bool>,
}

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

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq, Default)]
pub struct PackageFile {
    pub filetype: FileType,
    pub path: String,
}

#[derive(Debug, PartialEq)]
pub enum MetadataType {
    Primary,
    Filelists,
    Other,

    PrimaryZck,
    FilelistsZck,
    OtherZck,

    PrimaryDb,
    FilelistsDb,
    OtherDb,

    Unknown,
}

impl From<&str> for MetadataType {
    fn from(name: &str) -> Self {
        match name {
            METADATA_PRIMARY => MetadataType::Primary,
            METADATA_FILELISTS => MetadataType::Filelists,
            METADATA_OTHER => MetadataType::Other,

            METADATA_PRIMARY_DB => MetadataType::PrimaryDb,
            METADATA_FILELISTS_DB => MetadataType::FilelistsDb,
            METADATA_OTHER_DB => MetadataType::OtherDb,

            METADATA_PRIMARY_ZCK => MetadataType::PrimaryZck,
            METADATA_FILELISTS_ZCK => MetadataType::FilelistsZck,
            METADATA_OTHER_ZCK => MetadataType::OtherZck,

            _ => MetadataType::Unknown,
        }
    }
}

#[derive(Debug, PartialEq, Default)]
pub struct DistroTag {
    pub cpeid: Option<String>,
    pub name: String,
}

impl DistroTag {
    pub fn new(name: String, cpeid: Option<String>) -> Self {
        DistroTag { name, cpeid }
    }
}

#[derive(Debug, PartialEq)]
pub struct RepoMdRecord {
    // TODO: location real? location base?  https://github.com/rpm-software-management/createrepo_c/commit/7e4ba3de1e9792f9d65f68c0d1cb18ed14ce1b68#diff-26e7fd2fdd746961fa628b1e9e42175640ec8d269c17e1608628d3377e0c07d4R371
    /// Record type
    pub mdtype: String,
    /// Relative location of the file in a repository
    pub location_href: String,
    /// Mtime of the file
    pub timestamp: u64,
    /// Size of the file
    pub size: u64,
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
