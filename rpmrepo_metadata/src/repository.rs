use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Cursor, Write};
use std::path::Path;
use std::{collections::HashMap, path::PathBuf};

use quick_xml::{Reader, Writer};

use crate::updateinfo::UpdateinfoXmlWriter;
use crate::UpdateinfoXml;
use crate::{utils, PackageParser};

use super::filelist::FilelistsXmlWriter;
use super::metadata::{
    ChecksumType, CompressionType, DistroTag, FilelistsXml, MetadataType, OtherXml, Package,
    PrimaryXml, RepoMdData, RepoMdRecord, RepomdXml, RpmMetadata, UpdateRecord, METADATA_FILELISTS,
    METADATA_OTHER, METADATA_PRIMARY,
};
use super::other::OtherXmlWriter;
use super::primary::PrimaryXmlWriter;
use super::MetadataError;
use indexmap::IndexMap;

// TODO: uphold invariants
// a) no duplicate pkgids / checksums
// b) no duplicate NEVRA (normalized for epoch)
#[derive(Debug, PartialEq, Default)]
pub struct Repository {
    repomd_data: RepoMdData,
    packages: IndexMap<String, Package>,
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

    pub fn packages(&self) -> &IndexMap<String, Package> {
        &self.packages
    }

    // TODO: better API for package access (entry-like)
    pub fn packages_mut(&mut self) -> &mut IndexMap<String, Package> {
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

    pub fn load_metadata_file<M: RpmMetadata>(&mut self, path: &Path) -> Result<(), MetadataError> {
        let mut reader = utils::xml_reader_from_path(path)?;
        M::load_metadata(self, &mut reader)
    }

    pub fn load_metadata_str<M: RpmMetadata>(&mut self, str: &str) -> Result<(), MetadataError> {
        let mut reader = quick_xml::Reader::from_str(str);
        utils::configure_xml_reader(&mut reader);
        M::load_metadata(self, &mut reader)
    }

    pub fn load_metadata_bytes<M: RpmMetadata>(
        &mut self,
        bytes: &[u8],
    ) -> Result<(), MetadataError> {
        let (reader, _compression) = niffler::get_reader(Box::new(bytes))?;
        let mut reader = quick_xml::Reader::from_reader(BufReader::new(reader));
        utils::configure_xml_reader(&mut reader);

        M::load_metadata(self, &mut reader)
    }

    pub fn write_metadata_file<M: RpmMetadata>(
        &self,
        path: &Path,
        compression: CompressionType,
    ) -> Result<(), MetadataError> {
        let new_path = PathBuf::from(path);
        let new_path = new_path.join(M::filename());
        let (_, writer) = utils::create_xml_writer(&new_path, compression)?;
        M::write_metadata(self, writer)?;
        Ok(())
    }

    pub fn write_metadata_string<M: RpmMetadata>(&self) -> Result<String, MetadataError> {
        let bytes = self.write_metadata_bytes::<M>()?;
        Ok(String::from_utf8(bytes).map_err(|e| e.utf8_error())?)
    }

    pub fn write_metadata_bytes<M: RpmMetadata>(&self) -> Result<Vec<u8>, MetadataError> {
        let mut buf = Vec::new();
        let writer = quick_xml::Writer::new_with_indent(Cursor::new(&mut buf), b' ', 2);
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

    pub primary_xml_writer: Option<PrimaryXmlWriter<Box<dyn Write>>>,
    pub filelists_xml_writer: Option<FilelistsXmlWriter<Box<dyn Write>>>,
    pub other_xml_writer: Option<OtherXmlWriter<Box<dyn Write>>>,

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

        let (primary_path, primary_writer) = utils::create_xml_writer(
            &repodata_dir.join("primary.xml"),
            options.metadata_compression_type,
        )?;
        let (filelists_path, filelists_writer) = utils::create_xml_writer(
            &repodata_dir.join("filelists.xml"),
            options.metadata_compression_type,
        )?;
        let (other_path, other_writer) = utils::create_xml_writer(
            &repodata_dir.join("other.xml"),
            options.metadata_compression_type,
        )?;

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
            let (updateinfo_path, updateinfo_writer) = utils::create_xml_writer(
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
            utils::create_xml_writer(&repodata_dir.join("repomd.xml"), CompressionType::None)?;
        RepomdXml::write_data(&mut repomd_writer, &self.repomd_data)?;

        // TODO: a report of the files created?

        Ok(())
    }
}

pub struct RepositoryReader {
    repository: Repository, // TODO: we're only using this for the repomd, maybe just use it directly
    path: PathBuf,
}

impl RepositoryReader {
    pub fn new_from_directory(path: &Path) -> Result<Self, MetadataError> {
        let mut repo = Repository::new();
        repo.load_metadata_file::<RepomdXml>(&path.join("repodata/repomd.xml"))?;

        Ok(Self {
            repository: repo,
            path: path.to_owned(),
        })
    }

    pub fn iter_packages(&self) -> Result<PackageParser, MetadataError> {
        let primary_path = self.path.join(
            &self
                .repository
                .repomd()
                .get_record(METADATA_PRIMARY)
                .unwrap()
                .location_href,
        );
        let filelists_path = self.path.join(
            &self
                .repository
                .repomd()
                .get_record(METADATA_FILELISTS)
                .unwrap()
                .location_href,
        );
        let other_path = self.path.join(
            &self
                .repository
                .repomd()
                .get_record(METADATA_OTHER)
                .unwrap()
                .location_href,
        );

        PackageParser::from_files(&primary_path, &filelists_path, &other_path)
    }

    // pub fn iter_advisories(&self) -> Result<> {

    // }

    // pub fn iter_comps(&self) -> Result<> {

    // }

    pub fn into_repo(self) -> Repository {
        // TODO: load everything
        self.repository
    }
}
