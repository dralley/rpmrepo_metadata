// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

mod common;
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
pub use metadata::{
    Checksum, ChecksumType, CompressionType, FileType, FilelistsXml, MetadataError, OtherXml,
    Package, PackageFile, PrimaryXml, RepomdData, RepomdRecord, RepomdXml, Requirement,
    UpdateCollection, UpdateCollectionModule, UpdateCollectionPackage, UpdateRecord,
    UpdateReference, UpdateinfoXml,
};
pub use package::PackageParser;
pub use repository::{Repository, RepositoryOptions, RepositoryReader, RepositoryWriter};
