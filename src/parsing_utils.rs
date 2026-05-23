use std::borrow::Cow;
use std::io::BufRead;

use quick_xml::{self, events::*};

use crate::MetadataError;
use crate::constants::XML_VERSION;

/// Resolve an attribute value, handling double-encoded `&#38;`.
///
/// Workaround for https://github.com/rpm-software-management/createrepo_c/issues/286
pub(crate) fn resolve_attr<'a>(
    attr: &quick_xml::events::attributes::Attribute<'a>,
) -> Result<Cow<'a, str>, MetadataError> {
    let normalized = attr.normalized_value(XML_VERSION)?;
    if normalized.contains("&#38;") {
        Ok(Cow::Owned(normalized.replace("&#38;", "&")))
    } else {
        Ok(normalized)
    }
}

/// Unescape XML text content.
pub(crate) fn resolve_text<'a>(
    text: &quick_xml::events::BytesText<'a>,
) -> Result<Cow<'a, str>, MetadataError> {
    match text.xml_content(XML_VERSION)? {
        Cow::Borrowed(s) => {
            // s borrows from the buffer with lifetime 'a — unescape preserves it
            Ok(quick_xml::escape::unescape(s)?)
        }
        Cow::Owned(s) => {
            // decoded is owned — unescape result would borrow a local, so always own
            let unescaped = quick_xml::escape::unescape(&s)?;
            Ok(Cow::Owned(unescaped.into_owned()))
        }
    }
}

/// Extension trait for unescaping XML text content into an owned `String`.
pub(crate) trait XmlTextUnescape {
    /// Decode and unescape XML text content.
    fn xml_text(&self) -> Result<String, crate::MetadataError>;
}

impl XmlTextUnescape for quick_xml::events::BytesText<'_> {
    fn xml_text(&self) -> Result<String, crate::MetadataError> {
        let decoded = self.xml_content(XML_VERSION)?;
        let unescaped = quick_xml::escape::unescape(&decoded)?;
        Ok(unescaped.into_owned())
    }
}

/// Extension trait for normalizing and unescaping XML attribute values into an owned `String`.
pub(crate) trait XmlAttrUnescape {
    /// Normalize and unescape an XML attribute value, handling double-encoded ampersands.
    fn xml_attr(&self) -> Result<String, crate::MetadataError>;
}

impl XmlAttrUnescape for quick_xml::events::attributes::Attribute<'_> {
    /// Normalize an attribute value then resolve double-encoded ampersands.
    ///
    /// Workaround for an issue first encountered in createrepo_c:
    /// https://github.com/rpm-software-management/createrepo_c/issues/286
    ///
    /// `normalized_value` handles standard XML entity resolution (`&amp;` -> `&`,
    /// `&#38;` -> `&`). Some RPM repositories contain double-encoded ampersands
    /// (`&amp;#38;`) which after the first pass leave `&#38;` as a remnant.
    /// This mirrors createrepo_c's `unescape_ampersand_from_values`.
    fn xml_attr(&self) -> Result<String, crate::MetadataError> {
        let normalized = self.normalized_value(XML_VERSION)?.into_owned();
        Ok(normalized.replace("&#38;", "&"))
    }
}

/// Parse the opening element of a metadata XML file and return the `packages` count.
pub fn parse_header_tag<R: BufRead>(
    reader: &mut quick_xml::Reader<R>,
    expected_tag: &str,
) -> Result<usize, MetadataError> {
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Decl(_) => (),
            Event::Start(e) if e.name().as_ref() == expected_tag.as_bytes() => {
                let count = e.try_get_attribute("packages")?.unwrap().value;
                return Ok(std::str::from_utf8(&count)?.parse()?);
            }
            _ => return Err(MetadataError::MissingHeaderError),
        }
    }
}

/// Extract epoch, version, and release attributes from a `<version>` XML element.
pub fn parse_evr_from_tag<'a>(
    tag: &'a BytesStart<'a>,
) -> Result<(Cow<'a, str>, Cow<'a, str>, Cow<'a, str>), MetadataError> {
    let mut epoch = Cow::Borrowed("0");
    let mut version_cow = None;
    let mut release_cow = None;

    for attr_result in tag.attributes() {
        let attr = attr_result?;
        match attr.key.as_ref() {
            b"epoch" => epoch = resolve_attr(&attr)?,
            b"ver" => version_cow = Some(resolve_attr(&attr)?),
            b"rel" => release_cow = Some(resolve_attr(&attr)?),
            _ => (),
        }
    }

    let version = version_cow.ok_or(MetadataError::MissingAttributeError("ver"))?;
    let release = release_cow.ok_or(MetadataError::MissingAttributeError("rel"))?;
    Ok((epoch, version, release))
}
