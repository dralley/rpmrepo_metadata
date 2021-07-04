extern crate rpmrepo;

use std::fs::OpenOptions;
use std::io::{Cursor, Read, Seek, SeekFrom};

use pretty_assertions::assert_eq;
use tempdir::TempDir;
use quick_xml;

use rpmrepo::metadata::*;

mod common;

static EMPTY_FILELISTS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<filelists xmlns="http://linux.duke.edu/metadata/filelists" packages="0">
</filelists>
"#;

static COMPLEX_FILELISTS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<filelists xmlns="http://linux.duke.edu/metadata/filelists" packages="1">
  <package pkgid="6e46283a16954c9cecd3799246eb1a426d7d8a8b1bc8d57c55c3da4253e200e5" name="complex-package" arch="x86_64">
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
    let mut buf = Vec::new();

    let xml_writer = quick_xml::Writer::new_with_indent(Cursor::new(&mut buf), b' ', 2);
    let mut writer = FilelistsXml::new_writer(xml_writer);
    writer.write_header(0)?;
    writer.finish()?;

    let buffer= writer.into_inner().into_inner();

    let actual = std::str::from_utf8(buffer)?;
    let expected = EMPTY_FILELISTS;
    assert_eq!(&actual, &expected);

    Ok(())
}

#[test]
fn test_filelists_xml_writer_complex_pkg() -> Result<(), MetadataError> {
    let mut buf = Vec::new();

    let xml_writer = quick_xml::Writer::new_with_indent(Cursor::new(&mut buf), b' ', 2);
    let mut writer = FilelistsXml::new_writer(xml_writer);

    writer.write_header(1)?;
    writer.write_package(&common::COMPLEX_PACKAGE)?;
    writer.finish()?;

    let buffer = writer.into_inner().into_inner();

    let actual = std::str::from_utf8(buffer)?;
    let expected = COMPLEX_FILELISTS;

    assert_eq!(&actual, &expected);

    Ok(())
}

#[test]
#[should_panic]
fn test_filelists_xml_writer_not_enough_packages() {
    let mut buf = Vec::new();

    let xml_writer = quick_xml::Writer::new_with_indent(Cursor::new(&mut buf), b' ', 2);
    let mut writer = FilelistsXml::new_writer(xml_writer);

    writer.write_header(1).unwrap();
    writer.finish().unwrap();
}

#[test]
#[should_panic]
fn test_filelists_xml_writer_too_many_packages() {
    let mut buf = Vec::new();

    let xml_writer = quick_xml::Writer::new_with_indent(Cursor::new(&mut buf), b' ', 2);
    let mut writer = FilelistsXml::new_writer(xml_writer);

    writer.write_header(1).unwrap();
    writer.write_package(&common::RPM_EMPTY).unwrap();
    writer.write_package(&common::RPM_WITH_NON_ASCII).unwrap();
    writer.finish().unwrap();
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

    let xml_writer = quick_xml::Writer::new_with_indent(f, b' ', 2);
    let mut writer = FilelistsXml::new_writer(xml_writer);

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

// pub(crate) fn to_string<M: RpmMetadata>(&self) -> Result<String, MetadataError> {
//     let bytes = self.to_bytes::<M>()?;
//     Ok(String::from_utf8(bytes).map_err(|e| e.utf8_error())?)
// }

// pub(crate) fn to_bytes<M: RpmMetadata>(&self) -> Result<Vec<u8>, MetadataError> {
//     let mut buf = Vec::new();
//     let mut writer = Writer::new_with_indent(Cursor::new(&mut buf), b' ', 2);
//     M::write_metadata(self, &mut writer)?;
//     Ok(writer.into_inner().into_inner().to_vec())
// }

// Test roundtrip (serialize + deserialize) on a real repomd.xml (Fedora 33 x86_64 release "everything")
// #[test]
// fn test_filelists_roundtrip() -> Result<(), MetadataError> {
//     let first_deserialize = FilelistsXml::from_file(Path::new(FIXTURE_FILELIST_PATH))?;
//     let first_serialize = first_deserialize.to_string()?;

//     let second_deserialize = FilelistsXml::from_str(&first_serialize)?;
//     let second_serialize = second_deserialize.to_string()?;

//     assert_eq!(first_deserialize, second_deserialize);
//     assert_eq!(first_serialize, second_serialize);

//     Ok(())
// }

// #[test]
// fn repomd() -> Result<(), MetadataError> {
//     // let fixture_path = "./tests/assets/complex_repo/";
//     let fixture_path = "../test_repo/";

//     let repo = Repository::load_from_directory(fixture_path.as_ref())?;

//     assert_eq!(repo.packages().len(), 10700);
//     // assert_eq!(repo.packages.len(), 3);

//     // repo.to_directory("./tests/assets/test_repo/".as_ref())?;

//     Ok(())
// }
