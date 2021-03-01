use quick_xml::{Reader, Writer};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use std::io::{BufRead, Write};

use super::common::{EVR, try_get_attribute};
use super::metadata::{RpmMetadata, XML_NS_FILELISTS};
use super::MetadataError;

const TAG_FILELISTS: &[u8] = b"filelists";
const TAG_PACKAGE: &[u8] = b"package";
const TAG_VERSION: &[u8] = b"version";
const TAG_FILE: &[u8] = b"file";


#[derive(Debug, PartialEq, Default)]
pub struct Updates {
    pub updates: Vec<UpdateRecord>,
}

#[derive(Debug, PartialEq, Default)]
pub struct UpdateRecord {
    from: String,
    update_type: String,
    version: String,
    id: String,
    title: String,
    issued_date: u64,
    updated_date: Option<u64>,
    rights: String,
    release: String,
    severity: String,
    summary: String,
    description: String,
    references: Vec<UpdateReference>,
    pkglist: Vec<UpdateCollection>,
}

#[derive(Debug, PartialEq, Default)]
pub struct UpdateCollection {
    name: String,
    shortname: String,
    packages: Vec<Package>,
}

#[derive(Debug, PartialEq, Default)]
pub struct UpdateReference {
    href: String,
    id: String,
    title: String,
    reftype: String,
}

#[derive(Debug, PartialEq, Default)]
pub struct UpdatePackage {
    epoch: u32,
    filename: String,
    name: String,
    reboot_suggested: bool,
    release: String,
    src: String,
    checksum: Checksum,
    version: String,
}

impl RpmMetadata for Updates {

    fn deserialize<R: BufRead>(reader: &mut Reader<R>) -> Result<Updates, MetadataError> {
        let updates = Updates::default();
        Ok(updates)
    }

    fn serialize<W: Write>(&self, writer: &mut Writer<W>) -> Result<(), MetadataError> {
        writer.write_event(Event::Decl(BytesDecl::new(b"1.0", Some(b"UTF-8"), None)))?;

        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::MetadataIO;
    use once_cell::sync::OnceCell;
    use pretty_assertions::assert_eq;
    use std::path::Path;


}
