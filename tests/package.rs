// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate rpmrepo_metadata;

use pretty_assertions::assert_eq;
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
