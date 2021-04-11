use std::{fs::File, io::Read};
use std::io::Cursor;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::{collections::BTreeMap, path::PathBuf};

use quick_xml::{Reader, Writer};

use super::{metadata::{
    Compression, DistroTag, FilelistsXml, MetadataType, OtherXml, Package, PrimaryXml,
    RepoMdRecord, RepomdXml, RpmMetadata, UpdateRecord, METADATA_FILELISTS, METADATA_OTHER,
    METADATA_PRIMARY,
}, other};
use super::MetadataError;

fn configure_reader<R: BufRead>(reader: &mut Reader<R>) {
    reader.expand_empty_elements(true).trim_text(true);
}

#[derive(Debug, PartialEq, Default)]
pub struct RpmRepository {
    // TODO: super inefficient, fix this later. write benches first because the difference will probably be entertaining.
    pub packages: BTreeMap<String, Package>,

    pub revision: Option<String>,
    pub metadata_files: Vec<RepoMdRecord>,

    pub repo_tags: Vec<String>,
    pub content_tags: Vec<String>,
    pub distro_tags: Vec<DistroTag>,

    pub advisories: Vec<UpdateRecord>,
}

impl RpmRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_record(&mut self, record: RepoMdRecord) {
        self.metadata_files.push(record);
    }

    pub fn get_record(&self, rectype: &str) -> Option<&RepoMdRecord> {
        self.records().iter().find(|r| &r.mdtype == rectype)
    }

    pub fn records(&self) -> &Vec<RepoMdRecord> {
        &self.metadata_files
    }

    pub fn remove_record(&mut self, rectype: &str) {
        self.metadata_files.retain(|r| &r.mdtype != rectype);
    }

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

    pub fn sort_records(&mut self) {
        fn value(item: &RepoMdRecord) -> u32 {
            let mdtype = MetadataType::from(item.mdtype.as_str());
            match mdtype {
                MetadataType::Primary => 1,
                MetadataType::Filelists => 2,
                MetadataType::Other => 3,
                MetadataType::PrimaryDb => 4,
                MetadataType::FilelistsDb => 5,
                MetadataType::OtherDb => 6,
                MetadataType::PrimaryZck => 7,
                MetadataType::FilelistsZck => 8,
                MetadataType::OtherZck => 9,
                MetadataType::Unknown => 10,
            }
        }
        self.metadata_files.sort_by(|a, b| value(a).cmp(&value(b)));
    }

    pub fn get_primary_data(&self) -> &RepoMdRecord {
        self.get_record(METADATA_PRIMARY)
            .expect("Cannot find primary.xml")
    }

    pub fn get_filelist_data(&self) -> &RepoMdRecord {
        self.get_record(METADATA_FILELISTS)
            .expect("Cannot find filelists.xml")
    }

    pub fn get_other_data(&self) -> &RepoMdRecord {
        self.get_record(METADATA_OTHER)
            .expect("Cannot find other.xml")
    }

    // pub fn packages(&self) -> &[Package] {
    //     &self.packages
    // }

    pub fn from_directory(path: &Path) -> Result<Self, MetadataError> {
        let mut repo = RpmRepository::new();

        repo.load_metadata_file::<RepomdXml>(&path.join("repodata/repomd.xml"))?;

        let primary_href = path.join(
            repo.get_record(METADATA_PRIMARY)
                .unwrap()
                .location_href
                .as_str(),
        );
        let filelists_href = path.join(
            repo.get_record(METADATA_FILELISTS)
                .unwrap()
                .location_href
                .as_str(),
        );
        let other_href = path.join(
            repo.get_record(METADATA_OTHER)
                .unwrap()
                .location_href
                .as_str(),
        );

        repo.load_metadata_file::<PrimaryXml>(&primary_href)?;
        repo.load_metadata_file::<FilelistsXml>(&filelists_href)?;
        repo.load_metadata_file::<OtherXml>(&other_href)?;

        Ok(repo)
    }

    pub fn to_directory(&self, path: &Path) -> Result<(), MetadataError> {
        let repodata_dir = path.join("repodata");

        std::fs::create_dir_all(&repodata_dir)?;

        self.write_metadata_file::<RepomdXml>(&repodata_dir, Compression::None)?;
        self.write_metadata_file::<PrimaryXml>(&repodata_dir, Compression::None)?;
        self.write_metadata_file::<FilelistsXml>(&repodata_dir, Compression::None)?;
        self.write_metadata_file::<OtherXml>(&repodata_dir, Compression::None)?;

        Ok(())
    }

    pub fn from_files(
        primary_xml: &Path,
        filelists_xml: &Path,
        other_xml: &Path,
    ) -> Result<Self, MetadataError> {
        let mut repo = RpmRepository::new();
        repo.load_metadata_file::<PrimaryXml>(primary_xml)?;
        repo.load_metadata_file::<FilelistsXml>(filelists_xml)?;
        repo.load_metadata_file::<OtherXml>(other_xml)?;

        Ok(repo)
    }

    pub(crate) fn load_metadata_file<M: RpmMetadata>(
        &mut self,
        path: &Path,
    ) -> Result<(), MetadataError> {
        let file = File::open(path)?;
        let (reader, _compression) = niffler::get_reader(Box::new(&file))?;
        let mut reader = Reader::from_reader(BufReader::new(reader));
        configure_reader(&mut reader);

        M::load_metadata(self, &mut reader)
    }

    pub(crate) fn load_metadata_str<M: RpmMetadata>(
        &mut self,
        str: &str,
    ) -> Result<(), MetadataError> {
        let mut reader = Reader::from_str(str);
        configure_reader(&mut reader);

        M::load_metadata(self, &mut reader)
    }

    pub(crate) fn load_metadata_bytes<M: RpmMetadata>(
        &mut self,
        bytes: &[u8],
    ) -> Result<(), MetadataError> {
        let (reader, _compression) = niffler::get_reader(Box::new(bytes))?;
        let mut reader = Reader::from_reader(BufReader::new(reader));
        let mut reader = Reader::from_reader(bytes);
        configure_reader(&mut reader);

        M::load_metadata(self, &mut reader)
    }

    pub(crate) fn write_metadata_file<M: RpmMetadata>(
        &self,
        path: &Path,
        compression: Compression,
    ) -> Result<(), MetadataError> {
        let extension = match compression {
            Compression::None => "",
            Compression::Gzip => ".gz",
            Compression::Xz => ".xz",
            Compression::Bz2 => ".bz2",
        };

        let mut filename = PathBuf::from(M::NAME).as_os_str().to_owned();
        filename.push(&extension);
        let path = path.join(&filename);

        let file = File::create(path)?;

        let write_buffer = match compression {
            Compression::None => Box::new(file),
            Compression::Gzip => niffler::get_writer(
                Box::new(file),
                niffler::compression::Format::Gzip,
                niffler::Level::Nine,
            )?,
            Compression::Xz => niffler::get_writer(
                Box::new(file),
                niffler::compression::Format::Lzma,
                niffler::Level::Nine,
            )?,
            Compression::Bz2 => niffler::get_writer(
                Box::new(file),
                niffler::compression::Format::Bzip,
                niffler::Level::Nine,
            )?,
            _ => unimplemented!(),
        };
        let mut writer = Writer::new_with_indent(write_buffer, b' ', 2);
        M::write_metadata(self, &mut writer)?;
        Ok(())
    }

    pub(crate) fn to_string<M: RpmMetadata>(&self) -> Result<String, MetadataError> {
        let bytes = self.to_bytes::<M>()?;
        Ok(String::from_utf8(bytes).map_err(|e| e.utf8_error())?)
    }

    pub(crate) fn to_bytes<M: RpmMetadata>(&self) -> Result<Vec<u8>, MetadataError> {
        let mut buf = Vec::new();
        let mut writer = Writer::new_with_indent(Cursor::new(&mut buf), b' ', 2);
        M::write_metadata(self, &mut writer)?;
        Ok(writer.into_inner().into_inner().to_vec())
    }

    // TODO: allocation? one arena allocator per package, everything freed at once

    // TODO: what to do with updateinfo, groups, modules when packages added or removed?

    // configuration options for writing metadata:
    // * number of old packages?
    // * checksum types for metadata
    // * compression types. how customizable does it need to be?
    // * sqlite metadata yes/no
    // * zchunk metadata?
    // * signing
}


pub fn stream_from_directory(path: &Path) -> Result<PackageStreamer, MetadataError> {
    let mut repo = RpmRepository::new();

    repo.load_metadata_file::<RepomdXml>(&path.join("repodata/repomd.xml"))?;

    let primary_href = path.join(
        repo.get_record(METADATA_PRIMARY)
            .unwrap()
            .location_href
            .as_str(),
    );
    let filelists_href = path.join(
        repo.get_record(METADATA_FILELISTS)
            .unwrap()
            .location_href
            .as_str(),
    );
    let other_href = path.join(
        repo.get_record(METADATA_OTHER)
            .unwrap()
            .location_href
            .as_str(),
    );

    let primary_file = File::open(&primary_href)?;
    let (primary_file_reader, _compression) = niffler::get_reader(Box::new(primary_file))?;
    let mut primary_reader = Reader::from_reader(BufReader::new(primary_file_reader));
    configure_reader(&mut primary_reader);

    let filelists_file = File::open(&filelists_href)?;
    let (filelists_file_reader, _compression) = niffler::get_reader(Box::new(filelists_file))?;
    let mut filelists_reader = Reader::from_reader(BufReader::new(filelists_file_reader));
    configure_reader(&mut filelists_reader);

    let other_file = File::open(&other_href)?;
    let (other_file_reader, _compression) = niffler::get_reader(Box::new(other_file))?;
    let mut other_reader = Reader::from_reader(BufReader::new(other_file_reader));
    configure_reader(&mut other_reader);

    Ok(PackageStreamer { primary_reader, filelists_reader, other_reader })
}


pub struct PackageStreamer {
    primary_reader: Reader<BufReader<Box<dyn Read>>>,
    filelists_reader: Reader<BufReader<Box<dyn Read>>>,
    other_reader: Reader<BufReader<Box<dyn Read>>>,
}

impl Iterator for PackageStreamer {
    type Item = Result<Package, MetadataError>;
    fn next(&mut self) -> Option<Self::Item> {
        let mut package = Package::default();

        // super::primary::read_package(package, self.primary_reader)?;
        // FilelistsXml::read_into_package(package, self.filelists_reader)?;
        // OtherXml::read_into_package(package, self.other_reader)?;

        Some(Ok(package))
    }
}
