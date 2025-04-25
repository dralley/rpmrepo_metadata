// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate rpmrepo_metadata;

use pretty_assertions::assert_eq;
use rpm::{FileMode, FileOptions, FileOptionsBuilder};
use rpmrepo_metadata::*;
use std::fs::OpenOptions;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::Path;
use tempdir::TempDir;

mod common;

pub const COMPLEX_PKG_PATH: &str = "./tests/assets/packages/complex-package-2.3.4-5.el8.x86_64.rpm";

#[test]
fn test_read_rpm_from_file() -> Result<(), MetadataError> {
    let mut pkg = utils::load_rpm_package(COMPLEX_PKG_PATH)?;
    pkg.location_href = "complex-package-2.3.4-5.el8.x86_64.rpm".to_owned();
    assert_eq!(&pkg, &*common::COMPLEX_PACKAGE);

    Ok(())
}

#[test]
fn test_parse_rpm_with_symbolic_link() -> Result<(), MetadataError> {
    let rpm_package = rpm::PackageBuilder::new("foo", "1.0.0", "MIT", "aarch64", "foo")
        .with_file(
            "./tests/assets/complex_repo_pkglist.txt",
            FileOptions::new("/foo.txt")
                .symlink("/bar.txt")
                .mode(FileMode::SymbolicLink {
                    permissions: 0o0700,
                }),
        )?
        .build()?;
    let tmp_dir = tempdir::TempDir::new("test_parse_rpm_with_symbolic_link")?;

    let out = tmp_dir.path().join("out.rpm");

    rpm_package.write_file(&out)?;

    utils::load_rpm_package(&out)?;

    Ok(())
}
