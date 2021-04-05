use quick_xml::events::{BytesDecl, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};
use std::io::{BufRead, Write};

use crate::RpmRepository;

use super::metadata::{RpmMetadata, UpdateInfoXml, UpdateRecord};
use super::MetadataError;

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
const TAG_PACKAGE: &[u8] = b"package";
const TAG_FILENAME: &[u8] = b"filename";
const TAG_REBOOT_SUGGESTED: &[u8] = b"reboot_suggested";
const TAG_REFERENCES: &[u8] = b"references";
const TAG_REFERENCE: &[u8] = b"reference";

impl RpmMetadata for UpdateInfoXml {
    const NAME: &'static str = "updateinfo.xml";

    fn load_metadata<R: BufRead>(
        repository: &mut RpmRepository,
        reader: &mut Reader<R>,
    ) -> Result<(), MetadataError> {
        read_updateinfo_xml(repository, reader)
    }

    fn write_metadata<W: Write>(
        repository: &RpmRepository,
        writer: &mut Writer<W>,
    ) -> Result<(), MetadataError> {
        write_updateinfo_xml(repository, writer)
    }
}

fn read_updateinfo_xml<R: BufRead>(
    repository: &mut RpmRepository,
    reader: &mut Reader<R>,
) -> Result<(), MetadataError> {
    Ok(())
}

fn write_updateinfo_xml<W: Write>(
    repository: &RpmRepository,
    writer: &mut Writer<W>,
) -> Result<(), MetadataError> {
    // <?xml version="1.0" encoding="UTF-8"?>
    writer.write_event(Event::Decl(BytesDecl::new(b"1.0", Some(b"UTF-8"), None)))?;

    // <updates>
    let updates_tag = BytesStart::borrowed_name(TAG_UPDATES);
    writer.write_event(Event::Start(updates_tag.to_borrowed()))?;

    for record in &repository.advisories {
        write_updaterecord(&record, writer)?;
    }

    // </updates>
    writer.write_event(Event::End(updates_tag.to_end()))?;

    // trailing newline
    writer.write_event(Event::Text(BytesText::from_plain_str("\n")))?;
    Ok(())
}

fn write_updaterecord<W: Write>(
    record: &UpdateRecord,
    writer: &mut Writer<W>,
) -> Result<(), MetadataError> {
    //   <update from="updates@fedoraproject.org" status="stable" type="bugfix" version="2.0">
    let mut updates_tag = BytesStart::borrowed_name(TAG_UPDATE);
    updates_tag.push_attribute(("from", record.from.as_str()));
    updates_tag.push_attribute(("status", record.status.as_str()));
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

    // <rights>Copyright (C) 2021 Red Hat, Inc. and others.</rights>
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

    // TODO: find example
    writer
        .create_element(TAG_SOLUTION)
        .write_cdata_content(BytesText::from_plain_str(record.solution.as_str()))?;

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
            // <collection short="F32">
            let mut tag_collection = BytesStart::borrowed_name(TAG_COLLECTION);
            tag_collection.push_attribute(("short", collection.shortname.as_str()));
            writer.write_event(Event::Start(tag_collection.to_borrowed()))?;

            // updatecollectionmodule

            for package in &collection.packages {

            }
            //     <name>Fedora 32</name>
            //     <package name="fbzx" version="4.2.0" release="1.fc32" epoch="0" arch="src" src="https://download.fedoraproject.org/pub/fedora/linux/updates/32/SRPMS/f/fbzx-4.2.0-1.fc32.src.rpm">
            //     <filename>fbzx-4.2.0-1.fc32.src.rpm</filename>
            //     </package>
            //     <package name="fbzx-debugsource" version="4.2.0" release="1.fc32" epoch="0" arch="armv7hl" src="https://download.fedoraproject.org/pub/fedora/linux/updates/32/armv7hl/f/fbzx-debugsource-4.2.0-1.fc32.armv7hl.rpm">
            //     <filename>fbzx-debugsource-4.2.0-1.fc32.armv7hl.rpm</filename>
            //     </package>
            //     <package name="fbzx-debuginfo" version="4.2.0" release="1.fc32" epoch="0" arch="armv7hl" src="https://download.fedoraproject.org/pub/fedora/linux/updates/32/armv7hl/f/fbzx-debuginfo-4.2.0-1.fc32.armv7hl.rpm">
            //     <filename>fbzx-debuginfo-4.2.0-1.fc32.armv7hl.rpm</filename>
            //     </package>

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

    Ok(())
}
