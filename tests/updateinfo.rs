// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate rpmrepo_metadata;

use pretty_assertions::assert_eq;
use rpmrepo_metadata::UpdateRecord;
use rpmrepo_metadata::*;
use std::fs::{Metadata, OpenOptions};
use std::io::{Cursor, Read, Seek, SeekFrom};
use tempdir::TempDir;

mod common;

static EMPTY_UPDATEINFO: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<updates>
</updates>
"#;

static EMPTY_UPDATEINFO_NO_FOOTER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<updates>
"#;

static EMPTY_UPDATEINFO_NO_DECL: &str = r#"
<updates>
</updates>
"#;

static COMPLEX_UPDATEINFO: &str = r#""#;

#[test]
fn test_updateinfo_xml_writer_empty() -> Result<(), MetadataError> {
    let mut writer = UpdateinfoXml::new_writer(utils::create_xml_writer(Cursor::new(Vec::new())));

    writer.write_header()?;
    writer.finish()?;

    let buffer = writer.into_inner().into_inner();

    let actual = std::str::from_utf8(&buffer)?;
    let expected = EMPTY_UPDATEINFO;

    assert_eq!(&actual, &expected);

    Ok(())
}

// #[test]
// fn test_updateinfo_xml_writer_complex_pkg() -> Result<(), MetadataError> {
//     let mut writer = UpdateinfoXml::new_writer(utils::create_xml_writer(Cursor::new(Vec::new())));

//     writer.write_header()?;
//     writer.write_updaterecord(&common::UPDATERECORD)?;
//     writer.finish()?;

//     let buffer = writer.into_inner().into_inner();

//     let actual = std::str::from_utf8(&buffer)?;
//     let expected = COMPLEX_PRIMARY;

//     assert_eq!(&actual, &expected);

//     Ok(())
// }

#[test]
fn test_updateinfo_xml_writer_file() -> Result<(), MetadataError> {
    let working_dir = TempDir::new("")?;
    let other_name = working_dir.path().join("updateinfo.xml");

    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(other_name)
        .unwrap();

    let mut writer = UpdateinfoXml::new_writer(utils::create_xml_writer(f));

    writer.write_header().unwrap();
    // writer.write_package(&common::RPM_EMPTY).unwrap();
    writer.finish()?;

    let mut f = writer.into_inner();

    f.seek(SeekFrom::Start(0))?;
    let mut actual = String::new();

    f.read_to_string(&mut actual).unwrap();

    assert_eq!(actual, EMPTY_UPDATEINFO);

    Ok(())
}

// #[test]
// fn test_updateinfo_xml_read_header() -> Result<(), MetadataError> {
//     // Test that the header parses correctly when there are no packages
//     let mut updateinfo_xml =
//         UpdateinfoXml::new_reader(utils::create_xml_reader(EMPTY_UPDATEINFO.as_bytes()));
//     assert_eq!(updateinfo_xml.read_header()?, 0);
//     assert!(matches!(
//         updateinfo_xml.read_header(),
//         Err(MetadataError::MissingHeaderError)
//     ));

//     // Test that the header parses correctly when there are no packages and the footer element doesn't exist (EOF)
//     let mut updateinfo_xml =
//         UpdateinfoXml::new_reader(utils::create_xml_reader(EMPTY_UPDATEINFO_NO_FOOTER.as_bytes()));
//     assert_eq!(updateinfo_xml.read_header()?, 0);
//     assert!(matches!(
//         updateinfo_xml.read_header(),
//         Err(MetadataError::MissingHeaderError)
//     ));

//     // Test that the header parses correctly when there is no XML declaration at the top
//     let mut updateinfo_xml =
//         UpdateinfoXml::new_reader(utils::create_xml_reader(EMPTY_UPDATEINFO_NO_DECL.as_bytes()));
//     assert_eq!(updateinfo_xml.read_header()?, 0);
//     assert!(matches!(
//         updateinfo_xml.read_header(),
//         Err(MetadataError::MissingHeaderError)
//     ));

//     // Test that the header parses correctly when there is packages
//     let mut updateinfo_xml =
//         UpdateinfoXml::new_reader(utils::create_xml_reader(COMPLEX_UPDATEINFO.as_bytes()));
//     assert_eq!(updateinfo_xml.read_header()?, 1);
//     assert!(matches!(
//         updateinfo_xml.read_header(),
//         Err(MetadataError::MissingHeaderError)
//     ));

//     Ok(())
// }

#[test]
fn test_updateinfo_xml_read_updaterecord() -> Result<(), MetadataError> {
    // Test that no updaterecord is returned if the xml has no updaterecords
    let mut updateinfo_xml =
        UpdateinfoXml::new_reader(utils::create_xml_reader(EMPTY_UPDATEINFO.as_bytes()));
    // assert_eq!(updateinfo_xml.read_header()?, ());
    assert!(matches!(updateinfo_xml.read_update()?, None));

    // Test that no updaterecords are parsed when there are no packages and the footer element doesn't exist (EOF)
    let mut updateinfo_xml = UpdateinfoXml::new_reader(utils::create_xml_reader(
        EMPTY_UPDATEINFO_NO_FOOTER.as_bytes(),
    ));
    // assert_eq!(updateinfo_xml.read_header()?, ());
    assert!(matches!(updateinfo_xml.read_update()?, None));

    // // Test that an updaterecord is parsed correctly when there are updaterecords
    // let mut updateinfo_xml =
    //     UpdateinfoXml::new_reader(utils::create_xml_reader(COMPLEX_UPDATEINFO.as_bytes()));
    // // assert_eq!(updateinfo_xml.read_header()?, ());
    // assert!(matches!(updateinfo_xml.read_update()?, Some(_)));
    // assert!(matches!(updateinfo_xml.read_update()?, None));

    Ok(())
}
