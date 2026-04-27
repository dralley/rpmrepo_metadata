// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::io::{BufRead, Write};

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::name::QName;
use quick_xml::{Reader, Writer};

use crate::metadata::{
    UpdateCollection, UpdateCollectionModule, UpdateCollectionPackage, UpdateReference,
};

use super::metadata::{RpmMetadata, UpdateRecord, UpdateinfoXml};
use super::{MetadataError, Repository};

const TAG_UPDATES: &str = "updates";
const TAG_UPDATE: &str = "update";
const TAG_ID: &str = "id";
const TAG_TITLE: &str = "title";
const TAG_RELEASE: &str = "release";
const TAG_SEVERITY: &str = "severity";
const TAG_ISSUED: &str = "issued";
const TAG_UPDATED: &str = "updated";
const TAG_RIGHTS: &str = "copyright";
const TAG_SUMMARY: &str = "summary";
const TAG_DESCRIPTION: &str = "description";
const TAG_SOLUTION: &str = "solution";
const TAG_PKGLIST: &str = "pkglist";
const TAG_COLLECTION: &str = "collection";
const TAG_NAME: &str = "name";
const TAG_MODULE: &str = "module";
const TAG_PACKAGE: &str = "package";
const TAG_FILENAME: &str = "filename";
#[allow(dead_code)]
const TAG_REBOOT_SUGGESTED: &str = "reboot_suggested";
const TAG_REFERENCES: &str = "references";
const TAG_REFERENCE: &str = "reference";

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
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

        // <updates>
        let updates_tag = BytesStart::new(TAG_UPDATES);
        self.writer
            .write_event(Event::Start(updates_tag.borrow()))?;

        Ok(())
    }

    pub fn write_updaterecord(&mut self, record: &UpdateRecord) -> Result<(), MetadataError> {
        write_updaterecord(record, &mut self.writer)
    }

    pub fn finish(&mut self) -> Result<(), MetadataError> {
        // </updates>
        self.writer
            .write_event(Event::End(BytesEnd::new(TAG_UPDATES)))?;

        // trailing newline
        self.writer.write_event(Event::Text(BytesText::new("\n")))?;

        // write everything out to disk - otherwise it won't happen until drop() which impedes debugging
        self.writer.get_mut().flush()?;

        Ok(())
    }

    pub fn into_inner(self) -> W {
        self.writer.into_inner()
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

impl<R: BufRead> Iterator for UpdateinfoXmlReader<R> {
    type Item = Result<UpdateRecord, MetadataError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.read_update().transpose()
    }
}

impl UpdateinfoXml {
    pub fn new_writer<W: Write>(writer: quick_xml::Writer<W>) -> UpdateinfoXmlWriter<W> {
        UpdateinfoXmlWriter { writer }
    }

    pub fn new_reader<R: BufRead>(reader: quick_xml::Reader<R>) -> UpdateinfoXmlReader<R> {
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
        match reader.read_event_into(&mut buf)? {
            Event::End(e) if e.name().as_ref() == TAG_UPDATE.as_bytes() => break,
            Event::Start(e) => match std::str::from_utf8(e.name().as_ref()).unwrap_or("") {
                TAG_UPDATE => {
                    record.status = e
                        .try_get_attribute("status")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("status"))?
                        .unescape_value()?
                        .into_owned();
                    record.from = e
                        .try_get_attribute("from")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("from"))?
                        .unescape_value()?
                        .into_owned();
                    record.update_type = e
                        .try_get_attribute("type")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("type"))?
                        .unescape_value()?
                        .into_owned();
                    record.version = e
                        .try_get_attribute("version")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("version"))?
                        .unescape_value()?
                        .into_owned();
                }
                TAG_ID => {
                    record.id = reader
                        .read_text_into(QName(TAG_ID.as_bytes()), &mut format_text_buf)?
                        .decode()?
                        .into_owned();
                }
                TAG_TITLE => {
                    record.title = reader
                        .read_text_into(QName(TAG_TITLE.as_bytes()), &mut format_text_buf)?
                        .decode()?
                        .into_owned();
                }
                TAG_ISSUED => {
                    record.issued_date = Some(
                        reader
                            .read_text_into(QName(TAG_ISSUED.as_bytes()), &mut format_text_buf)?
                            .decode()?
                            .into_owned(),
                    );
                }
                TAG_UPDATED => {
                    record.updated_date = Some(
                        reader
                            .read_text_into(QName(TAG_UPDATED.as_bytes()), &mut format_text_buf)?
                            .decode()?
                            .into_owned(),
                    );
                }
                TAG_RIGHTS => {
                    record.rights = reader
                        .read_text_into(QName(TAG_RIGHTS.as_bytes()), &mut format_text_buf)?
                        .decode()?
                        .into_owned();
                }
                TAG_RELEASE => {
                    record.release = reader
                        .read_text_into(QName(TAG_RELEASE.as_bytes()), &mut format_text_buf)?
                        .decode()?
                        .into_owned();
                }
                TAG_SEVERITY => {
                    record.severity = reader
                        .read_text_into(QName(TAG_SEVERITY.as_bytes()), &mut format_text_buf)?
                        .decode()?
                        .into_owned();
                }
                TAG_SUMMARY => {
                    record.summary = reader
                        .read_text_into(QName(TAG_SUMMARY.as_bytes()), &mut format_text_buf)?
                        .decode()?
                        .into_owned();
                }
                TAG_DESCRIPTION => {
                    record.description = reader
                        .read_text_into(QName(TAG_DESCRIPTION.as_bytes()), &mut format_text_buf)?
                        .decode()?
                        .into_owned();
                }
                TAG_SOLUTION => {
                    record.solution = reader
                        .read_text_into(QName(TAG_SOLUTION.as_bytes()), &mut format_text_buf)?
                        .decode()?
                        .into_owned();
                }
                // reboot_suggested, not clear if it needs to be parsed
                TAG_REFERENCES => {
                    loop {
                        match reader.read_event_into(&mut buf)? {
                            Event::Start(e) if e.name().as_ref() == TAG_REFERENCE.as_bytes() => {
                                let mut reference = UpdateReference::default();
                                reference.href = e
                                    .try_get_attribute("href")?
                                    .ok_or_else(|| MetadataError::MissingAttributeError("href"))?
                                    .unescape_value()?
                                    .into_owned();
                                reference.id = e
                                    .try_get_attribute("id")?
                                    .ok_or_else(|| MetadataError::MissingAttributeError("id"))?
                                    .unescape_value()?
                                    .into_owned();
                                reference.reftype = e
                                    .try_get_attribute("type")?
                                    .ok_or_else(|| MetadataError::MissingAttributeError("type"))?
                                    .unescape_value()?
                                    .into_owned();
                                reference.title = e
                                    .try_get_attribute("title")?
                                    .ok_or_else(|| MetadataError::MissingAttributeError("title"))?
                                    .unescape_value()?
                                    .into_owned();
                                record.references.push(reference);
                            }
                            Event::End(e) if e.name().as_ref() == TAG_REFERENCES.as_bytes() => {
                                break;
                            }
                            _ => (), // TODO
                        }
                    }
                }
                TAG_PKGLIST => record.pkglist = parse_pkglist(reader)?,
                _ => (),
            },
            Event::Eof => return Ok(None),
            _ => (),
        }
        buf.clear();
        format_text_buf.clear();
    }

    Ok(Some(record))
}

pub fn parse_pkglist<R: BufRead>(
    reader: &mut Reader<R>,
) -> Result<Vec<UpdateCollection>, MetadataError> {
    let mut current_collection = None;
    let mut current_package = None;
    let mut buf = Vec::with_capacity(256);
    let mut text_buf = Vec::with_capacity(256);
    let mut collections = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::End(e) if e.name().as_ref() == TAG_PKGLIST.as_bytes() => break,
            Event::Start(e) if e.name().as_ref() == TAG_COLLECTION.as_bytes() => {
                current_collection = Some(UpdateCollection::default());
            }
            Event::End(e) if e.name().as_ref() == TAG_COLLECTION.as_bytes() => {
                collections.push(current_collection.take().unwrap());
            }
            Event::Start(e) => match std::str::from_utf8(e.name().as_ref()).unwrap_or("") {
                TAG_NAME => {
                    current_collection.as_mut().unwrap().name = reader
                        .read_text_into(QName(TAG_NAME.as_bytes()), &mut text_buf)?
                        .decode()?
                        .into_owned();
                }
                TAG_MODULE => {
                    let name = e
                        .try_get_attribute("name")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("name"))?
                        .unescape_value()?
                        .into_owned();
                    let stream = e
                        .try_get_attribute("stream")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("stream"))?
                        .unescape_value()?
                        .into_owned();
                    let version = e
                        .try_get_attribute("version")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("version"))?
                        .unescape_value()?
                        .into_owned();
                    let context = e
                        .try_get_attribute("context")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("context"))?
                        .unescape_value()?
                        .into_owned();
                    let arch = e
                        .try_get_attribute("arch")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("arch"))?
                        .unescape_value()?
                        .into_owned();

                    let version = version.parse()?;

                    let module = UpdateCollectionModule {
                        name,
                        stream,
                        version,
                        context,
                        arch,
                    };
                    current_collection.as_mut().unwrap().module = Some(module);
                }
                TAG_PACKAGE => {
                    let mut package = UpdateCollectionPackage::default();

                    let name = e
                        .try_get_attribute("name")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("name"))?
                        .unescape_value()?
                        .into_owned();
                    let version = e
                        .try_get_attribute("version")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("version"))?
                        .unescape_value()?
                        .into_owned();
                    let epoch = e
                        .try_get_attribute("epoch")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("epoch"))?
                        .unescape_value()?
                        .into_owned();
                    let src = e
                        .try_get_attribute("src")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("src"))?
                        .unescape_value()?
                        .into_owned();
                    let release = e
                        .try_get_attribute("release")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("release"))?
                        .unescape_value()?
                        .into_owned();
                    let arch = e
                        .try_get_attribute("arch")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("arch"))?
                        .unescape_value()?
                        .into_owned();

                    package.name = name;
                    package.version = version;
                    package.release = release;
                    package.arch = arch;
                    package.epoch = epoch;
                    package.src = src;
                    current_package = Some(package);
                    // current_collection.unwrap().packages.push(package);
                }
                TAG_FILENAME => {
                    current_package.as_mut().unwrap().filename = reader
                        .read_text_into(QName(TAG_FILENAME.as_bytes()), &mut text_buf)?
                        .decode()?
                        .into_owned();
                }
                e => panic!("{}", dbg!(e)),
            },
            _ => (), // TODO
        }
        buf.clear();
        text_buf.clear();
    }

    Ok(collections)
}

fn write_updaterecord<W: Write>(
    record: &UpdateRecord,
    writer: &mut Writer<W>,
) -> Result<(), MetadataError> {
    // <update from="updates@fedoraproject.org" status="stable" type="bugfix" version="2.0">
    let mut updates_tag = BytesStart::new(TAG_UPDATE);
    updates_tag.push_attribute(("status", record.status.as_str()));
    updates_tag.push_attribute(("from", record.from.as_str()));
    updates_tag.push_attribute(("type", record.update_type.as_str()));
    updates_tag.push_attribute(("version", record.version.as_str()));
    writer.write_event(Event::Start(updates_tag.borrow()))?;

    // <id>FEDORA-2020-15f9382449</id>
    writer
        .create_element(TAG_ID)
        .write_text_content(BytesText::new(record.id.as_str()))?;

    // <title>nano-4.9.3-1.fc32</title>
    writer
        .create_element(TAG_TITLE)
        .write_text_content(BytesText::new(record.title.as_str()))?;

    // <issued date="2020-05-27 04:10:31"/>
    if let Some(issued_date) = &record.issued_date {
        writer
            .create_element(TAG_ISSUED)
            .write_text_content(BytesText::new(issued_date.as_str()))?;
    }

    // <updated date="2021-04-03 00:15:00"/>
    if let Some(updated_date) = &record.updated_date {
        writer
            .create_element(TAG_UPDATED)
            .write_text_content(BytesText::new(updated_date.as_str()))?;
    }

    // <rights>Copyright (C) 2021 blah blah blah.</rights>
    writer
        .create_element(TAG_RIGHTS)
        .write_text_content(BytesText::new(record.rights.as_str()))?;

    // <release>Fedora 32</release>
    writer
        .create_element(TAG_RELEASE)
        .write_text_content(BytesText::new(record.release.as_str()))?;

    // <severity>Moderate</severity>
    writer
        .create_element(TAG_SEVERITY)
        .write_text_content(BytesText::new(record.severity.as_str()))?;

    // <summary>nano-4.9.3-1.fc32 bugfix update</summary>
    writer
        .create_element(TAG_SUMMARY)
        .write_text_content(BytesText::new(record.summary.as_str()))?;

    // <description>- update to the latest upstream bugfix release</description>
    writer
        .create_element(TAG_DESCRIPTION)
        .write_text_content(BytesText::new(record.description.as_str()))?;

    // <solution>Another description, usually about how the update should be applied</solution>
    writer
        .create_element(TAG_SOLUTION)
        .write_text_content(BytesText::new(record.solution.as_str()))?;

    // It's not clear that any metadata actually uses this
    // // <reboot_suggested>True</reboot_suggestion> (optional)
    // if record.reboot_suggested {
    //     writer
    //         .create_element(TAG_REBOOT_SUGGESTED)
    //         .write_text_content(BytesText::new("True"))?;
    // }

    let tag_references = BytesStart::new(TAG_REFERENCES);
    if !record.references.is_empty() {
        // <references>
        writer.write_event(Event::Start(tag_references.borrow()))?;

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
        writer.write_event(Event::Empty(tag_references.borrow()))?;
    }

    let tag_pkglist = BytesStart::new(TAG_PKGLIST);
    if !record.pkglist.is_empty() {
        // <pkglist>
        writer.write_event(Event::Start(tag_pkglist.borrow()))?;

        for collection in &record.pkglist {
            // <collection short="F35">
            let mut tag_collection = BytesStart::new(TAG_COLLECTION);
            tag_collection.push_attribute(("short", collection.shortname.as_str()));
            writer.write_event(Event::Start(tag_collection.borrow()))?;

            // <name>Fedora 35</name>
            writer
                .create_element(TAG_NAME)
                .write_text_content(BytesText::new(&collection.name))?;

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
                let mut package_tag = BytesStart::new(TAG_PACKAGE);
                package_tag.push_attribute(("name", package.name.as_str()));
                package_tag.push_attribute(("version", package.version.as_str()));
                package_tag.push_attribute(("release", package.release.as_str()));
                package_tag.push_attribute(("epoch", package.epoch.to_string().as_str()));
                package_tag.push_attribute(("arch", package.arch.as_str()));
                package_tag.push_attribute(("src", package.src.as_str()));
                writer.write_event(Event::Start(package_tag.borrow()))?;

                // <filename>pypy-7.3.6-1.fc35.src.rpm</filename>
                writer
                    .create_element(TAG_FILENAME)
                    .write_text_content(BytesText::new(&package.filename))?;

                // <sum type="sha256">8e214681104e4ba73726e0ce11d21b963ec0390fd70458d439ddc72372082034</sum> (optional)
                if let Some(checksum) = &package.checksum {
                    let (checksum_type, value) = checksum.to_values()?;
                    writer
                        .create_element("sum")
                        .with_attribute(("type", checksum_type))
                        .write_text_content(BytesText::new(value))?;
                }
                if package.reboot_suggested {
                    writer
                        .create_element("reboot_suggested")
                        .write_text_content(BytesText::new("1"))?;
                }
                if package.restart_suggested {
                    writer
                        .create_element("restart_suggested")
                        .write_text_content(BytesText::new("1"))?;
                }
                if package.relogin_suggested {
                    writer
                        .create_element("relogin_suggested")
                        .write_text_content(BytesText::new("1"))?;
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
        writer.write_event(Event::Empty(tag_pkglist.borrow()))?;
    }

    // </update>
    writer.write_event(Event::End(updates_tag.to_end()))?;

    // trailing newline
    writer.write_event(Event::Text(BytesText::new("\n")))?;

    // write everything out to disk - otherwise it won't happen until drop() which impedes debugging
    writer.get_mut().flush()?;

    Ok(())
}
