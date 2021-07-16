use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Cursor, Write};
use std::path::Path;
use std::{collections::BTreeMap, path::PathBuf};

use quick_xml::{Reader, Writer};

use crate::updateinfo::UpdateinfoXmlWriter;
use crate::UpdateinfoXml;

use super::filelist::FilelistsXmlWriter;
use super::metadata::{
    ChecksumType, CompressionType, DistroTag, FilelistsXml, MetadataType, OtherXml, Package,
    PrimaryXml, RepoMdData, RepoMdRecord, RepomdXml, RpmMetadata, UpdateRecord, METADATA_FILELISTS,
    METADATA_OTHER, METADATA_PRIMARY,
};
use super::other::OtherXmlWriter;
use super::primary::PrimaryXmlWriter;
use super::MetadataError;

fn configure_reader<R: BufRead>(reader: &mut Reader<R>) {
    reader.expand_empty_elements(true).trim_text(true);
}

// TODO: uphold invariants
// a) no duplicate pkgids / checksums
// b) no duplicate NEVRA (normalized for epoch)
#[derive(Debug, PartialEq, Default)]
pub struct Repository {
    repomd_data: RepoMdData,
    packages: BTreeMap<String, Package>,
    advisories: Vec<UpdateRecord>,
}

impl Repository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn repomd<'repo>(&'repo self) -> &'repo RepoMdData {
        &self.repomd_data
    }

    pub fn repomd_mut<'repo>(&'repo mut self) -> &'repo mut RepoMdData {
        &mut self.repomd_data
    }

    pub fn packages(&self) -> &BTreeMap<String, Package> {
        &self.packages
    }

    // TODO: better API for package access (entry-like)
    pub fn packages_mut(&mut self) -> &mut BTreeMap<String, Package> {
        &mut self.packages
    }

    pub fn advisories(&self) -> &Vec<UpdateRecord> {
        &self.advisories
    }

    // TODO: better API for package access (entry-like)
    pub fn advisories_mut(&mut self) -> &mut Vec<UpdateRecord> {
        &mut self.advisories
    }

    pub fn load_from_directory(path: &Path) -> Result<Self, MetadataError> {
        let mut repo = Repository::new();

        repo.load_metadata_file::<RepomdXml>(&path.join("repodata/repomd.xml"))?;

        let primary_href = path.join(
            &repo
                .repomd()
                .get_record(METADATA_PRIMARY)
                .unwrap()
                .location_href,
        );
        let filelists_href = path.join(
            &repo
                .repomd()
                .get_record(METADATA_FILELISTS)
                .unwrap()
                .location_href,
        );
        let other_href = path.join(
            &repo
                .repomd()
                .get_record(METADATA_OTHER)
                .unwrap()
                .location_href,
        );

        repo.load_metadata_file::<PrimaryXml>(&primary_href)?;
        repo.load_metadata_file::<FilelistsXml>(&filelists_href)?;
        repo.load_metadata_file::<OtherXml>(&other_href)?;

        Ok(repo)
    }

    pub fn write_to_directory(
        &self,
        path: &Path,
        options: RepositoryOptions,
    ) -> Result<(), MetadataError> {
        let mut repo_writer =
            RepositoryWriter::new_with_options(&path, self.packages().len(), options)?;
        for (_, pkg) in self.packages() {
            repo_writer.add_package(pkg)?;
        }
        repo_writer.finish()?;

        Ok(())
    }

    pub fn load_from_files(
        primary_xml: &Path,
        filelists_xml: &Path,
        other_xml: &Path,
    ) -> Result<Self, MetadataError> {
        let mut repo = Repository::new();

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

    pub fn load_metadata_str<M: RpmMetadata>(
        &mut self,
        str: &str,
    ) -> Result<(), MetadataError> {
        let mut reader = Reader::from_str(str);
        configure_reader(&mut reader);

        M::load_metadata(self, &mut reader)
    }

    pub fn load_metadata_bytes<M: RpmMetadata>(
        &mut self,
        bytes: &[u8],
    ) -> Result<(), MetadataError> {
        let (reader, _compression) = niffler::get_reader(Box::new(bytes))?;
        let reader = Reader::from_reader(BufReader::new(reader));
        let mut reader = Reader::from_reader(bytes);
        configure_reader(&mut reader);

        M::load_metadata(self, &mut reader)
    }

    pub fn write_metadata_file<M: RpmMetadata>(
        &self,
        path: &Path,
        compression: CompressionType,
    ) -> Result<(), MetadataError> {
        let new_path = PathBuf::from(path);
        let new_path = new_path.join(M::filename());
        let (_, writer) = create_xml_writer(&new_path, compression)?;
        M::write_metadata(self, writer)?;
        Ok(())
    }

    pub fn to_string<M: RpmMetadata>(&self) -> Result<String, MetadataError> {
        let bytes = self.to_bytes::<M>()?;
        Ok(String::from_utf8(bytes).map_err(|e| e.utf8_error())?)
    }

    pub fn to_bytes<M: RpmMetadata>(&self) -> Result<Vec<u8>, MetadataError> {
        let mut buf = Vec::new();
        let writer = Writer::new_with_indent(Cursor::new(&mut buf), b' ', 2);
        M::write_metadata(self, writer)?;
        Ok(buf)
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

// TODO: maybe split this up so that it just configures the writer, but takes a Box<dyn Write> which can be pre-configured with compression
fn create_xml_writer(
    path: &Path,
    compression: CompressionType,
) -> Result<(PathBuf, Writer<Box<dyn Write>>), MetadataError> {
    let extension = compression.to_file_extension();

    // TODO: easier way to do this?
    let mut filename = path.as_os_str().to_owned();
    filename.push(&extension);
    let filename = PathBuf::from(&filename);

    let file = BufWriter::new(File::create(&filename)?);

    let inner_writer = match compression {
        CompressionType::None => Box::new(file),
        CompressionType::Gzip => niffler::get_writer(
            Box::new(file),
            niffler::compression::Format::Gzip,
            niffler::Level::Nine,
        )?,
        CompressionType::Bz2 => niffler::get_writer(
            Box::new(file),
            niffler::compression::Format::Bzip,
            niffler::Level::Nine,
        )?,
        CompressionType::Xz => niffler::get_writer(
            Box::new(file),
            niffler::compression::Format::Lzma,
            niffler::Level::Nine,
        )?,
        _ => unimplemented!(),
    };
    let writer = Writer::new_with_indent(inner_writer, b' ', 2);
    Ok((filename, writer))
}

#[derive(Debug, Copy, Clone)]
pub struct RepositoryOptions {
    simple_metadata_filenames: bool,
    metadata_compression_type: CompressionType,
    metadata_checksum_type: ChecksumType,
    package_checksum_type: ChecksumType,
}

impl Default for RepositoryOptions {
    fn default() -> Self {
        Self {
            simple_metadata_filenames: false,
            metadata_compression_type: CompressionType::Gzip,
            metadata_checksum_type: ChecksumType::Sha256,
            package_checksum_type: ChecksumType::Sha256,
        }
    }
}

impl RepositoryOptions {
    pub fn package_checksum_type(self, chktype: ChecksumType) -> Self {
        Self {
            package_checksum_type: chktype,
            ..self
        }
    }

    pub fn metadata_checksum_type(self, chktype: ChecksumType) -> Self {
        Self {
            metadata_checksum_type: chktype,
            ..self
        }
    }

    pub fn metadata_compression_type(self, comptype: CompressionType) -> Self {
        Self {
            metadata_compression_type: comptype,
            ..self
        }
    }

    pub fn simple_metadata_filenames(self, val: bool) -> Self {
        Self {
            simple_metadata_filenames: val,
            ..self
        }
    }
}

pub struct RepositoryWriter {
    options: RepositoryOptions,
    path: PathBuf,

    // TODO: main_metadata_writer
    primary_xml_writer: Option<PrimaryXmlWriter<Box<dyn Write>>>,
    filelists_xml_writer: Option<FilelistsXmlWriter<Box<dyn Write>>>,
    other_xml_writer: Option<OtherXmlWriter<Box<dyn Write>>>,

    // TODO
    // sqlite_data_writer: Option<SqliteDataWriter>,
    repomd_data: RepoMdData,

    updateinfo_xml_writer: Option<UpdateinfoXmlWriter<Box<dyn Write>>>,
}

impl RepositoryWriter {
    pub fn new(path: &Path, num_pkgs: usize) -> Result<Self, MetadataError> {
        Self::new_with_options(path, num_pkgs, RepositoryOptions::default())
    }

    pub fn new_with_options(
        path: &Path,
        num_pkgs: usize,
        options: RepositoryOptions,
    ) -> Result<Self, MetadataError> {
        let repodata_dir = path.join("repodata");
        std::fs::create_dir_all(&repodata_dir)?;

        let (primary_path, primary_writer) = create_xml_writer(
            &repodata_dir.join("primary.xml"),
            options.metadata_compression_type,
        )?;
        let (filelists_path, filelists_writer) = create_xml_writer(
            &repodata_dir.join("filelists.xml"),
            options.metadata_compression_type,
        )?;
        let (other_path, other_writer) = create_xml_writer(
            &repodata_dir.join("other.xml"),
            options.metadata_compression_type,
        )?;

        // TODO: make sure this is buffered
        let mut primary_xml_writer = PrimaryXml::new_writer(primary_writer);
        let mut filelists_xml_writer = FilelistsXml::new_writer(filelists_writer);
        let mut other_xml_writer = OtherXml::new_writer(other_writer);

        primary_xml_writer.write_header(num_pkgs)?;
        filelists_xml_writer.write_header(num_pkgs)?;
        other_xml_writer.write_header(num_pkgs)?;

        Ok(Self {
            options,
            path: path.to_owned(),

            primary_xml_writer: Some(primary_xml_writer),
            filelists_xml_writer: Some(filelists_xml_writer),
            other_xml_writer: Some(other_xml_writer),

            repomd_data: RepoMdData::default(),

            updateinfo_xml_writer: None,
        })
    }

    pub fn repomd_mut(&mut self) -> &mut RepoMdData {
        &mut self.repomd_data
    }

    pub fn add_package(&mut self, pkg: &Package) -> Result<(), MetadataError> {
        self.primary_xml_writer
            .as_mut()
            .unwrap()
            .write_package(pkg)?;
        self.filelists_xml_writer
            .as_mut()
            .unwrap()
            .write_package(pkg)?;
        self.other_xml_writer.as_mut().unwrap().write_package(pkg)?;

        // TODO:
        // if self.sqlite_data_writer.is_none() {

        // }

        Ok(())
    }

    pub fn add_advisory(&mut self, record: &UpdateRecord) -> Result<(), MetadataError> {
        // TODO: clean this up
        if self.updateinfo_xml_writer.is_none() {
            let repodata_dir = self.path.join("repodata");
            let (updateinfo_path, updateinfo_writer) = create_xml_writer(
                &repodata_dir.join("updateinfo.xml"),
                self.options.metadata_compression_type,
            )?;

            let mut updateinfo_xml_writer = UpdateinfoXml::new_writer(updateinfo_writer);
            updateinfo_xml_writer.write_header()?;

            self.updateinfo_xml_writer = Some(updateinfo_xml_writer)
        }

        self.updateinfo_xml_writer
            .as_mut()
            .unwrap()
            .write_updaterecord(record)?;

        Ok(())
    }

    pub fn finish(&mut self) -> Result<(), MetadataError> {
        let extension = self.options.metadata_compression_type.to_file_extension();

        // TODO: this is a mess
        let repodata_dir = self.path.join("repodata");
        let mut primary_path = repodata_dir.join("primary.xml").as_os_str().to_owned();
        let mut filelists_path = repodata_dir.join("filelists.xml").as_os_str().to_owned();
        let mut other_path = repodata_dir.join("other.xml").as_os_str().to_owned();

        if !extension.is_empty() {
            primary_path.push(extension);
            filelists_path.push(extension);
            other_path.push(extension);
        }

        self.primary_xml_writer.as_mut().unwrap().finish()?;
        self.filelists_xml_writer.as_mut().unwrap().finish()?;
        self.other_xml_writer.as_mut().unwrap().finish()?;

        // TODO: maybe clean this up?
        // All of the ceremony, including making the fields in the struct optional, is required to
        // be able to drop() the writers, because the underlying encoders do not finish their work unless
        // dropped. The underlying compression encoders do have methods to finish encoding, however, we
        // do not have access to those because it's behind Box<dyn Read>.
        self.primary_xml_writer = None;
        self.filelists_xml_writer = None;
        self.other_xml_writer = None;

        self.repomd_mut()
            .add_record(RepoMdRecord::new("primary", &primary_path.as_ref())?);
        self.repomd_mut()
            .add_record(RepoMdRecord::new("filelists", &filelists_path.as_ref())?);
        self.repomd_mut()
            .add_record(RepoMdRecord::new("other", &other_path.as_ref())?);

        if let Some(updateinfo_xml_writer) = &mut self.updateinfo_xml_writer {
            updateinfo_xml_writer.finish()?;
            self.updateinfo_xml_writer = None;
        }

        let (_, mut repomd_writer) =
            create_xml_writer(&repodata_dir.join("repomd.xml"), CompressionType::None)?;
        RepomdXml::write_data(&mut repomd_writer, &self.repomd_data)?;

        // TODO: a report of the files created?

        Ok(())
    }
}

pub struct RepositoryReader {
    // primary_reader: Reader<BufReader<Box<dyn Read>>>,
    // filelists_reader: Reader<BufReader<Box<dyn Read>>>,
    // other_reader: Reader<BufReader<Box<dyn Read>>>,
    repository: Repository,
}

// impl Iterator for StreamingReader {
//     type Item = Result<Package, MetadataError>;
//     fn next(&mut self) -> Option<Self::Item> {
//         let mut package = Package::default();

//         // super::primary::read_package(package, self.primary_reader)?;
//         // FilelistsXml::read_into_package(package, self.filelists_reader)?;
//         // OtherXml::read_into_package(package, self.other_reader)?;

//         Some(Ok(package))
//     }
// }

impl RepositoryReader {
    pub fn new_from_directory(path: &Path) -> Result<Self, MetadataError> {
        let mut repo = Repository::new();

        repo.load_metadata_file::<RepomdXml>(&path.join("repodata/repomd.xml"))?;

        let primary_href = path.join(
            &repo
                .repomd()
                .get_record(METADATA_PRIMARY)
                .unwrap()
                .location_href,
        );
        let filelists_href = path.join(
            &repo
                .repomd()
                .get_record(METADATA_FILELISTS)
                .unwrap()
                .location_href,
        );
        let other_href = path.join(
            &repo
                .repomd()
                .get_record(METADATA_OTHER)
                .unwrap()
                .location_href,
        );

        // let primary_file = File::open(&primary_href)?;
        // let (primary_file_reader, _compression) = niffler::get_reader(Box::new(primary_file))?;
        // let mut primary_reader = Reader::from_reader(BufReader::new(primary_file_reader));
        // configure_reader(&mut primary_reader);

        // let filelists_file = File::open(&filelists_href)?;
        // let (filelists_file_reader, _compression) = niffler::get_reader(Box::new(filelists_file))?;
        // let mut filelists_reader = Reader::from_reader(BufReader::new(filelists_file_reader));
        // configure_reader(&mut filelists_reader);

        // let other_file = File::open(&other_href)?;
        // let (other_file_reader, _compression) = niffler::get_reader(Box::new(other_file))?;
        // let mut other_reader = Reader::from_reader(BufReader::new(other_file_reader));
        // configure_reader(&mut other_reader);

        repo.load_metadata_file::<PrimaryXml>(&primary_href)?;
        repo.load_metadata_file::<FilelistsXml>(&filelists_href)?;
        repo.load_metadata_file::<OtherXml>(&other_href)?;

        Ok(Self {
            // primary_reader,
            // filelists_reader,
            // other_reader,
            repository: repo,
        })
    }

    pub fn into_repo(self) -> Repository {
        self.repository
    }
}
