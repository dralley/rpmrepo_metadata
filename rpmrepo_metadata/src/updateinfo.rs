use std::io::{BufRead, Write};

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

use crate::metadata::{
    UpdateCollection, UpdateCollectionModule, UpdateCollectionPackage, UpdateReference,
};

use super::metadata::{RpmMetadata, UpdateRecord, UpdateinfoXml};
use super::{MetadataError, Repository};

const TAG_UPDATES: &[u8] = b"updates";
const TAG_UPDATE: &[u8] = b"update";
const TAG_ID: &[u8] = b"id";
const TAG_TITLE: &[u8] = b"title";
const TAG_RELEASE: &[u8] = b"release";
const TAG_SEVERITY: &[u8] = b"severity";
const TAG_ISSUED: &[u8] = b"issued";
const TAG_UPDATED: &[u8] = b"updated";
const TAG_RIGHTS: &[u8] = b"copyright";
const TAG_SUMMARY: &[u8] = b"summary";
const TAG_DESCRIPTION: &[u8] = b"description";
const TAG_SOLUTION: &[u8] = b"solution";
const TAG_PKGLIST: &[u8] = b"pkglist";
const TAG_COLLECTION: &[u8] = b"collection";
const TAG_NAME: &[u8] = b"name";
const TAG_MODULE: &[u8] = b"module";
const TAG_PACKAGE: &[u8] = b"package";
const TAG_FILENAME: &[u8] = b"filename";
const TAG_REBOOT_SUGGESTED: &[u8] = b"reboot_suggested";
const TAG_REFERENCES: &[u8] = b"references";
const TAG_REFERENCE: &[u8] = b"reference";

impl RpmMetadata for UpdateinfoXml {
    fn filename() -> &'static str {
        "updateinfo.xml"
    }

    fn load_metadata<R: BufRead>(
        repository: &mut Repository,
        reader: Reader<R>,
    ) -> Result<(), MetadataError> {
        let mut reader = UpdateinfoXml::new_reader(reader);
        // reader.read_header()?;
        while let Some(updaterecord) = reader.read_update()? {
            repository
                .advisories_mut()
                .insert(updaterecord.id.clone(), updaterecord);
        }
        Ok(())
    }

    fn write_metadata<W: Write>(
        repository: &Repository,
        writer: Writer<W>,
    ) -> Result<(), MetadataError> {
        let mut writer = UpdateinfoXml::new_writer(writer);
        writer.write_header()?;
        for record in repository.advisories().values() {
            writer.write_updaterecord(record)?;
        }
        writer.finish()?;
        Ok(())
    }
}

pub struct UpdateinfoXmlWriter<W: Write> {
    writer: Writer<W>,
}

impl<W: Write> UpdateinfoXmlWriter<W> {
    pub fn write_header(&mut self) -> Result<(), MetadataError> {
        // <?xml version="1.0" encoding="UTF-8"?>
        self.writer
            .write_event(Event::Decl(BytesDecl::new(b"1.0", Some(b"UTF-8"), None)))?;

        // <updates>
        let updates_tag = BytesStart::borrowed_name(TAG_UPDATES);
        self.writer
            .write_event(Event::Start(updates_tag.to_borrowed()))?;

        Ok(())
    }

    pub fn write_updaterecord(&mut self, record: &UpdateRecord) -> Result<(), MetadataError> {
        write_updaterecord(record, &mut self.writer)
    }

    pub fn finish(&mut self) -> Result<(), MetadataError> {
        // </updates>
        self.writer
            .write_event(Event::End(BytesEnd::borrowed(TAG_UPDATES)))?;

        // trailing newline
        self.writer
            .write_event(Event::Text(BytesText::from_plain_str("\n")))?;

        // write everything out to disk - otherwise it won't happen until drop() which impedes debugging
        self.writer.inner().flush()?;

        Ok(())
    }

    pub fn into_inner(self) -> Writer<W> {
        self.writer
    }
}

pub struct UpdateinfoXmlReader<R: BufRead> {
    reader: Reader<R>,
}

impl<R: BufRead> UpdateinfoXmlReader<R> {
    pub fn read_update(&mut self) -> Result<Option<UpdateRecord>, MetadataError> {
        parse_updaterecord(&mut self.reader)
    }
}

impl UpdateinfoXml {
    pub fn new_writer<W: Write>(writer: Writer<W>) -> UpdateinfoXmlWriter<W> {
        UpdateinfoXmlWriter { writer }
    }

    pub fn new_reader<R: BufRead>(reader: Reader<R>) -> UpdateinfoXmlReader<R> {
        UpdateinfoXmlReader { reader }
    }
}

fn parse_updaterecord<R: BufRead>(
    reader: &mut Reader<R>,
) -> Result<Option<UpdateRecord>, MetadataError> {
    let mut buf = Vec::new();
    let mut format_text_buf = Vec::new();

    let mut record = UpdateRecord::default();

    // TODO: get rid of unwraps, various branches could happen in wrong order
    loop {
        match reader.read_event(&mut buf)? {
            Event::End(e) if e.name() == TAG_UPDATE => break,
            Event::Start(e) => match e.name() {
                TAG_UPDATE => {
                    // for attr in e.attributes() {
                    //     let attr = attr?;

                    //     match attr.key {
                    //         b"status" => record.status = attr.unescape_and_decode_value(reader)?,
                    //         b"from" => record.from = attr.unescape_and_decode_value(reader)?,
                    //         b"type" => record.update_type = attr.unescape_and_decode_value(reader)?,
                    //         b"version" => record.version = attr.unescape_and_decode_value(reader)?,
                    //         a @ _ => panic!("unrecognized attribute {}", std::str::from_utf8(a)?),
                    //     }
                    // }

                    record.status = e
                        .try_get_attribute("status")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("status"))?
                        .unescape_and_decode_value(reader)?;
                    record.from = e
                        .try_get_attribute("from")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("from"))?
                        .unescape_and_decode_value(reader)?;
                    record.update_type = e
                        .try_get_attribute("type")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("type"))?
                        .unescape_and_decode_value(reader)?;
                    record.version = e
                        .try_get_attribute("version")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("version"))?
                        .unescape_and_decode_value(reader)?;
                }
                TAG_ID => {
                    record.id = reader.read_text(TAG_ID, &mut format_text_buf)?;
                }
                TAG_TITLE => {
                    record.title = reader.read_text(TAG_TITLE, &mut format_text_buf)?;
                }
                TAG_ISSUED => {
                    record.issued_date = Some(reader.read_text(TAG_ISSUED, &mut format_text_buf)?);
                }
                TAG_UPDATED => {
                    record.updated_date =
                        Some(reader.read_text(TAG_UPDATED, &mut format_text_buf)?);
                }
                TAG_RIGHTS => {
                    record.rights = reader.read_text(TAG_RIGHTS, &mut format_text_buf)?;
                }
                TAG_RELEASE => {
                    record.release = reader.read_text(TAG_RELEASE, &mut format_text_buf)?;
                }
                TAG_SEVERITY => {
                    record.severity = reader.read_text(TAG_SEVERITY, &mut format_text_buf)?;
                }
                TAG_SUMMARY => {
                    record.summary = reader.read_text(TAG_SUMMARY, &mut format_text_buf)?;
                }
                TAG_DESCRIPTION => {
                    record.description = reader.read_text(TAG_DESCRIPTION, &mut format_text_buf)?;
                }
                TAG_SOLUTION => {
                    record.solution = reader.read_text(TAG_SOLUTION, &mut format_text_buf)?;
                }
                // reboot_suggested, not clear if it needs to be parsed
                TAG_REFERENCES => {
                    loop {
                        match reader.read_event(&mut buf)? {
                            Event::Start(e) if e.name() == TAG_REFERENCE => {
                                let mut reference = UpdateReference::default();
                                for attr in e.attributes() {
                                    let attr = attr?;
                                    reference.href = e
                                        .try_get_attribute("href")?
                                        .ok_or_else(|| {
                                            MetadataError::MissingAttributeError("href")
                                        })?
                                        .unescape_and_decode_value(reader)?;
                                    reference.id = e
                                        .try_get_attribute("id")?
                                        .ok_or_else(|| MetadataError::MissingAttributeError("id"))?
                                        .unescape_and_decode_value(reader)?;
                                    reference.reftype = e
                                        .try_get_attribute("type")?
                                        .ok_or_else(|| {
                                            MetadataError::MissingAttributeError("type")
                                        })?
                                        .unescape_and_decode_value(reader)?;
                                    reference.title = e
                                        .try_get_attribute("title")?
                                        .ok_or_else(|| {
                                            MetadataError::MissingAttributeError("title")
                                        })?
                                        .unescape_and_decode_value(reader)?;
                                }
                                record.references.push(reference);
                            }
                            Event::End(e) if e.name() == TAG_REFERENCES => break,
                            _ => (), // TODO
                        }
                    }
                }
                TAG_PKGLIST => {
                    loop {
                        match reader.read_event(&mut buf)? {
                            // Event::Start(e) => match e.name() {
                            //     TAG_MODULE => {
                            //         let name = e
                            //             .try_get_attribute("name")?
                            //             .ok_or_else(|| {
                            //                 MetadataError::MissingAttributeError("name")
                            //             })?
                            //             .unescape_and_decode_value(reader)?;
                            //         let stream = e
                            //             .try_get_attribute("stream")?
                            //             .ok_or_else(|| {
                            //                 MetadataError::MissingAttributeError("stream")
                            //             })?
                            //             .unescape_and_decode_value(reader)?;
                            //         let version = e
                            //             .try_get_attribute("version")?
                            //             .ok_or_else(|| {
                            //                 MetadataError::MissingAttributeError("version")
                            //             })?
                            //             .unescape_and_decode_value(reader)?;
                            //         let context = e
                            //             .try_get_attribute("context")?
                            //             .ok_or_else(|| {
                            //                 MetadataError::MissingAttributeError("context")
                            //             })?
                            //             .unescape_and_decode_value(reader)?;
                            //         let arch = e
                            //             .try_get_attribute("arch")?
                            //             .ok_or_else(|| {
                            //                 MetadataError::MissingAttributeError("arch")
                            //             })?
                            //             .unescape_and_decode_value(reader)?;

                            //         let version = version.parse()?;

                            //         let module = UpdateCollectionModule {
                            //             name,
                            //             stream,
                            //             version,
                            //             context,
                            //             arch,
                            //         };
                            //         collection.unwrap().module = Some(module);
                            //    }
                            //     TAG_PACKAGE => {
                            //         let mut package = UpdateCollectionPackage::default();

                            //         let name = e
                            //             .try_get_attribute("name")?
                            //             .ok_or_else(|| {
                            //                 MetadataError::MissingAttributeError("name")
                            //             })?
                            //             .unescape_and_decode_value(reader)?;
                            //         let version = e
                            //             .try_get_attribute("version")?
                            //             .ok_or_else(|| {
                            //                 MetadataError::MissingAttributeError("version")
                            //             })?
                            //             .unescape_and_decode_value(reader)?;
                            //         let epoch = e
                            //             .try_get_attribute("epoch")?
                            //             .ok_or_else(|| {
                            //                 MetadataError::MissingAttributeError("epoch")
                            //             })?
                            //             .unescape_and_decode_value(reader)?;
                            //         let src = e
                            //             .try_get_attribute("src")?
                            //             .ok_or_else(|| MetadataError::MissingAttributeError("src"))?
                            //             .unescape_and_decode_value(reader)?;
                            //         let release = e
                            //             .try_get_attribute("release")?
                            //             .ok_or_else(|| {
                            //                 MetadataError::MissingAttributeError("release")
                            //             })?
                            //             .unescape_and_decode_value(reader)?;
                            //         let arch = e
                            //             .try_get_attribute("arch")?
                            //             .ok_or_else(|| {
                            //                 MetadataError::MissingAttributeError("arch")
                            //             })?
                            //             .unescape_and_decode_value(reader)?;

                            //         package.name = name;
                            //         package.version = version;
                            //         package.release = release;
                            //         package.arch = arch;
                            //         package.epoch = epoch;
                            //         package.src = src;

                            //         collection.unwrap().packages.push(package);
                            //     }
                            // },
                            Event::End(e) if e.name() == TAG_REFERENCE => break,
                            _ => (), // TODO
                        }
                    }
                }
                _ => (),
            },
            Event::Eof => break,
            _ => (),
        }
        buf.clear();
        format_text_buf.clear();
    }

    Ok(None)
}

fn write_updaterecord<W: Write>(
    record: &UpdateRecord,
    writer: &mut Writer<W>,
) -> Result<(), MetadataError> {
    // <update from="updates@fedoraproject.org" status="stable" type="bugfix" version="2.0">
    let mut updates_tag = BytesStart::borrowed_name(TAG_UPDATE);
    updates_tag.push_attribute(("status", record.status.as_str()));
    updates_tag.push_attribute(("from", record.from.as_str()));
    updates_tag.push_attribute(("type", record.update_type.as_str()));
    updates_tag.push_attribute(("version", record.version.as_str()));
    writer.write_event(Event::Start(updates_tag.to_borrowed()))?;

    // <id>FEDORA-2020-15f9382449</id>
    writer
        .create_element(TAG_ID)
        .write_text_content(BytesText::from_plain_str(record.id.as_str()))?;

    // <title>nano-4.9.3-1.fc32</title>
    writer
        .create_element(TAG_TITLE)
        .write_text_content(BytesText::from_plain_str(record.title.as_str()))?;

    // <issued date="2020-05-27 04:10:31"/>
    if let Some(issued_date) = &record.issued_date {
        writer
            .create_element(TAG_ISSUED)
            .write_text_content(BytesText::from_plain_str(issued_date.as_str()))?;
    }

    // <updated date="2021-04-03 00:15:00"/>
    if let Some(updated_date) = &record.updated_date {
        writer
            .create_element(TAG_UPDATED)
            .write_text_content(BytesText::from_plain_str(updated_date.as_str()))?;
    }

    // <rights>Copyright (C) 2021 blah blah blah.</rights>
    writer
        .create_element(TAG_RIGHTS)
        .write_text_content(BytesText::from_plain_str(record.rights.as_str()))?;

    // <release>Fedora 32</release>
    writer
        .create_element(TAG_RELEASE)
        .write_text_content(BytesText::from_plain_str(record.release.as_str()))?;

    // <severity>Moderate</severity>
    writer
        .create_element(TAG_SEVERITY)
        .write_text_content(BytesText::from_plain_str(record.severity.as_str()))?;

    // <summary>nano-4.9.3-1.fc32 bugfix update</summary>
    writer
        .create_element(TAG_SUMMARY)
        .write_text_content(BytesText::from_plain_str(record.summary.as_str()))?;

    // <description>- update to the latest upstream bugfix release</description>
    writer
        .create_element(TAG_DESCRIPTION)
        .write_text_content(BytesText::from_plain_str(record.description.as_str()))?;

    // <solution>Another description, usually about how the update should be applied</solution>
    writer
        .create_element(TAG_SOLUTION)
        .write_cdata_content(BytesText::from_plain_str(record.solution.as_str()))?;

    // It's not clear that any metadata actually uses this
    // // <reboot_suggested>True</reboot_suggestion> (optional)
    // if record.reboot_suggested {
    //     writer
    //         .create_element(TAG_REBOOT_SUGGESTED)
    //         .write_text_content(BytesText::from_plain_str("True"))?;
    // }

    let tag_references = BytesStart::borrowed_name(TAG_REFERENCES);
    if !record.references.is_empty() {
        // <references>
        writer.write_event(Event::Start(tag_references.to_borrowed()))?;

        for reference in &record.references {
            // <reference href="https://bugzilla.redhat.com/show_bug.cgi?id=1839351" id="1839351" type="bugzilla" title="nano-4.9.3 is available"/>
            writer
                .create_element(TAG_REFERENCE)
                .with_attribute(("href", reference.href.as_str()))
                .with_attribute(("id", reference.id.as_str()))
                .with_attribute(("type", reference.reftype.as_str()))
                .with_attribute(("title", reference.title.as_str()))
                .write_empty()?;
        }

        // </references>
        writer.write_event(Event::End(tag_references.to_end()))?;
    } else {
        // <references/>
        writer.write_event(Event::Empty(tag_references.to_borrowed()))?;
    }

    let tag_pkglist = BytesStart::borrowed_name(TAG_PKGLIST);
    if !record.pkglist.is_empty() {
        // <pkglist>
        writer.write_event(Event::Start(tag_pkglist.to_borrowed()))?;

        for collection in &record.pkglist {
            // <collection short="F35">
            let mut tag_collection = BytesStart::borrowed_name(TAG_COLLECTION);
            tag_collection.push_attribute(("short", collection.shortname.as_str()));
            writer.write_event(Event::Start(tag_collection.to_borrowed()))?;

            // <name>Fedora 35</name>
            writer
                .create_element(TAG_NAME)
                .write_text_content(BytesText::from_plain_str(&collection.name))?;

            // <module stream="3.0" version="8000020190425181943" arch="x86_64" name="freeradius" context="75ec4169" />
            if let Some(module) = &collection.module {
                writer
                    .create_element(TAG_MODULE)
                    .with_attribute(("name", module.name.as_str()))
                    .with_attribute(("stream", module.stream.as_str()))
                    .with_attribute(("version", module.version.to_string().as_str()))
                    .with_attribute(("context", module.context.as_str()))
                    .with_attribute(("arch", module.arch.as_str()))
                    .write_empty()?;
            }

            for package in &collection.packages {
                // <package src="kexec-tools-2.0.4-32.el7_0.1.src.rpm" name="kexec-tools" epoch="0" version="2.0.4" release="32.el7" arch="x86_64">
                let mut package_tag = BytesStart::borrowed_name(TAG_PACKAGE);
                package_tag.push_attribute(("name", package.name.as_str()));
                package_tag.push_attribute(("version", package.version.as_str()));
                package_tag.push_attribute(("release", package.release.as_str()));
                package_tag.push_attribute(("epoch", package.epoch.to_string().as_str()));
                package_tag.push_attribute(("arch", package.arch.as_str()));
                package_tag.push_attribute(("src", package.src.as_str()));
                writer.write_event(Event::Start(package_tag.to_borrowed()))?;

                // <filename>pypy-7.3.6-1.fc35.src.rpm</filename>
                writer
                    .create_element(TAG_FILENAME)
                    .write_text_content(BytesText::from_plain_str(&package.filename))?;

                // <sum type="sha256">8e214681104e4ba73726e0ce11d21b963ec0390fd70458d439ddc72372082034</sum> (optional)
                if let Some(checksum) = &package.checksum {
                    let (checksum_type, value) = checksum.to_values()?;
                    writer
                        .create_element("sum")
                        .with_attribute(("type", checksum_type))
                        .write_text_content(BytesText::from_plain_str(value))?;
                }
                if package.reboot_suggested {
                    writer
                        .create_element("reboot_suggested")
                        .write_text_content(BytesText::from_plain_str("1"))?;
                }
                if package.restart_suggested {
                    writer
                        .create_element("restart_suggested")
                        .write_text_content(BytesText::from_plain_str("1"))?;
                }
                if package.relogin_suggested {
                    writer
                        .create_element("relogin_suggested")
                        .write_text_content(BytesText::from_plain_str("1"))?;
                }

                // </package>
                writer.write_event(Event::End(package_tag.to_end()))?;
            }

            // </collection>
            writer.write_event(Event::End(tag_collection.to_end()))?;
        }

        // </pkglist>
        writer.write_event(Event::End(tag_pkglist.to_end()))?;
    } else {
        // <pkglist/>
        writer.write_event(Event::Empty(tag_pkglist.to_borrowed()))?;
    }

    // </update>
    writer.write_event(Event::End(updates_tag.to_end()))?;

    // trailing newline
    writer.write_event(Event::Text(BytesText::from_plain_str("\n")))?;

    // write everything out to disk - otherwise it won't happen until drop() which impedes debugging
    writer.inner().flush()?;

    Ok(())
}
