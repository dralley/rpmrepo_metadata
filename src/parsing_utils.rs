use std::borrow::Cow;
use std::io::BufRead;

use quick_xml::{self, events::*};

use crate::MetadataError;
use crate::constants::XML_VERSION;

/// Replace characters forbidden by XML 1.0 with U+FFFD and log a warning.
///
/// TAB, LF, and CR are valid XML whitespace and are preserved. RPM header
/// fields are otherwise untrusted input, and a forbidden control character
/// would make the entire generated metadata document invalid.
pub(crate) fn sanitize_xml_text<F>(
    value: impl Into<String>,
    field: &str,
    package_nevra: F,
) -> String
where
    F: FnOnce() -> Option<String>,
{
    let value = value.into();
    if value.chars().all(is_xml_1_0_char) {
        return value;
    }

    match package_nevra() {
        Some(nevra) => log::warn!(
            "Replacing XML-forbidden characters in {field} for package {nevra} with U+FFFD"
        ),
        None => log::warn!("Replacing XML-forbidden characters in {field} with U+FFFD"),
    }

    value
        .chars()
        .map(|character| {
            if is_xml_1_0_char(character) {
                character
            } else {
                '\u{FFFD}'
            }
        })
        .collect()
}

/// Reports whether a Unicode scalar value is permitted by XML 1.0's `Char` production.
pub(crate) fn is_xml_1_0_char(value: char) -> bool {
    matches!(
        value,
        '\u{9}' | '\u{A}' | '\u{D}' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}'
    )
}

/// Normalize an attribute value then resolve double-encoded ampersands.
///
/// Workaround for an issue first encountered in createrepo_c:
/// https://github.com/rpm-software-management/createrepo_c/issues/286
///
/// `normalized_value` handles standard XML entity resolution (`&amp;` -> `&`,
/// `&#38;` -> `&`). Some RPM repositories contain double-encoded ampersands
/// (`&amp;#38;`) which after the first pass leave `&#38;` as a remnant.
/// This mirrors createrepo_c's `unescape_ampersand_from_values`.
pub(crate) fn resolve_attr<'a>(
    attr: &quick_xml::events::attributes::Attribute<'a>,
) -> Result<Cow<'a, str>, MetadataError> {
    let normalized = attr.normalized_value(XML_VERSION)?;
    let mut s: Option<String> = None;
    loop {
        let new_s = {
            let current: &str = s.as_deref().unwrap_or(normalized.as_ref());
            if !current.contains("&#38;") {
                break;
            }
            current.replace("&#38;", "&")
        };
        s = Some(new_s);
    }
    Ok(match s {
        Some(owned) => Cow::Owned(owned),
        None => normalized,
    })
}

/// Unescape XML text content.
pub(crate) fn resolve_text<'a>(
    text: &quick_xml::events::BytesText<'a>,
) -> Result<Cow<'a, str>, MetadataError> {
    match text.xml_content(XML_VERSION) {
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
    fn xml_text<'a>(&'a self) -> Result<Cow<'a, str>, crate::MetadataError>;
}

impl XmlTextUnescape for quick_xml::events::BytesText<'_> {
    fn xml_text<'a>(&'a self) -> Result<Cow<'a, str>, MetadataError> {
        resolve_text(self)
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
            Event::Start(e) if e.name().as_ref() == expected_tag => {
                let count = e.try_get_attribute("packages")?.unwrap().value;
                return Ok(count.parse()?);
            }
            _ => return Err(MetadataError::MissingHeaderError),
        }
    }
}

/// Extract epoch, version, and release attributes from a `<version>` XML element.
#[allow(clippy::type_complexity)]
pub fn parse_evr_from_tag<'a>(
    tag: &'a BytesStart<'a>,
) -> Result<(Cow<'a, str>, Cow<'a, str>, Cow<'a, str>), MetadataError> {
    let mut epoch = Cow::Borrowed("0");
    let mut version_cow = None;
    let mut release_cow = None;

    for attr_result in tag.attributes() {
        let attr = attr_result?;
        match attr.key.as_ref() {
            "epoch" => epoch = resolve_attr(&attr)?,
            "ver" => version_cow = Some(resolve_attr(&attr)?),
            "rel" => release_cow = Some(resolve_attr(&attr)?),
            _ => (),
        }
    }

    let version = version_cow.ok_or(MetadataError::MissingAttributeError("ver"))?;
    let release = release_cow.ok_or(MetadataError::MissingAttributeError("rel"))?;
    Ok((epoch, version, release))
}
