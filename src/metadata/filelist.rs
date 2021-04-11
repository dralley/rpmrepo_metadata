use std::io::{BufRead, Write};

use quick_xml::events::{BytesDecl, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

use super::metadata::{
    FileType, FilelistsXml, Package, PackageFile, RpmMetadata, EVR, XML_NS_FILELISTS,
};
use super::{MetadataError, Repository};

const TAG_FILELISTS: &[u8] = b"filelists";
const TAG_PACKAGE: &[u8] = b"package";
const TAG_VERSION: &[u8] = b"version";
const TAG_FILE: &[u8] = b"file";

impl RpmMetadata for FilelistsXml {
    const NAME: &'static str = "filelists.xml";

    fn load_metadata<R: BufRead>(
        repository: &mut Repository,
        reader: &mut Reader<R>,
    ) -> Result<(), MetadataError> {
        read_filelists_xml(repository, reader)
    }

    fn write_metadata<W: Write>(
        repository: &Repository,
        writer: &mut Writer<W>,
    ) -> Result<(), MetadataError> {
        write_filelists_xml(repository, writer)
    }
}

// <?xml version="1.0" encoding="UTF-8"?>
// <filelists xmlns="http://linux.duke.edu/metadata/filelists" packages="1">
//   <package pkgid="6a915b6e1ad740994aa9688d70a67ff2b6b72e0ced668794aeb27b2d0f2e237b" name="fontconfig" arch="x86_64">
//     <version epoch="0" ver="2.8.0" rel="5.el6"/>
//     <file type="dir">/etc/fonts/conf.avail</file>
//     ...
//     <file>/etc/fonts/conf.avail/10-autohint.conf</file>
//   </package>
// </filelists>
fn read_filelists_xml<R: BufRead>(
    repository: &mut Repository,
    reader: &mut Reader<R>,
) -> Result<(), MetadataError> {
    let mut buf = Vec::new();

    let mut found_metadata_tag = false;

    loop {
        match reader.read_event(&mut buf)? {
            Event::Start(e) => match e.name() {
                TAG_FILELISTS => {
                    found_metadata_tag = true;
                }
                TAG_PACKAGE => {
                    parse_package(repository, reader, &e)?;
                }
                _ => (),
            },
            Event::Eof => break,
            Event::Decl(_) => (), // TOOD
            _ => (),
        }
    }
    if !found_metadata_tag {
        // TODO
    }
    Ok(())
}

fn write_filelists_xml<W: Write>(
    repository: &Repository,
    writer: &mut Writer<W>,
) -> Result<(), MetadataError> {
    // <?xml version="1.0" encoding="UTF-8"?>
    writer.write_event(Event::Decl(BytesDecl::new(b"1.0", Some(b"UTF-8"), None)))?;

    // <filelists xmlns="http://linux.duke.edu/metadata/filelists" packages="210">
    let num_pkgs = repository.packages().len().to_string();
    let mut filelists_tag = BytesStart::borrowed_name(TAG_FILELISTS);
    filelists_tag.push_attribute(("xmlns", XML_NS_FILELISTS));
    filelists_tag.push_attribute(("packages", num_pkgs.as_str()));
    writer.write_event(Event::Start(filelists_tag.to_borrowed()))?;

    for package in repository.packages().values() {
        // <package pkgid="a2d3bce512f79b0bc840ca7912a86bbc0016cf06d5c363ffbb6fd5e1ef03de1b" name="fontconfig" arch="x86_64">
        let mut package_tag = BytesStart::borrowed_name(TAG_PACKAGE);
        let (_, pkgid) = package.checksum.to_values()?;
        package_tag.push_attribute(("pkgid", pkgid));
        package_tag.push_attribute(("name", package.name.as_str()));
        package_tag.push_attribute(("arch", package.arch.as_str()));
        writer.write_event(Event::Start(package_tag.to_borrowed()))?;

        // <version epoch="0" ver="2.8.0" rel="5.fc33"/>
        let (epoch, version, release) = package.evr.values();
        let mut version_tag = BytesStart::borrowed_name(TAG_VERSION);
        version_tag.push_attribute(("epoch", epoch));
        version_tag.push_attribute(("ver", version));
        version_tag.push_attribute(("rel", release));
        writer.write_event(Event::Empty(version_tag))?;

        // <file type="dir">/etc/fonts/conf.avail</file>
        for file in &package.rpm_files {
            let mut file_tag = BytesStart::borrowed_name(TAG_FILE);
            file_tag.push_attribute(("type".as_bytes(), file.filetype.to_values()));
            writer.write_event(Event::Start(file_tag.to_borrowed()))?;
            writer.write_event(Event::Text(BytesText::from_plain_str(&file.path)))?;
            writer.write_event(Event::End(file_tag.to_end()))?;
        }

        // </package>
        writer.write_event(Event::End(package_tag.to_end()))?;
    }

    // </filelists>
    writer.write_event(Event::End(filelists_tag.to_end()))?;
    Ok(())
}

//   <package pkgid="a2d3bce512f79b0bc840ca7912a86bbc0016cf06d5c363ffbb6fd5e1ef03de1b" name="fontconfig" arch="x86_64">
//     <version epoch="0" ver="2.8.0" rel="5.fc33"/>
//     <file type="dir">/etc/fonts/conf.avail</file>
//     ...
//     <file>/etc/fonts/conf.avail/10-autohint.conf</file>
//   </package>
pub fn parse_package<R: BufRead>(
    repository: &mut Repository,
    reader: &mut Reader<R>,
    open_tag: &BytesStart,
) -> Result<(), MetadataError> {
    let mut buf = Vec::new();

    let pkgid = open_tag
        .try_get_attribute("pkgid")?
        .ok_or_else(|| MetadataError::MissingAttributeError("pkgid"))?
        .unescape_and_decode_value(reader)?;
    let name = open_tag
        .try_get_attribute("name")?
        .ok_or_else(|| MetadataError::MissingAttributeError("name"))?
        .unescape_and_decode_value(reader)?;
    let arch = open_tag
        .try_get_attribute("arch")?
        .ok_or_else(|| MetadataError::MissingAttributeError("arch"))?
        .unescape_and_decode_value(reader)?;

    let mut package = repository
        .packages_mut()
        .entry(pkgid)
        .or_insert(Package::default()); // TODO

    // TODO: using empty strings as null value is slightly questionable
    if package.name.is_empty() {
        package.name = name.to_owned();
    }

    if package.arch.is_empty() {
        package.arch = arch.to_owned();
    }

    loop {
        match reader.read_event(&mut buf)? {
            Event::End(e) if e.name() == TAG_PACKAGE => break,

            Event::Start(e) => match e.name() {
                TAG_VERSION => {
                    package.evr = parse_evr(reader, &e)?;
                }
                TAG_FILE => {
                    let file = parse_file(reader, &e)?;
                    package.rpm_files.push(file);
                }
                _ => (),
            },
            _ => (),
        }
    }
    Ok(())
}

// <version epoch="0" ver="2.8.0" rel="5.fc33"/>
pub fn parse_evr<R: BufRead>(
    reader: &mut Reader<R>,
    open_tag: &BytesStart,
) -> Result<EVR, MetadataError> {
    let epoch = open_tag
        .try_get_attribute("epoch")?
        .ok_or_else(|| MetadataError::MissingAttributeError("epoch"))?
        .unescape_and_decode_value(reader)?;
    let version = open_tag
        .try_get_attribute("ver")?
        .ok_or_else(|| MetadataError::MissingAttributeError("ver"))?
        .unescape_and_decode_value(reader)?;
    let release = open_tag
        .try_get_attribute("rel")?
        .ok_or_else(|| MetadataError::MissingAttributeError("rel"))?
        .unescape_and_decode_value(reader)?;

    // TODO: double-allocations
    Ok(EVR::new(&epoch, &version, &release))
}

// <file type="dir">/etc/fonts/conf.avail</file>
pub fn parse_file<R: BufRead>(
    reader: &mut Reader<R>,
    open_tag: &BytesStart,
) -> Result<PackageFile, MetadataError> {
    let mut file = PackageFile::default();
    file.path = reader.read_text(open_tag.name(), &mut Vec::new())?;

    if let Some(filetype) = open_tag.try_get_attribute("type")? {
        file.filetype = FileType::try_create(filetype.value.as_ref())?;
    }

    Ok(file)
}
