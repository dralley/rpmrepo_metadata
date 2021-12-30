use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};
use std::io::{BufRead, Write};

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
const TAG_COPYRIGHT: &[u8] = b"copyright";
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
        loop {
            let updaterecord = reader.read_update()?;
            repository.advisories_mut().entry(updaterecord.id.clone()).or_insert(updaterecord);
        }
        Ok(())
    }

    fn write_metadata<W: Write>(
        repository: &Repository,
        writer: Writer<W>,
    ) -> Result<(), MetadataError> {
        let mut writer = UpdateinfoXml::new_writer(writer);
        writer.write_header()?;
        for (_, record) in repository.advisories() {
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
    pub fn read_update(&mut self) -> Result<UpdateRecord, MetadataError> {
        Ok(UpdateRecord::default())
    }
}


// impl Iterator for PackageParser {
//     type Item = Result<Package, MetadataError>;
//     fn next(&mut self) -> Option<Self::Item> {
//         self.parse_package().transpose()
//     }
// }

impl UpdateinfoXml {
    pub fn new_writer<W: Write>(writer: Writer<W>) -> UpdateinfoXmlWriter<W> {
        UpdateinfoXmlWriter { writer }
    }

    pub fn new_reader<R: BufRead>(reader: Reader<R>) -> UpdateinfoXmlReader<R> {
        UpdateinfoXmlReader { reader }
    }
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
        .create_element(TAG_COPYRIGHT)
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
