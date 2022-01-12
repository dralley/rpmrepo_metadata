use std::fs::OpenOptions;
use std::io::{Cursor, Read, Seek, SeekFrom};

use pretty_assertions::assert_eq;
use tempdir::TempDir;

use rpmrepo_metadata::*;

mod common;

static EMPTY_OTHERDATA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<otherdata xmlns="http://linux.duke.edu/metadata/other" packages="0">
</otherdata>
"#;

static EMPTY_OTHERDATA_NO_FOOTER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<otherdata xmlns="http://linux.duke.edu/metadata/other" packages="0">"#;

static EMPTY_OTHERDATA_NO_DECL: &str = r#"<otherdata xmlns="http://linux.duke.edu/metadata/other" packages="0">
</otherdata>
"#;

static COMPLEX_OTHERDATA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<otherdata xmlns="http://linux.duke.edu/metadata/other" packages="1">
  <package pkgid="bbb7b0e9350a0f75b923bdd0ef4f9af39765c668a3e70bfd3486ea9f0f618aaf" name="complex-package" arch="x86_64">
    <version epoch="1" ver="2.3.4" rel="5.el8"/>
    <changelog author="Lucille Bluth &lt;lucille@bluthcompany.com&gt; - 1.1.1-1" date="1617192000">- It's a banana, Michael. How much could it cost, $10?</changelog>
    <changelog author="Job Bluth &lt;job@alliance-of-magicians.com&gt; - 2.2.2-2" date="1619352000">- I've made a huge mistake</changelog>
    <changelog author="George Bluth &lt;george@federalprison.gov&gt; - 3.3.3-3" date="1623672000">- There’s always money in the banana stand</changelog>
  </package>
</otherdata>
"#;

#[test]
fn test_other_xml_writer_empty() -> Result<(), MetadataError> {
    let mut writer = OtherXml::new_writer(utils::create_xml_writer(Cursor::new(Vec::new())));

    writer.write_header(0)?;
    writer.finish()?;

    let buffer = writer.into_inner().into_inner();

    let actual = std::str::from_utf8(&buffer)?;
    let expected = EMPTY_OTHERDATA;

    assert_eq!(&actual, &expected);

    Ok(())
}

#[test]
fn test_other_xml_writer_complex_pkg() -> Result<(), MetadataError> {
    let mut writer = OtherXml::new_writer(utils::create_xml_writer(Cursor::new(Vec::new())));

    writer.write_header(1)?;
    writer.write_package(&common::COMPLEX_PACKAGE)?;
    writer.finish()?;

    let buffer = writer.into_inner().into_inner();

    let actual = std::str::from_utf8(&buffer)?;
    let expected = COMPLEX_OTHERDATA;

    assert_eq!(&actual, &expected);

    Ok(())
}

#[test]
fn test_other_xml_writer_file() -> Result<(), MetadataError> {
    let working_dir = TempDir::new("")?;
    let other_name = working_dir.path().join("other.xml");

    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(other_name)
        .unwrap();
    let mut writer = OtherXml::new_writer(utils::create_xml_writer(f));

    writer.write_header(0).unwrap();
    // writer.write_package(&common::RPM_EMPTY).unwrap();
    writer.finish()?;

    let mut f = writer.into_inner();

    f.seek(SeekFrom::Start(0))?;
    let mut actual = String::new();

    f.read_to_string(&mut actual).unwrap();

    assert_eq!(actual, EMPTY_OTHERDATA);

    Ok(())
}

#[test]
fn test_filelists_xml_read_header() -> Result<(), MetadataError> {
    // Test that the header parses correctly when there are no packages
    let mut other_xml = OtherXml::new_reader(utils::create_xml_reader(EMPTY_OTHERDATA.as_bytes()));
    assert_eq!(other_xml.read_header()?, 0);
    assert!(matches!(
        other_xml.read_header(),
        Err(MetadataError::MissingHeaderError)
    ));

    // Test that the header parses correctly when there are no packages and the footer element doesn't exist (EOF)
    let mut other_xml = OtherXml::new_reader(utils::create_xml_reader(
        EMPTY_OTHERDATA_NO_FOOTER.as_bytes(),
    ));
    assert_eq!(other_xml.read_header()?, 0);
    assert!(matches!(
        other_xml.read_header(),
        Err(MetadataError::MissingHeaderError)
    ));

    // Test that the header parses correctly when there is no XML declaration at the top
    let mut other_xml =
        OtherXml::new_reader(utils::create_xml_reader(EMPTY_OTHERDATA_NO_DECL.as_bytes()));
    assert_eq!(other_xml.read_header()?, 0);
    assert!(matches!(
        other_xml.read_header(),
        Err(MetadataError::MissingHeaderError)
    ));

    // Test that the header parses correctly when there is packages
    let mut other_xml =
        OtherXml::new_reader(utils::create_xml_reader(COMPLEX_OTHERDATA.as_bytes()));
    assert_eq!(other_xml.read_header()?, 1);
    assert!(matches!(
        other_xml.read_header(),
        Err(MetadataError::MissingHeaderError)
    ));

    Ok(())
}

#[test]
fn test_filelists_xml_read_package() -> Result<(), MetadataError> {
    // Test that no package is returned if the xml has no packages
    let mut other_xml = OtherXml::new_reader(utils::create_xml_reader(EMPTY_OTHERDATA.as_bytes()));
    assert_eq!(other_xml.read_header()?, 0);
    let mut package = None;
    other_xml.read_package(&mut package)?;
    assert!(matches!(package, None));

    // Test that no packaged is parsed when there are no packages and the footer element doesn't exist (EOF)
    let mut other_xml = OtherXml::new_reader(utils::create_xml_reader(
        EMPTY_OTHERDATA_NO_FOOTER.as_bytes(),
    ));
    assert_eq!(other_xml.read_header()?, 0);
    let mut package = None;
    other_xml.read_package(&mut package)?;
    assert!(matches!(package, None));

    // Test that a package is parsed correctly when there is packages
    let mut other_xml =
        OtherXml::new_reader(utils::create_xml_reader(COMPLEX_OTHERDATA.as_bytes()));
    assert_eq!(other_xml.read_header()?, 1);
    let mut package = None;
    other_xml.read_package(&mut package)?;
    assert!(matches!(package, Some(_)));
    package.take();
    other_xml.read_package(&mut package)?;
    assert!(matches!(package, None));

    Ok(())
}
