// <?xml version="1.0" encoding="UTF-8"?>
// <otherdata xmlns="http://linux.duke.edu/metadata/other" packages="1">
// <package pkgid="a2d3bce512f79b0bc840ca7912a86bbc0016cf06d5c363ffbb6fd5e1ef03de1b" name="deadbeef-devel" arch="x86_64">
//   <version epoch="0" ver="1.8.4" rel="2.fc33"/>
//   <changelog author="RPM Fusion Release Engineering &lt;leigh123linux@gmail.com&gt; - 0.7.3-0.2.20190209git373f556" date="1551700800">- Rebuilt for https://fedoraproject.org/wiki/Fedora_30_Mass_Rebuild</changelog>
//   <changelog author="Vasiliy N. Glazov &lt;vascom2@gmail.com&gt; - 1.8.0-1" date="1554724800">- Update to 1.8.0</changelog>
//   <changelog author="Vasiliy N. Glazov &lt;vascom2@gmail.com&gt; - 1.8.1-1" date="1561723200">- Update to 1.8.1</changelog>
//   <changelog author="Leigh Scott &lt;leigh123linux@gmail.com&gt; - 1.8.1-2" date="1565179200">- Rebuild for new ffmpeg version</changelog>
//   <changelog author="Vasiliy N. Glazov &lt;vascom2@gmail.com&gt; - 1.8.2-1" date="1565352000">- Update to 1.8.2</changelog>
// </package>
// </package>
// </otherdata>

use std::io::{BufRead, Write};

use quick_xml::events::{BytesDecl, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

use super::metadata::{Changelog, OtherXml, Package, RpmMetadata, EVR, XML_NS_OTHER};
use super::{MetadataError, RpmRepository};

const TAG_OTHERDATA: &[u8] = b"otherdata";
const TAG_PACKAGE: &[u8] = b"package";
const TAG_VERSION: &[u8] = b"version";
const TAG_CHANGELOG: &[u8] = b"changelog";

impl RpmMetadata for OtherXml {
    const NAME: &'static str = "other.xml";

    fn load_metadata<R: BufRead>(
        repository: &mut RpmRepository,
        reader: &mut Reader<R>,
    ) -> Result<(), MetadataError> {
        read_other_xml(repository, reader)
    }

    fn write_metadata<W: Write>(
        repository: &RpmRepository,
        writer: &mut Writer<W>,
    ) -> Result<(), MetadataError> {
        write_other_xml(repository, writer)
    }
}

fn read_other_xml<R: BufRead>(
    repository: &mut RpmRepository,
    reader: &mut Reader<R>,
) -> Result<(), MetadataError> {
    let mut buf = Vec::new();

    let mut found_metadata_tag = false;

    loop {
        match reader.read_event(&mut buf)? {
            Event::Start(e) => match e.name() {
                TAG_OTHERDATA => {
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

pub fn write_other_xml<W: Write>(
    repository: &RpmRepository,
    writer: &mut Writer<W>,
) -> Result<(), MetadataError> {
    // <?xml version="1.0" encoding="UTF-8"?>
    writer.write_event(Event::Decl(BytesDecl::new(b"1.0", Some(b"UTF-8"), None)))?;

    // <otherdata xmlns="http://linux.duke.edu/metadata/other" packages="200">
    let mut other_tag = BytesStart::borrowed_name(TAG_OTHERDATA);
    other_tag.push_attribute(("xmlns", XML_NS_OTHER));
    other_tag.push_attribute(("packages", repository.packages.len().to_string().as_str()));

    // <filelists>
    writer.write_event(Event::Start(other_tag.to_borrowed()))?;

    // <packages>
    for package in repository.packages.values() {
        let mut package_tag = BytesStart::borrowed_name(TAG_PACKAGE);
        let (_, pkgid) = package.checksum.to_values()?;
        package_tag.push_attribute(("pkgid".as_bytes(), pkgid.as_bytes()));
        package_tag.push_attribute(("name".as_bytes(), package.name.as_bytes()));
        package_tag.push_attribute(("arch".as_bytes(), package.arch.as_bytes()));
        writer.write_event(Event::Start(package_tag.to_borrowed()))?;

        let (epoch, version, release) = package.evr.values();
        // <version epoch="0" ver="2.8.0" rel="5.el6"/>
        let mut version_tag = BytesStart::borrowed_name(TAG_VERSION);
        version_tag.push_attribute(("epoch".as_bytes(), epoch.as_bytes()));
        version_tag.push_attribute(("ver".as_bytes(), version.as_bytes()));
        version_tag.push_attribute(("rel".as_bytes(), release.as_bytes()));
        writer.write_event(Event::Empty(version_tag))?;

        for changelog in &package.rpm_changelogs {
            //  <changelog author="dalley &lt;dalley@redhat.com&gt; - 2.7.2-1" date="1251720000">- Update to 2.7.2</changelog>
            writer
                .create_element(TAG_CHANGELOG)
                .with_attribute(("author".as_bytes(), changelog.author.as_str().as_bytes()))
                .with_attribute((
                    "date".as_bytes(),
                    format!("{}", changelog.date).as_str().as_bytes(),
                ))
                .write_text_content(BytesText::from_plain_str(&changelog.description))?;
        }

        // </package>
        writer.write_event(Event::End(package_tag.to_end()))?;
    }
    // </otherdata>
    writer.write_event(Event::End(other_tag.to_end()))?;
    Ok(())
}

//   <package pkgid="6a915b6e1ad740994aa9688d70a67ff2b6b72e0ced668794aeb27b2d0f2e237b" name="fontconfig" arch="x86_64">
//     <version epoch="0" ver="2.8.0" rel="5.el6"/>
//     <changelog author="Behdad Esfahbod &lt;besfahbo@redhat.com&gt; - 2.7.2-1" date="1251720000">- Update to 2.7.2</changelog>
//     <changelog author="Behdad Esfahbod &lt;besfahbo@redhat.com&gt; - 2.7.3-1" date="1252411200">- Update to 2.7.3</changelog>
//     <changelog author="Behdad Esfahbod &lt;besfahbo@redhat.com&gt; - 2.8.0-1" date="1259841600">- Update to 2.8.0</changelog>
//   </package>
pub fn parse_package<R: BufRead>(
    repository: &mut RpmRepository,
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
        .packages
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
                TAG_CHANGELOG => {
                    let file = parse_changelog(reader, &e)?;
                    package.rpm_changelogs.push(file);
                }
                _ => (),
            },
            _ => (),
        }
    }

    Ok(())
}

// <version epoch="0" ver="2.8.0" rel="5.el6"/>
pub fn parse_evr<R: BufRead>(
    reader: &mut Reader<R>,
    open_tag: &BytesStart,
) -> Result<EVR, MetadataError> {
    let epoch = open_tag
        .try_get_attribute("epoch")?
        .unwrap()
        .unescape_and_decode_value(reader)?;
    let version = open_tag
        .try_get_attribute("ver")?
        .unwrap()
        .unescape_and_decode_value(reader)?;
    let release = open_tag
        .try_get_attribute("rel")?
        .unwrap()
        .unescape_and_decode_value(reader)?;

    // TODO: double-allocations
    Ok(EVR::new(&epoch, &version, &release))
}

pub fn parse_changelog<R: BufRead>(
    reader: &mut Reader<R>,
    open_tag: &BytesStart,
) -> Result<Changelog, MetadataError> {
    let mut changelog = Changelog::default();

    changelog.author = open_tag
        .try_get_attribute("author")?
        .unwrap()
        .unescape_and_decode_value(reader)?;
    changelog.date = open_tag
        .try_get_attribute("date")?
        .unwrap()
        .unescape_and_decode_value(reader)?
        .parse()?;

    changelog.description = reader.read_text(open_tag.name(), &mut Vec::new())?;

    Ok(changelog)
}
