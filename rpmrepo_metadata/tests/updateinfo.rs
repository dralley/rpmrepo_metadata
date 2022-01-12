extern crate rpmrepo_metadata;

use pretty_assertions::assert_eq;
use rpmrepo_metadata::*;
use std::fs::{Metadata, OpenOptions};
use std::io::{Cursor, Read, Seek, SeekFrom};
use tempdir::TempDir;

mod common;

static EMPTY_UPDATEINFO: &str = r#""#;

static EMPTY_UPDATEINFO_NO_FOOTER: &str = r#""#;

static EMPTY_UPDATEINFO_NO_DECL: &str = r#""#;

static COMPLEX_UPDATEINFO: &str = r#""#;

#[test]
fn test_updateinfo_xml_writer_empty() -> Result<(), MetadataError> {
    // let mut writer = PrimaryXml::new_writer(utils::create_xml_writer(Cursor::new(Vec::new())));

    // writer.write_header(0)?;
    // writer.finish()?;

    // let buffer = writer.into_inner().into_inner();

    // let actual = std::str::from_utf8(&buffer)?;
    // let expected = EMPTY_PRIMARY;

    // assert_eq!(&actual, &expected);

    Ok(())
}

#[test]
fn test_updateinfo_xml_writer_complex_pkg() -> Result<(), MetadataError> {
    // let mut writer = PrimaryXml::new_writer(utils::create_xml_writer(Cursor::new(Vec::new())));

    // writer.write_header(1)?;
    // writer.write_package(&common::COMPLEX_PACKAGE)?;
    // writer.finish()?;

    // let buffer = writer.into_inner().into_inner();

    // let actual = std::str::from_utf8(&buffer)?;
    // let expected = COMPLEX_PRIMARY;

    // assert_eq!(&actual, &expected);

    Ok(())
}

#[test]
fn test_updateinfo_xml_writer_file() -> Result<(), MetadataError> {
    // let working_dir = TempDir::new("")?;
    // let other_name = working_dir.path().join("primary.xml");

    // let f = OpenOptions::new()
    //     .read(true)
    //     .write(true)
    //     .create(true)
    //     .open(other_name)
    //     .unwrap();

    // let mut writer = PrimaryXml::new_writer(utils::create_xml_writer(f));

    // writer.write_header(0).unwrap();
    // // writer.write_package(&common::RPM_EMPTY).unwrap();
    // writer.finish()?;

    // let mut f = writer.into_inner();

    // f.seek(SeekFrom::Start(0))?;
    // let mut actual = String::new();

    // f.read_to_string(&mut actual).unwrap();

    // assert_eq!(actual, EMPTY_PRIMARY);

    Ok(())
}
