use std::io::{BufRead, Write};

use quick_xml::escape::partial_escape;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

use crate::Checksum;

use super::metadata::{Changelog, OtherXml, Package, RpmMetadata, XML_NS_OTHER};
use super::{MetadataError, Repository, EVR};

const TAG_OTHERDATA: &[u8] = b"otherdata";
const TAG_PACKAGE: &[u8] = b"package";
const TAG_VERSION: &[u8] = b"version";
const TAG_CHANGELOG: &[u8] = b"changelog";

impl RpmMetadata for OtherXml {
    fn filename() -> &'static str {
        "other.xml"
    }

    fn load_metadata<R: BufRead>(
        repository: &mut Repository,
        reader: Reader<R>,
    ) -> Result<(), MetadataError> {
        let mut reader = OtherXml::new_reader(reader);
        reader.read_header()?;
        let mut package = None;
        loop {
            reader.read_package(&mut package)?;
            if package == None {
                break;
            }
            let pkgid = package.as_ref().unwrap().pkgid().to_owned();
            repository
                .packages_mut()
                .entry(pkgid)
                .and_modify(|p| {
                    std::mem::swap(
                        &mut p.rpm_changelogs,
                        &mut package.as_mut().unwrap().rpm_changelogs,
                    )
                })
                .or_insert(package.take().unwrap());
        }
        Ok(())
    }

    fn write_metadata<W: Write>(
        repository: &Repository,
        writer: Writer<W>,
    ) -> Result<(), MetadataError> {
        let mut writer = OtherXml::new_writer(writer);
        writer.write_header(repository.packages().len())?;
        for package in repository.packages().values() {
            writer.write_package(package)?;
        }
        writer.finish()
    }
}

impl OtherXml {
    pub fn new_writer<W: Write>(writer: Writer<W>) -> OtherXmlWriter<W> {
        OtherXmlWriter { writer }
    }

    pub fn new_reader<R: BufRead>(reader: Reader<R>) -> OtherXmlReader<R> {
        OtherXmlReader { reader }
    }
}

pub struct OtherXmlWriter<W: Write> {
    writer: Writer<W>,
}

impl<W: Write> OtherXmlWriter<W> {
    pub fn write_header(&mut self, num_pkgs: usize) -> Result<(), MetadataError> {
        // <?xml version="1.0" encoding="UTF-8"?>
        self.writer
            .write_event(Event::Decl(BytesDecl::new(b"1.0", Some(b"UTF-8"), None)))?;

        // <otherdata xmlns="http://linux.duke.edu/metadata/other" packages="200">
        let mut other_tag = BytesStart::borrowed_name(TAG_OTHERDATA);
        other_tag.push_attribute(("xmlns", XML_NS_OTHER));
        other_tag.push_attribute(("packages", num_pkgs.to_string().as_str()));
        self.writer.write_event(Event::Start(other_tag))?;

        Ok(())
    }

    pub fn write_package(&mut self, package: &Package) -> Result<(), MetadataError> {
        let mut package_tag = BytesStart::borrowed_name(TAG_PACKAGE);
        let (_, pkgid) = package.checksum().to_values()?;
        package_tag.push_attribute(("pkgid", pkgid));
        package_tag.push_attribute(("name", package.name()));
        package_tag.push_attribute(("arch", package.arch()));
        self.writer
            .write_event(Event::Start(package_tag.to_borrowed()))?;

        let (epoch, version, release) = package.evr().values();
        // <version epoch="0" ver="2.8.0" rel="5.el6"/>
        let mut version_tag = BytesStart::borrowed_name(TAG_VERSION);
        version_tag.push_attribute(("epoch", epoch));
        version_tag.push_attribute(("ver", version));
        version_tag.push_attribute(("rel", release));
        self.writer.write_event(Event::Empty(version_tag))?;

        for changelog in package.changelogs() {
            //  <changelog author="dalley &lt;dalley@redhat.com&gt; - 2.7.2-1" date="1251720000">- Update to 2.7.2</changelog>
            self.writer
                .create_element(TAG_CHANGELOG)
                .with_attribute(("author", changelog.author.as_str()))
                .with_attribute(("date", changelog.date.to_string().as_str()))
                .write_text_content(BytesText::from_escaped(partial_escape(
                    &changelog.description.as_bytes(),
                )))?;
        }

        // </package>
        self.writer.write_event(Event::End(package_tag.to_end()))?;

        Ok(())
    }

    pub fn finish(&mut self) -> Result<(), MetadataError> {
        // </otherdata>
        self.writer
            .write_event(Event::End(BytesEnd::borrowed(TAG_OTHERDATA)))?;

        // trailing newline
        self.writer
            .write_event(Event::Text(BytesText::from_plain_str("\n")))?;

        // write everything out to disk - otherwise it won't happen until drop() which impedes debugging
        self.writer.inner().flush()?;

        Ok(())
    }

    pub fn into_inner(self) -> W {
        self.writer.into_inner()
    }
}

pub struct OtherXmlReader<R: BufRead> {
    reader: Reader<R>,
}

impl<R: BufRead> OtherXmlReader<R> {
    pub fn read_header(&mut self) -> Result<usize, MetadataError> {
        parse_header(&mut self.reader)
    }

    pub fn read_package(&mut self, package: &mut Option<Package>) -> Result<(), MetadataError> {
        parse_package(package, &mut self.reader)
    }
}

// <?xml version="1.0" encoding="UTF-8"?>
// <otherdata xmlns="http://linux.duke.edu/metadata/other" packages="35">
fn parse_header<R: BufRead>(reader: &mut Reader<R>) -> Result<usize, MetadataError> {
    let mut buf = Vec::new();

    // TODO: get rid of this buffer
    loop {
        match reader.read_event(&mut buf)? {
            Event::Decl(_) => (),
            Event::Start(e) if e.name() == TAG_OTHERDATA => {
                let count = e.try_get_attribute("packages")?.unwrap().value;
                return Ok(std::str::from_utf8(&count)?.parse()?);
            }
            _ => return Err(MetadataError::MissingHeaderError),
        }
    }
}

//   <package pkgid="6a915b6e1ad740994aa9688d70a67ff2b6b72e0ced668794aeb27b2d0f2e237b" name="fontconfig" arch="x86_64">
//     <version epoch="0" ver="2.8.0" rel="5.el6"/>
//     <changelog author="Behdad Esfahbod &lt;besfahbo@redhat.com&gt; - 2.7.2-1" date="1251720000">- Update to 2.7.2</changelog>
//     <changelog author="Behdad Esfahbod &lt;besfahbo@redhat.com&gt; - 2.7.3-1" date="1252411200">- Update to 2.7.3</changelog>
//     <changelog author="Behdad Esfahbod &lt;besfahbo@redhat.com&gt; - 2.8.0-1" date="1259841600">- Update to 2.8.0</changelog>
//   </package>
pub fn parse_package<R: BufRead>(
    package: &mut Option<Package>,
    reader: &mut Reader<R>,
) -> Result<(), MetadataError> {
    let mut buf = Vec::new();

    // TODO: get rid of unwraps, various branches could happen in wrong order
    loop {
        match reader.read_event(&mut buf)? {
            Event::End(e) if e.name() == TAG_PACKAGE => break,
            Event::Start(e) => match e.name() {
                TAG_PACKAGE => {
                    let pkgid = e
                        .try_get_attribute("pkgid")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("pkgid"))?
                        .unescape_and_decode_value(reader)?;
                    let name = e
                        .try_get_attribute("name")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("name"))?
                        .unescape_and_decode_value(reader)?;
                    let arch = e
                        .try_get_attribute("arch")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("arch"))?
                        .unescape_and_decode_value(reader)?;

                    if let Some(pkg) = package {
                        assert!(pkg.pkgid() == &pkgid); // TODO err instead of assert
                    } else {
                        let mut pkg = Package::default();
                        pkg.set_name(&name)
                            .set_arch(&arch)
                            .set_checksum(Checksum::Unknown(pkgid));
                        *package = Some(pkg);
                    };
                }
                TAG_VERSION => {
                    package.as_mut().unwrap().set_evr(parse_evr(reader, &e)?);
                }
                TAG_CHANGELOG => {
                    let changelog = parse_changelog(reader, &e)?;
                    // TODO: Temporary changelog?
                    package.as_mut().unwrap().add_changelog(
                        &changelog.author,
                        &changelog.description,
                        changelog.date,
                    );
                }
                _ => (),
            },
            Event::Eof => break,
            _ => (),
        }
    }

    // package.parse_state |= ParseState::OTHER;
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
