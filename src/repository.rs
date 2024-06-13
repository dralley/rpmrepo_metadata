// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use crate::updateinfo::{UpdateinfoXmlReader, UpdateinfoXmlWriter};
use crate::UpdateinfoXml;
use crate::{utils, PackageIterator};

use super::filelist::FilelistsXmlWriter;
use super::metadata::{
    ChecksumType,
    CompressionType,
    FilelistsXml,
    OtherXml,
    Package,
    PrimaryXml,
    RepomdData,
    RepomdRecord,
    RepomdXml,
    RpmMetadata,
    UpdateRecord, // DistroTag, MetadataType
};
use super::other::OtherXmlWriter;
use super::primary::PrimaryXmlWriter;
use super::MetadataError;
use indexmap::IndexMap;

/// A high level API for working with RPM repositories.
///
/// This struct attempts to uphold invariants such as
///  a) only one package of any given NEVRA (name-epoch-version-release-architecture) combination is permitted
///  b) updateinfo IDs are unique
///
/// Helpers are also provided for keeping packages ordered (helps with the metadata compression ratio).
///
/// All metadata is maintained in working memory (this can be large).
#[derive(Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Repository {
    repomd_data: RepomdData,
    packages: IndexMap<String, Package>,
    advisories: IndexMap<String, UpdateRecord>,
}

// TODO: worth doing any allocation tricks? (probably not)
// TODO: probably consolidate package_checksum_type and metadata_checksum_type, no real need for both
// TODO: provide a way to e.g. remove N old packages from the repo
// TODO: what to do with updateinfo, groups, modules when packages added or removed?
// TODO: uphold invariants
// a) no duplicate NEVRA (normalized for epoch)
// b) no duplicate advisory IDs
// TODO:

// configuration options for writing metadata:
// * checksum types for metadata
// * compression types. how customizable does it need to be?
// * zchunk metadata?
// * signing
impl Repository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn repomd<'repo>(&'repo self) -> &'repo RepomdData {
        &self.repomd_data
    }

    pub fn repomd_mut<'repo>(&'repo mut self) -> &'repo mut RepomdData {
        &mut self.repomd_data
    }

    pub fn packages(&self) -> &IndexMap<String, Package> {
        &self.packages
    }

    // TODO: better API for package access (entry-like)
    pub fn packages_mut(&mut self) -> &mut IndexMap<String, Package> {
        &mut self.packages
    }

    pub fn advisories(&self) -> &IndexMap<String, UpdateRecord> {
        &self.advisories
    }

    // TODO: better API for package access (entry-like)
    pub fn advisories_mut(&mut self) -> &mut IndexMap<String, UpdateRecord> {
        &mut self.advisories
    }

    /// Sorts the package entries by `location_href`.
    ///
    /// Helps with compression ratios for certain types of compression, and makes it more easily searchable.
    pub fn sort(&mut self) {
        self.packages
            .sort_by(|_k1, v1, _k2, v2| v1.location_href().cmp(v2.location_href()));
    }

    /// Create a new [`Repository`] from a path pointing to an RPM repository.
    ///
    /// Will fail if the RPM repository is not valid.
    pub fn load_from_directory(path: &Path) -> Result<Self, MetadataError> {
        let reader = RepositoryReader::new_from_directory(path)?;
        Ok(reader.into_repo()?)
    }

    /// Load a metadata file into an existing repository.
    pub fn load_metadata_file<M: RpmMetadata>(&mut self, path: &Path) -> Result<(), MetadataError> {
        let reader = utils::xml_reader_from_file(path)?;
        M::load_metadata(self, reader)
    }

    /// Load metadata from a string into an existing repository.
    pub fn load_metadata_str<M: RpmMetadata>(&mut self, str: &str) -> Result<(), MetadataError> {
        let reader = utils::create_xml_reader(str.as_bytes());
        M::load_metadata(self, reader)
    }

    /// Load metadata from an array of bytes (assumed to be UTF-8) into an existing repository.
    pub fn load_metadata_bytes<M: RpmMetadata>(
        &mut self,
        bytes: &[u8],
    ) -> Result<(), MetadataError> {
        let reader = utils::create_xml_reader(bytes);
        M::load_metadata(self, reader)
    }

    /// Write all the RPM metadata out to a directory with default options.
    pub fn write_to_directory(&self, path: &Path) -> Result<(), MetadataError> {
        Self::write_to_directory_with_options(&self, path, RepositoryOptions::default())
    }

    /// Write all the RPM metadata out to a directory with the provided options.
    pub fn write_to_directory_with_options(
        &self,
        path: &Path,
        options: RepositoryOptions,
    ) -> Result<(), MetadataError> {
        let mut writer = RepositoryWriter::new_with_options(path, self.packages().len(), options)?;

        for (_, pkg) in self.packages() {
            writer.add_package(pkg)?;
        }
        for (_, advisory) in self.advisories() {
            writer.add_advisory(advisory)?;
        }

        writer.finish()?;

        Ok(())
    }

    /// Write an individual metadata file to disk.
    pub fn write_metadata_file<M: RpmMetadata>(
        &self,
        path: &Path,
        compression: CompressionType,
    ) -> Result<PathBuf, MetadataError> {
        let new_path = PathBuf::from(path);
        let new_path = new_path.join(M::filename());
        let (fname, writer) = utils::xml_writer_for_path(&new_path, compression)?;
        M::write_metadata(self, writer)?;
        Ok(fname)
    }

    /// Write repository metadata to a String.
    pub fn write_metadata_string<M: RpmMetadata>(&self) -> Result<String, MetadataError> {
        let bytes = self.write_metadata_bytes::<M>()?;
        Ok(String::from_utf8(bytes).map_err(|e| e.utf8_error())?)
    }

    /// Write repository metadata to a buffer of bytes.
    pub fn write_metadata_bytes<M: RpmMetadata>(&self) -> Result<Vec<u8>, MetadataError> {
        let mut buf = Vec::new();
        let writer = utils::create_xml_writer(&mut buf);
        M::write_metadata(self, writer)?;
        Ok(buf)
    }
}

/// Options for writing RPM repository metadata.
///
/// - `simple_metadata_filenames` - Determines whether filenames should be bare e.g. `filelists.xml` or should include the file checksum.
/// - `metadata_compression_type` - The type of compression to use for repository metadata.
/// - `metadata_checksum_type` - The type of checksums to use for metadata.
/// - `package_checksum_type` - The type of checksums to use for packages.
#[derive(Copy, Clone, Debug)]
pub struct RepositoryOptions {
    pub simple_metadata_filenames: bool,
    pub metadata_compression_type: CompressionType,
    pub metadata_checksum_type: ChecksumType,
    pub package_checksum_type: ChecksumType,
}

impl Default for RepositoryOptions {
    fn default() -> Self {
        Self {
            simple_metadata_filenames: false,
            metadata_compression_type: CompressionType::Zstd,
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

/// Helper for writing RPM repository metadata manually.
///
/// A complete RPM repository can represent a significant amount of metadata split across multiple files.
/// This API provides a way to write different types of metadata separately and without needing to keep
/// everything in memory by storing it in a [`Repository`] first.
pub struct RepositoryWriter {
    options: RepositoryOptions,
    path: PathBuf,

    primary_xml_writer: Option<PrimaryXmlWriter<Box<dyn Write + Send>>>,
    filelists_xml_writer: Option<FilelistsXmlWriter<Box<dyn Write + Send>>>,
    other_xml_writer: Option<OtherXmlWriter<Box<dyn Write + Send>>>,
    updateinfo_xml_writer: Option<UpdateinfoXmlWriter<Box<dyn Write + Send>>>,

    num_pkgs_written: usize,
    num_pkgs: usize,

    repomd_data: RepomdData,
}

impl RepositoryWriter {
    /// Constructor for a new [`RepositoryWriter`] with default options. See [`RepositoryOptions`].
    pub fn new(path: &Path, num_pkgs: usize) -> Result<Self, MetadataError> {
        Self::new_with_options(path, num_pkgs, RepositoryOptions::default())
    }

    /// Constructor for a new [`RepositoryWriter`] with user-provided options. See [`RepositoryOptions`].
    pub fn new_with_options(
        path: &Path,
        num_pkgs: usize,
        options: RepositoryOptions,
    ) -> Result<Self, MetadataError> {
        let repodata_dir = path.join("repodata");
        std::fs::create_dir_all(&repodata_dir)?;

        let (_primary_path, primary_writer) = utils::xml_writer_for_path(
            &repodata_dir.join("primary.xml"),
            options.metadata_compression_type,
        )?;
        let (_filelists_path, filelists_writer) = utils::xml_writer_for_path(
            &repodata_dir.join("filelists.xml"),
            options.metadata_compression_type,
        )?;
        let (_other_path, other_writer) = utils::xml_writer_for_path(
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
            updateinfo_xml_writer: None,

            num_pkgs: num_pkgs,
            num_pkgs_written: 0,

            repomd_data: RepomdData::default(),
        })
    }

    /// Mutable accessor for the [`RepomdData`] struct which is written as repomd.xml later.
    pub fn repomd_mut(&mut self) -> &mut RepomdData {
        &mut self.repomd_data
    }

    /// Write a `Package` to the repo metadata.
    pub fn add_package(&mut self, pkg: &Package) -> Result<(), MetadataError> {
        self.num_pkgs_written += 1;
        assert!(
            self.num_pkgs_written <= self.num_pkgs,
            "Num packages written {} is more than number of packages declared in the header {}",
            self.num_pkgs_written,
            self.num_pkgs
        );

        self.primary_xml_writer
            .as_mut()
            .unwrap()
            .write_package(pkg)?;
        self.filelists_xml_writer
            .as_mut()
            .unwrap()
            .write_package(pkg)?;
        self.other_xml_writer.as_mut().unwrap().write_package(pkg)?;

        Ok(())
    }

    /// Write an `UpdateRecord` to the repo metadata.
    pub fn add_advisory(&mut self, record: &UpdateRecord) -> Result<(), MetadataError> {
        // TODO: clean this up
        if self.updateinfo_xml_writer.is_none() {
            let repodata_dir = self.path.join("repodata");
            let (updateinfo_path, updateinfo_writer) = utils::xml_writer_for_path(
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

    /// Consume the [`RepositoryWriter`], and finish writing the repository metadata to disk.
    ///
    /// - Checks that the number of packages written matches the number of packages declared.
    /// - Completes all metadata files.
    /// - Writes `repomd.xml`.
    pub fn finish(mut self) -> Result<(), MetadataError> {
        assert_eq!(
            self.num_pkgs_written, self.num_pkgs,
            "Number of packages written {} is different from the number declared in the header {}.",
            self.num_pkgs_written, self.num_pkgs
        );

        // TODO: this is a mess
        let path = self.path.clone();
        let repodata_dir = self.path.join("repodata");
        let primary_path = utils::apply_compression_suffix(
            &PathBuf::from("repodata").join("primary.xml"),
            self.options.metadata_compression_type,
        );
        let filelists_path = utils::apply_compression_suffix(
            &PathBuf::from("repodata").join("filelists.xml"),
            self.options.metadata_compression_type,
        );
        let other_path = utils::apply_compression_suffix(
            &PathBuf::from("repodata").join("other.xml"),
            self.options.metadata_compression_type,
        );

        self.primary_xml_writer.as_mut().unwrap().finish()?;
        self.filelists_xml_writer.as_mut().unwrap().finish()?;
        self.other_xml_writer.as_mut().unwrap().finish()?;

        // TODO: maybe clean this up?
        // All of the ceremony, including making the fields in the struct optional, is required to
        // be able to drop() the writers, because the underlying encoders do not finish their work unless
        // dropped. The underlying compression encoders do have methods to finish encoding, however, we
        // do not have access to those because it's behind Box<dyn Read>.
        drop(self.primary_xml_writer.take());
        drop(self.filelists_xml_writer.take());
        drop(self.other_xml_writer.take());

        let primary_xml = RepomdRecord::new(
            "primary",
            &primary_path.as_ref(),
            &path,
            self.options.metadata_checksum_type,
        )?;
        self.repomd_mut().add_record(primary_xml);
        let filelists_xml = RepomdRecord::new(
            "filelists",
            &filelists_path.as_ref(),
            &path,
            self.options.metadata_checksum_type,
        )?;
        self.repomd_mut().add_record(filelists_xml);
        let other_xml = RepomdRecord::new(
            "other",
            &other_path.as_ref(),
            &path,
            self.options.metadata_checksum_type,
        )?;
        self.repomd_mut().add_record(other_xml);

        if let Some(updateinfo_xml_writer) = &mut self.updateinfo_xml_writer {
            updateinfo_xml_writer.finish()?;
            self.updateinfo_xml_writer = None;
            let updateinfo_path = utils::apply_compression_suffix(
                &PathBuf::from("repodata").join("updateinfo.xml"),
                self.options.metadata_compression_type,
            );
            let updateinfo_xml = RepomdRecord::new(
                "updateinfo",
                &updateinfo_path.as_ref(),
                &path,
                self.options.metadata_checksum_type,
            )?;
            self.repomd_mut().add_record(updateinfo_xml);
        }

        let (_, mut repomd_writer) =
            utils::xml_writer_for_path(&repodata_dir.join("repomd.xml"), CompressionType::None)?;
        RepomdXml::write_data(&self.repomd_data, &mut repomd_writer)?;

        // TODO: a report of the files created?

        Ok(())
    }
}

/// Helper for reading metadata from an RPM repository manually.
///
/// A complete RPM repository can represent a significant amount of metadata split across multiple files.
/// This API provides a way to read different types of metadata without reading everything at once and
/// storing it in memory.
pub struct RepositoryReader {
    // TODO: we're only using this for the repomd, maybe just use it directly
    // but need to figure out how to generically support loading metadata files
    repository: Repository,
    path: PathBuf,
}

impl RepositoryReader {
    /// Create a new `RepositoryReader` for a given directory `path`.
    ///
    /// If `repodata/repomd.xml` cannot be found or if it cannot be parsed, this will fail.
    pub fn new_from_directory(path: &Path) -> Result<Self, MetadataError> {
        let mut repo = Repository::new();
        repo.load_metadata_file::<RepomdXml>(&path.join("repodata/repomd.xml"))?;

        Ok(Self {
            repository: repo,
            path: path.to_owned(),
        })
    }

    /// Return the contents of `repomd.xml` in a `RepomdData` struct.
    pub fn repomd(&self) -> &RepomdData {
        &self.repository.repomd()
    }

    /// Iterate over the packages of the repo.
    ///
    /// Create an iterator over the package metadata which will yield packages until completion or error.
    pub fn iter_packages(&self) -> Result<PackageIterator, MetadataError> {
        PackageIterator::from_repodata(&self.path, self.repository.repomd())
    }

    /// Iterate over the advisories of the repo.
    ///
    /// Create an iterator over "advisory" / updateinfo metadata which will yield updaterecords until completion or error.
    pub fn iter_advisories(&self) -> Result<UpdateinfoIterator, MetadataError> {
        UpdateinfoIterator::from_metadata(&self.path, self.repository.repomd())
    }

    // pub fn iter_comps(&self) -> Result<> {

    // }

    /// Consume the `RepositoryReader` and yield a [`Repository`] struct with the full repository contents.
    pub fn into_repo(mut self) -> Result<Repository, MetadataError> {
        let packages = self.iter_packages()?;
        self.repository
            .packages_mut()
            .reserve(packages.total_packages());

        for package in packages {
            let package = package?;
            self.repository
                .packages_mut()
                .insert(package.pkgid().to_owned(), package);
        }

        let advisories = self.iter_advisories()?;
        for advisory in advisories {
            let advisory = advisory?;
            self.repository
                .advisories_mut()
                .insert(advisory.id.to_owned(), advisory);
        }

        Ok(self.repository)
    }
}

pub struct UpdateinfoIterator {
    updateinfo: Option<UpdateinfoXmlReader<BufReader<Box<dyn std::io::Read + Send>>>>,
}

impl UpdateinfoIterator {
    fn from_metadata(base: &Path, repomd: &RepomdData) -> Result<Self, MetadataError> {
        let updateinfo_href = repomd
            .get_record(crate::metadata::METADATA_UPDATEINFO)
            .map(|u| base.join(&u.location_href));

        let reader = if let Some(updateinfo_href) = updateinfo_href {
            let reader = UpdateinfoXml::new_reader(utils::xml_reader_from_file(
                &base.join(updateinfo_href),
            )?);
            Some(reader)
        } else {
            None
        };

        Ok(Self { updateinfo: reader })
    }
}

impl Iterator for UpdateinfoIterator {
    type Item = Result<UpdateRecord, MetadataError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.updateinfo.as_mut()?.read_update().transpose()
    }
}
