extern crate rpmrepo_metadata;

use std::fs::OpenOptions;
use std::io::{Cursor, Read, Seek, SeekFrom};

use pretty_assertions::assert_eq;
use tempdir::TempDir;

use rpmrepo_metadata::*;

mod common;

static EMPTY_FILELISTS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<filelists xmlns="http://linux.duke.edu/metadata/filelists" packages="0">
</filelists>
"#;

static EMPTY_FILELISTS_NO_FOOTER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<filelists xmlns="http://linux.duke.edu/metadata/filelists" packages="0">"#;

static EMPTY_FILELISTS_NO_DECL: &str = r#"<filelists xmlns="http://linux.duke.edu/metadata/filelists" packages="0">
</filelists>
"#;

static COMPLEX_FILELISTS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<filelists xmlns="http://linux.duke.edu/metadata/filelists" packages="1">
  <package pkgid="bbb7b0e9350a0f75b923bdd0ef4f9af39765c668a3e70bfd3486ea9f0f618aaf" name="complex-package" arch="x86_64">
    <version epoch="1" ver="2.3.4" rel="5.el8"/>
    <file>/etc/complex/pkg.cfg</file>
    <file>/usr/bin/complex_a</file>
    <file type="dir">/usr/share/doc/complex-package</file>
    <file>/usr/share/doc/complex-package/README</file>
    <file type="dir">/var/lib/complex</file>
    <file type="ghost">/var/log/complex.log</file>
  </package>
</filelists>
"#;

#[test]
fn test_filelists_xml_writer_empty() -> Result<(), MetadataError> {
    let mut writer = FilelistsXml::new_writer(utils::create_xml_writer(Cursor::new(Vec::new())));

    writer.write_header(0)?;
    writer.finish()?;

    let buffer = writer.into_inner().into_inner();

    let actual = std::str::from_utf8(&buffer)?;
    let expected = EMPTY_FILELISTS;
    assert_eq!(&actual, &expected);

    Ok(())
}

#[test]
fn test_filelists_xml_writer_complex_pkg() -> Result<(), MetadataError> {
    let mut writer = FilelistsXml::new_writer(utils::create_xml_writer(Cursor::new(Vec::new())));

    writer.write_header(1)?;
    writer.write_package(&common::COMPLEX_PACKAGE)?;
    writer.finish()?;

    let buffer = writer.into_inner().into_inner();

    let actual = std::str::from_utf8(&buffer)?;
    let expected = COMPLEX_FILELISTS;

    assert_eq!(&actual, &expected);

    Ok(())
}

#[test]
fn test_filelists_xml_writer_file() -> Result<(), MetadataError> {
    let working_dir = TempDir::new("")?;
    let filelists_name = working_dir.path().join("filelists.xml");

    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(filelists_name)
        .unwrap();
    let mut writer = FilelistsXml::new_writer(utils::create_xml_writer(f));

    writer.write_header(0).unwrap();
    // TODO: actually test something here
    // writer.write_package(&common::RPM_EMPTY).unwrap();
    writer.finish()?;

    let mut f = writer.into_inner();

    f.seek(SeekFrom::Start(0))?;
    let mut actual = String::new();

    f.read_to_string(&mut actual).unwrap();

    assert_eq!(actual, EMPTY_FILELISTS);

    Ok(())
}

#[test]
fn test_filelists_xml_read_header() -> Result<(), MetadataError> {
    // Test that the header parses correctly when there are no packages
    let mut filelists_xml =
        FilelistsXml::new_reader(utils::create_xml_reader(EMPTY_FILELISTS.as_bytes()));
    assert_eq!(filelists_xml.read_header()?, 0);
    assert!(matches!(
        filelists_xml.read_header(),
        Err(MetadataError::MissingHeaderError)
    ));

    // Test that the header parses correctly when there are no packages and the footer element doesn't exist (EOF)
    let mut filelists_xml = FilelistsXml::new_reader(utils::create_xml_reader(
        EMPTY_FILELISTS_NO_FOOTER.as_bytes(),
    ));
    assert_eq!(filelists_xml.read_header()?, 0);
    assert!(matches!(
        filelists_xml.read_header(),
        Err(MetadataError::MissingHeaderError)
    ));

    // Test that the header parses correctly when there is no XML declaration at the top
    let mut filelists_xml =
        FilelistsXml::new_reader(utils::create_xml_reader(EMPTY_FILELISTS_NO_DECL.as_bytes()));
    assert_eq!(filelists_xml.read_header()?, 0);
    assert!(matches!(
        filelists_xml.read_header(),
        Err(MetadataError::MissingHeaderError)
    ));

    // Test that the header parses correctly when there is packages
    let mut filelists_xml =
        FilelistsXml::new_reader(utils::create_xml_reader(COMPLEX_FILELISTS.as_bytes()));
    assert_eq!(filelists_xml.read_header()?, 1);
    assert!(matches!(
        filelists_xml.read_header(),
        Err(MetadataError::MissingHeaderError)
    ));

    Ok(())
}

#[test]
fn test_filelists_xml_read_package() -> Result<(), MetadataError> {
    // Test that no package is returned if the xml has no packages
    let mut filelists_xml =
        FilelistsXml::new_reader(utils::create_xml_reader(EMPTY_FILELISTS.as_bytes()));
    assert_eq!(filelists_xml.read_header()?, 0);
    let mut package = None;
    filelists_xml.read_package(&mut package)?;
    assert!(matches!(package, None));

    // Test that no packaged is parsed when there are no packages and the footer element doesn't exist (EOF)
    let mut filelists_xml = FilelistsXml::new_reader(utils::create_xml_reader(
        EMPTY_FILELISTS_NO_FOOTER.as_bytes(),
    ));
    assert_eq!(filelists_xml.read_header()?, 0);
    let mut package = None;
    filelists_xml.read_package(&mut package)?;
    assert!(matches!(package, None));

    // Test that a package is parsed correctly when there is packages
    let mut filelists_xml =
        FilelistsXml::new_reader(utils::create_xml_reader(COMPLEX_FILELISTS.as_bytes()));
    assert_eq!(filelists_xml.read_header()?, 1);
    let mut package = None;
    filelists_xml.read_package(&mut package)?;
    assert!(matches!(package, Some(_)));
    package.take();
    filelists_xml.read_package(&mut package)?;
    assert!(matches!(package, None));

    Ok(())
}
