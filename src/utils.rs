// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use ahash::AHashMap;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};

use digest;
use hex;
use niffler;
use quick_xml;
use sha1;
use sha2;

use crate::{Checksum, ChecksumType, CompressionType, MetadataError};

// TODO: these Box<dyn Read> shouldn't be necessary
fn get_digest<D: digest::Digest>(mut reader: Box<dyn Read>) -> Result<String, MetadataError> {
    let mut buffer = [0; 4096];
    let mut hasher = D::new();

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    Ok(hex::encode(hasher.finalize().as_ref()))
}

/// Compute a checksum of the file at `path`.
pub fn checksum_file(path: &Path, checksum_type: ChecksumType) -> Result<Checksum, MetadataError> {
    let reader = Box::new(BufReader::new(File::open(path).unwrap())) as Box<dyn Read>;

    let result = match checksum_type {
        ChecksumType::Md5 => Checksum::Md5(get_digest::<md5::Md5>(reader)?),
        ChecksumType::Sha1 => Checksum::Sha1(get_digest::<sha1::Sha1>(reader)?),
        ChecksumType::Sha224 => Checksum::Sha224(get_digest::<sha2::Sha224>(reader)?),
        ChecksumType::Sha256 => Checksum::Sha256(get_digest::<sha2::Sha256>(reader)?),
        ChecksumType::Sha384 => Checksum::Sha384(get_digest::<sha2::Sha384>(reader)?),
        ChecksumType::Sha512 => Checksum::Sha512(get_digest::<sha2::Sha512>(reader)?),
        ChecksumType::Unknown => panic!("Cannot create digest using type Checksum::Unknown"),
    };

    Ok(result)
}
// TODO: not efficient to iterate the file twice

/// Compute a checksum of the decompressed contents of a compressed file, or `None` if uncompressed.
pub fn checksum_inner_file(
    path: &Path,
    checksum_type: ChecksumType,
) -> Result<Option<Checksum>, MetadataError> {
    let (reader, format) = niffler::from_path(path)?;

    if format == niffler::Format::No {
        return Ok(None);
    }

    let result = match checksum_type {
        ChecksumType::Md5 => Checksum::Md5(get_digest::<md5::Md5>(reader)?),
        ChecksumType::Sha1 => Checksum::Sha1(get_digest::<sha1::Sha1>(reader)?),
        ChecksumType::Sha224 => Checksum::Sha224(get_digest::<sha2::Sha224>(reader)?),
        ChecksumType::Sha256 => Checksum::Sha256(get_digest::<sha2::Sha256>(reader)?),
        ChecksumType::Sha384 => Checksum::Sha384(get_digest::<sha2::Sha384>(reader)?),
        ChecksumType::Sha512 => Checksum::Sha512(get_digest::<sha2::Sha512>(reader)?),
        ChecksumType::Unknown => panic!("Cannot create digest using type Checksum::Unknown"),
    };

    Ok(Some(result))
}

/// Return the decompressed size of a compressed file, or `None` if uncompressed.
pub fn size_inner_file(path: &Path) -> Result<Option<u64>, MetadataError> {
    let (reader, format) = niffler::from_path(path)?;

    let inner_size = match format {
        niffler::Format::No => None,
        _ => Some(reader.bytes().into_iter().count() as u64),
    };

    Ok(inner_size)
}

/// Create a configured XML reader with empty-element expansion and text trimming enabled.
pub fn create_xml_reader<R: io::BufRead>(inner: R) -> quick_xml::Reader<R> {
    let mut reader = quick_xml::Reader::from_reader(inner);
    reader.config_mut().expand_empty_elements = true;
    reader.config_mut().trim_text(true);
    reader
}

/// Create an XML writer with 2-space indentation.
pub fn create_xml_writer<W: io::Write + Send>(inner: W) -> quick_xml::Writer<W> {
    quick_xml::Writer::new_with_indent(inner, b' ', 2)
}

/// Open a file and automatically decompress it based on its magic bytes.
pub fn reader_from_file(path: &Path) -> Result<Box<dyn io::Read + Send>, MetadataError> {
    let (compress_reader, _compression) = niffler::send::from_path(path)?;
    Ok(compress_reader)
}

/// Open a (possibly compressed) file and return a configured XML reader over it.
pub fn xml_reader_from_file(
    path: &Path,
) -> Result<quick_xml::Reader<BufReader<Box<dyn io::Read + Send>>>, MetadataError> {
    let compress_reader = reader_from_file(path)?;
    Ok(create_xml_reader(BufReader::new(compress_reader)))
}

// TODO: maybe split this up so that it just configures the writer, but takes a Box<dyn Write> which can be pre-configured with compression
/// Create a compressed XML writer for the given path, returning the final filename with compression suffix.
pub fn xml_writer_for_path(
    path: &Path,
    compression: CompressionType,
) -> Result<(PathBuf, quick_xml::Writer<Box<dyn io::Write + Send>>), MetadataError> {
    let (filename, inner_writer) = writer_to_file(path, compression)?;
    let writer = create_xml_writer(inner_writer);
    Ok((filename, writer))
}

/// Append the appropriate compression file extension (e.g. `.gz`, `.xz`) to a path.
pub fn apply_compression_suffix(path: &Path, compression: CompressionType) -> PathBuf {
    let extension = compression.to_file_extension();
    // TODO: easier way to do this?
    let mut filename = path.as_os_str().to_owned();
    filename.push(&extension);
    PathBuf::from(&filename)
}

/// Create a compressed writer to the given path, returning the final filename with compression suffix.
pub fn writer_to_file(
    path: &Path,
    compression: CompressionType,
) -> Result<(PathBuf, Box<dyn io::Write + Send>), MetadataError> {
    let filename = apply_compression_suffix(path, compression);
    let format = match compression {
        CompressionType::None => niffler::send::compression::Format::No,
        CompressionType::Gzip => niffler::send::compression::Format::Gzip,
        CompressionType::Xz => niffler::send::compression::Format::Lzma,
        CompressionType::Bz2 => niffler::send::compression::Format::Bzip,
        CompressionType::Zstd => niffler::send::compression::Format::Zstd,
    };
    let writer = niffler::send::to_path(&filename, format, niffler::Level::Nine)?;
    Ok((filename, writer))
}

pub(crate) const XML_VERSION: quick_xml::XmlVersion = quick_xml::XmlVersion::Implicit1_0;

pub(crate) trait XmlTextUnescape {
    fn xml_text(&self) -> Result<String, crate::MetadataError>;
}

impl XmlTextUnescape for quick_xml::events::BytesText<'_> {
    fn xml_text(&self) -> Result<String, crate::MetadataError> {
        let decoded = self.xml_content(XML_VERSION)?;
        let unescaped = quick_xml::escape::unescape(&decoded)?;
        Ok(unescaped.into_owned())
    }
}

pub(crate) trait XmlAttrUnescape {
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

/// Whether a file path is considered "primary" metadata (included in primary.xml).
pub fn is_primary_file(path: &str) -> bool {
    path.starts_with("/etc/") || path.contains("bin/") || path == "/usr/lib/sendmail"
}

/// A string interning pool that deduplicates strings and returns compact integer IDs.
///
/// Strings are stored once and referenced by `u32` index.
#[derive(Clone, Debug)]
pub struct StringPool {
    strings: Vec<String>,
    index: AHashMap<String, u32>,
}

impl Default for StringPool {
    fn default() -> Self {
        Self {
            strings: Vec::new(),
            index: AHashMap::new(),
        }
    }
}

impl StringPool {
    /// Create an empty pool.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an empty pool with pre-allocated capacity for `cap` strings.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            strings: Vec::with_capacity(cap),
            index: AHashMap::with_capacity(cap),
        }
    }

    /// Insert `s` into the pool if not already present and return its stable ID.
    pub fn intern(&mut self, s: &str) -> u32 {
        if let Some(&id) = self.index.get(s) {
            return id;
        }
        let id = self.strings.len() as u32;
        let owned = s.to_owned();
        self.index.insert(owned.clone(), id);
        self.strings.push(owned);
        id
    }

    /// Look up a previously interned string by its ID.
    ///
    /// # Panics
    ///
    /// Panics if `id` was not returned by a prior call to [`intern`](Self::intern).
    pub fn resolve(&self, id: u32) -> &str {
        &self.strings[id as usize]
    }

    /// Return the number of unique strings in the pool.
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Return `true` if the pool contains no strings.
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}

/// Interned directory path ID, referencing a [`StringPool`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DirId(u32);

impl DirId {
    pub(crate) fn new(id: u32) -> Self {
        Self(id)
    }

    pub(crate) fn as_u32(self) -> u32 {
        self.0
    }
}

