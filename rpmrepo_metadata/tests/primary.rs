extern crate rpmrepo_metadata;

use pretty_assertions::assert_eq;
use rpmrepo_metadata::*;
use std::fs::{Metadata, OpenOptions};
use std::io::{Cursor, Read, Seek, SeekFrom};
use tempdir::TempDir;

mod common;

static EMPTY_PRIMARY: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<metadata xmlns="http://linux.duke.edu/metadata/common" xmlns:rpm="http://linux.duke.edu/metadata/rpm" packages="0">
</metadata>
"#;

static EMPTY_PRIMARY_NO_FOOTER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<metadata xmlns="http://linux.duke.edu/metadata/common" xmlns:rpm="http://linux.duke.edu/metadata/rpm" packages="0">"#;

static EMPTY_PRIMARY_NO_DECL: &str = r#"<metadata xmlns="http://linux.duke.edu/metadata/common" xmlns:rpm="http://linux.duke.edu/metadata/rpm" packages="0">"#;

static COMPLEX_PRIMARY: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<metadata xmlns="http://linux.duke.edu/metadata/common" xmlns:rpm="http://linux.duke.edu/metadata/rpm" packages="1">
  <package type="rpm">
    <name>complex-package</name>
    <arch>x86_64</arch>
    <version epoch="1" ver="2.3.4" rel="5.el8"/>
    <checksum type="sha256" pkgid="YES">bbb7b0e9350a0f75b923bdd0ef4f9af39765c668a3e70bfd3486ea9f0f618aaf</checksum>
    <summary>A package for exercising many different features of RPM metadata</summary>
    <description>Complex package</description>
    <packager>Michael Bluth</packager>
    <url>http://bobloblaw.com</url>
    <time file="1627052744" build="1627052743"/>
    <size package="8680" installed="117" archive="932"/>
    <location href="complex-package-2.3.4-5.el8.x86_64.rpm"/>
    <format>
      <rpm:license>MPLv2</rpm:license>
      <rpm:vendor>Bluth Company</rpm:vendor>
      <rpm:group>Development/Tools</rpm:group>
      <rpm:buildhost>localhost</rpm:buildhost>
      <rpm:sourcerpm>complex-package-2.3.4-5.el8.src.rpm</rpm:sourcerpm>
      <rpm:header-range start="4504" end="8413"/>
      <rpm:provides>
        <rpm:entry name="/usr/bin/ls"/>
        <rpm:entry name="complex-package" flags="EQ" epoch="1" ver="2.3.4" rel="5.el8"/>
        <rpm:entry name="complex-package(x86-64)" flags="EQ" epoch="1" ver="2.3.4" rel="5.el8"/>
        <rpm:entry name="laughter" flags="EQ" epoch="0" ver="33"/>
        <rpm:entry name="narration(ronhoward)"/>
      </rpm:provides>
      <rpm:requires>
        <rpm:entry name="/usr/bin/bash"/>
        <rpm:entry name="/usr/sbin/useradd" pre="1"/>
        <rpm:entry name="arson" flags="GE" epoch="0" ver="1.0.0" rel="1"/>
        <rpm:entry name="fur" flags="LE" epoch="0" ver="2"/>
        <rpm:entry name="staircar" flags="LE" epoch="0" ver="99.1" rel="3"/>
      </rpm:requires>
      <rpm:conflicts>
        <rpm:entry name="foxnetwork" flags="GT" epoch="0" ver="5555"/>
      </rpm:conflicts>
      <rpm:obsoletes>
        <rpm:entry name="bluemangroup" flags="LT" epoch="0" ver="32.1" rel="0"/>
        <rpm:entry name="cornballer" flags="LT" epoch="0" ver="444"/>
      </rpm:obsoletes>
      <rpm:suggests>
        <rpm:entry name="(bobloblaw &gt;= 1.1 if maritimelaw else anyone &lt; 0.5.1-2)"/>
        <rpm:entry name="(dove and return)"/>
        <rpm:entry name="(job or money &gt; 9000)"/>
      </rpm:suggests>
      <rpm:enhances>
        <rpm:entry name="(bananas or magic)"/>
      </rpm:enhances>
      <rpm:recommends>
        <rpm:entry name="((hiding and attic) if light-treason)"/>
        <rpm:entry name="GeneParmesan(PI)"/>
        <rpm:entry name="yacht" flags="GT" epoch="9" ver="11.0" rel="0"/>
      </rpm:recommends>
      <rpm:supplements>
        <rpm:entry name="((hiding and illusion) unless alliance-of-magicians)"/>
        <rpm:entry name="comedy" flags="EQ" epoch="0" ver="11.1" rel="4"/>
      </rpm:supplements>
      <file>/etc/complex/pkg.cfg</file>
      <file>/usr/bin/complex_a</file>
    </format>
  </package>
</metadata>
"#;

#[test]
fn test_primary_xml_writer_empty() -> Result<(), MetadataError> {
    let mut writer = PrimaryXml::new_writer(utils::create_xml_writer(Cursor::new(Vec::new())));

    writer.write_header(0)?;
    writer.finish()?;

    let buffer = writer.into_inner().into_inner();

    let actual = std::str::from_utf8(&buffer)?;
    let expected = EMPTY_PRIMARY;

    assert_eq!(&actual, &expected);

    Ok(())
}

#[test]
fn test_primary_xml_writer_complex_pkg() -> Result<(), MetadataError> {
    let mut writer = PrimaryXml::new_writer(utils::create_xml_writer(Cursor::new(Vec::new())));

    writer.write_header(1)?;
    writer.write_package(&common::COMPLEX_PACKAGE)?;
    writer.finish()?;

    let buffer = writer.into_inner().into_inner();

    let actual = std::str::from_utf8(&buffer)?;
    let expected = COMPLEX_PRIMARY;

    assert_eq!(&actual, &expected);

    Ok(())
}

#[test]
fn test_primary_xml_writer_file() -> Result<(), MetadataError> {
    let working_dir = TempDir::new("")?;
    let other_name = working_dir.path().join("primary.xml");

    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(other_name)
        .unwrap();

    let mut writer = PrimaryXml::new_writer(utils::create_xml_writer(f));

    writer.write_header(0).unwrap();
    // writer.write_package(&common::RPM_EMPTY).unwrap();
    writer.finish()?;

    let mut f = writer.into_inner();

    f.seek(SeekFrom::Start(0))?;
    let mut actual = String::new();

    f.read_to_string(&mut actual).unwrap();

    assert_eq!(actual, EMPTY_PRIMARY);

    Ok(())
}

#[test]
fn test_primary_xml_read_header() -> Result<(), MetadataError> {
    // Test that the header parses correctly when there are no packages
    let mut primary_xml =
        PrimaryXml::new_reader(utils::create_xml_reader(EMPTY_PRIMARY.as_bytes()));
    assert_eq!(primary_xml.read_header()?, 0);
    assert!(matches!(
        primary_xml.read_header(),
        Err(MetadataError::MissingHeaderError)
    ));

    // Test that the header parses correctly when there are no packages and the footer element doesn't exist (EOF)
    let mut primary_xml =
        PrimaryXml::new_reader(utils::create_xml_reader(EMPTY_PRIMARY_NO_FOOTER.as_bytes()));
    assert_eq!(primary_xml.read_header()?, 0);
    assert!(matches!(
        primary_xml.read_header(),
        Err(MetadataError::MissingHeaderError)
    ));

    // Test that the header parses correctly when there is no XML declaration at the top
    let mut primary_xml =
        PrimaryXml::new_reader(utils::create_xml_reader(EMPTY_PRIMARY_NO_DECL.as_bytes()));
    assert_eq!(primary_xml.read_header()?, 0);
    assert!(matches!(
        primary_xml.read_header(),
        Err(MetadataError::MissingHeaderError)
    ));

    // Test that the header parses correctly when there is packages
    let mut primary_xml =
        PrimaryXml::new_reader(utils::create_xml_reader(COMPLEX_PRIMARY.as_bytes()));
    assert_eq!(primary_xml.read_header()?, 1);
    assert!(matches!(
        primary_xml.read_header(),
        Err(MetadataError::MissingHeaderError)
    ));

    Ok(())
}

#[test]
fn test_primary_xml_read_package() -> Result<(), MetadataError> {
    // Test that no package is returned if the xml has no packages
    let mut primary_xml =
        PrimaryXml::new_reader(utils::create_xml_reader(EMPTY_PRIMARY.as_bytes()));
    assert_eq!(primary_xml.read_header()?, 0);
    let mut package = None;
    primary_xml.read_package(&mut package)?;
    assert!(matches!(package, None));

    // Test that no packaged is parsed when there are no packages and the footer element doesn't exist (EOF)
    let mut primary_xml =
        PrimaryXml::new_reader(utils::create_xml_reader(EMPTY_PRIMARY_NO_FOOTER.as_bytes()));
    assert_eq!(primary_xml.read_header()?, 0);
    let mut package = None;
    primary_xml.read_package(&mut package)?;
    assert!(matches!(package, None));

    // Test that a package is parsed correctly when there is packages
    let mut primary_xml =
        PrimaryXml::new_reader(utils::create_xml_reader(COMPLEX_PRIMARY.as_bytes()));
    assert_eq!(primary_xml.read_header()?, 1);
    let mut package = None;
    primary_xml.read_package(&mut package)?;
    assert!(matches!(package, Some(_)));
    package.take();
    primary_xml.read_package(&mut package)?;
    assert!(matches!(package, None));

    Ok(())
}
