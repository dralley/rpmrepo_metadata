// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::io::BufReader;
use std::path::Path;

use crate::filelist::FilelistsXmlReader;
use crate::metadata::{METADATA_FILELISTS, METADATA_OTHER, METADATA_PRIMARY};
use crate::other::OtherXmlReader;
use crate::primary::PrimaryXmlReader;
use crate::{utils, RepomdData};
use crate::{FilelistsXml, MetadataError, OtherXml, Package, PrimaryXml};

#[cfg(feature = "read_rpm")]
pub mod rpm_parsing {
    use super::*;
    use rpm::{self, Header};

    impl TryFrom<rpm::RPMPackage> for Package {
        type Error = rpm::RPMError;

        fn try_from(pkg: rpm::RPMPackage) -> Result<Package, Self::Error> {
            let mut pkg_metadata = Package::default();
            pkg_metadata.set_name(pkg.metadata.header.get_name()?);
            pkg_metadata.set_arch(pkg.metadata.header.get_arch()?);
            pkg_metadata.set_epoch(pkg.metadata.header.get_epoch()?);
            pkg_metadata.set_version(pkg.metadata.header.get_version()?);
            pkg_metadata.set_release(pkg.metadata.header.get_release()?);

            //     checksum: todo!(),
            //     location_href: todo!(),
            //     summary: todo!(),
            //     description: todo!(),
            //     packager: todo!(),
            //     url: todo!(),
            //     time: todo!(),
            //     size: todo!(),

            //     rpm_license: todo!(),
            //     rpm_vendor: todo!(),
            //     rpm_group: todo!(),
            //     rpm_buildhost: todo!(),
            //     rpm_sourcerpm: todo!(),
            //     rpm_header_range: todo!(),

            //     rpm_requires: todo!(),
            //     rpm_provides: todo!(),
            //     rpm_conflicts: todo!(),
            //     rpm_obsoletes: todo!(),
            //     rpm_suggests: todo!(),
            //     rpm_enhances: todo!(),
            //     rpm_recommends: todo!(),
            //     rpm_supplements: todo!(),

            //     rpm_changelogs: todo!(),
            //     rpm_files: todo!(),
            // };

            Ok(pkg_metadata)
        }
    }

    pub fn load_rpm_package(path: &Path) -> Result<Package, MetadataError> {
        let rpm_file = std::fs::File::open(path)?;
        let mut buf_reader = std::io::BufReader::new(rpm_file);
        let pkg = rpm::RPMPackage::parse(&mut buf_reader)?;

        Ok(Package::try_from(pkg)?)
    }
}

pub struct PackageIterator {
    primary_xml: PrimaryXmlReader<BufReader<Box<dyn std::io::Read + Send>>>,
    filelists_xml: FilelistsXmlReader<BufReader<Box<dyn std::io::Read + Send>>>,
    other_xml: OtherXmlReader<BufReader<Box<dyn std::io::Read + Send>>>,

    num_packages: usize,
    num_remaining: usize,
    in_progress_package: Option<Package>,
}

impl PackageIterator {
    pub fn from_repodata(base: &Path, repomd: &RepomdData) -> Result<Self, MetadataError> {
        let primary_path = base.join(&repomd.get_record(METADATA_PRIMARY).unwrap().location_href);
        let filelists_path =
            base.join(&repomd.get_record(METADATA_FILELISTS).unwrap().location_href);
        let other_path = base.join(&repomd.get_record(METADATA_OTHER).unwrap().location_href);
        Self::from_files(&primary_path, &filelists_path, &other_path)
    }

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
        let filelists_pkg_count = self.filelists_xml.read_header()?;
        let other_pkg_count = self.other_xml.read_header()?;

        if primary_pkg_count != filelists_pkg_count || primary_pkg_count != other_pkg_count {
            return Err(MetadataError::InconsistentMetadataError(
                "Metadata package counts don't match".to_owned(),
            ));
        }

        assert_eq!(primary_pkg_count, filelists_pkg_count);
        assert_eq!(primary_pkg_count, other_pkg_count);
        self.num_packages = primary_pkg_count;
        self.num_remaining = self.num_packages;

        Ok(())
    }

    pub fn parse_package(&mut self) -> Result<Option<Package>, MetadataError> {
        self.primary_xml
            .read_package(&mut self.in_progress_package)?;
        self.filelists_xml
            .read_package(&mut self.in_progress_package)?;
        self.other_xml.read_package(&mut self.in_progress_package)?;

        let package = self.in_progress_package.take();

        // TODO: re-enable this with actual error handling instead of panics - RHEL6 for example will fail
        // because the header lies about the number of packages
        if let Some(_) = package {
            self.num_remaining -= 1;
            // self.num_remaining = self
            //     .num_remaining
            //     .checked_sub(1)
            //     .expect("More packages parsed than declared in the metadata header.");
        } else {
            // assert!(
            //     self.num_remaining == 0,
            //     "Less packages parsed than declared in metadata header."
            // );
        }

        Ok(package)
    }

    pub fn remaining_packages(&self) -> usize {
        self.num_remaining
    }

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
