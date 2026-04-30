// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! A library for reading, writing, and managing RPM repository metadata.
//!
//! RPM repository metadata consists of several XML files (primary.xml, filelists.xml, other.xml,
//! repomd.xml, updateinfo.xml, comps.xml) that together describe the packages available in a
//! repository. This crate provides both high-level APIs ([`Repository`], [`RepositoryReader`],
//! [`RepositoryWriter`]) and lower-level streaming readers/writers for each metadata type.
//!
//! With the `read_rpm` feature enabled, RPM packages can be read directly from disk via
//! [`Package::from_file`].

mod common;
mod comps;
mod filelist;
mod metadata;
mod other;
mod package;
mod primary;
mod repomd;
mod repository;
mod updateinfo;
pub mod utils;

#[cfg(feature = "python_ext")]
mod python_ext;

pub use common::EVR;
pub use comps::CompsXmlReader;
pub use metadata::{
    Changelog, Checksum, ChecksumType, CompressionType, CompsCategory, CompsData, CompsEnvironment,
    CompsEnvironmentOption, CompsGroup, CompsLangpack, CompsPackageReq, CompsXml, FileType,
    FilelistsXml, MetadataError, OtherXml, Package, PackageFile, PrimaryXml, RepomdData,
    RepomdRecord, RepomdXml, Requirement, UpdateCollection, UpdateCollectionModule,
    UpdateCollectionPackage, UpdateRecord, UpdateReference, UpdateinfoXml,
};
pub use package::{PackageIterator, PackageOptions};
pub use repository::{
    Repository, RepositoryOptions, RepositoryReader, RepositoryWriter, UpdateinfoIterator,
};
pub use updateinfo::UpdateinfoXmlReader;
