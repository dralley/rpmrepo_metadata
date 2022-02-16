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

pub fn checksum_file(path: &Path, checksum_type: ChecksumType) -> Result<Checksum, MetadataError> {
    let mut reader = Box::new(BufReader::new(File::open(path).unwrap())) as Box<dyn Read>;

    let result = match checksum_type {
        ChecksumType::Sha1 => Checksum::Sha1(get_digest::<sha1::Sha1>(reader)?),
        ChecksumType::Sha256 => Checksum::Sha256(get_digest::<sha2::Sha256>(reader)?),
        ChecksumType::Sha384 => Checksum::Sha384(get_digest::<sha2::Sha384>(reader)?),
        ChecksumType::Sha512 => Checksum::Sha512(get_digest::<sha2::Sha512>(reader)?),
        ChecksumType::Unknown => panic!("Cannot create digest using type Checksum::Unknown"),
    };

    Ok(result)
}
// TODO: not efficient to iterate the file twice

pub fn checksum_inner_file(
    path: &Path,
    checksum_type: ChecksumType,
) -> Result<Option<Checksum>, MetadataError> {
    let (mut reader, format) = niffler::from_path(path)?;

    if format == niffler::Format::No {
        return Ok(None);
    }

    let result = match checksum_type {
        ChecksumType::Sha1 => Checksum::Sha1(get_digest::<sha1::Sha1>(reader)?),
        ChecksumType::Sha256 => Checksum::Sha256(get_digest::<sha2::Sha256>(reader)?),
        ChecksumType::Sha384 => Checksum::Sha384(get_digest::<sha2::Sha384>(reader)?),
        ChecksumType::Sha512 => Checksum::Sha512(get_digest::<sha2::Sha512>(reader)?),
        ChecksumType::Unknown => panic!("Cannot create digest using type Checksum::Unknown"),
    };

    Ok(Some(result))
}

pub fn size_inner_file(path: &Path) -> Result<Option<u64>, MetadataError> {
    let (reader, format) = niffler::from_path(path)?;

    let inner_size = match format {
        niffler::Format::No => None,
        _ => Some(reader.bytes().into_iter().count() as u64),
    };

    Ok(inner_size)
}

pub fn create_xml_reader<R: io::BufRead>(inner: R) -> quick_xml::Reader<R> {
    let mut reader = quick_xml::Reader::from_reader(inner);
    reader.expand_empty_elements(true).trim_text(true);
    reader
}

pub fn create_xml_writer<W: io::Write + Send>(inner: W) -> quick_xml::Writer<W> {
    quick_xml::Writer::new_with_indent(inner, b' ', 2)
}

pub fn reader_from_file(path: &Path) -> Result<Box<dyn io::Read + Send>, MetadataError> {
    let (compress_reader, _compression) = niffler::send::from_path(path)?;
    Ok(compress_reader)
}

pub fn xml_reader_from_file(
    path: &Path,
) -> Result<quick_xml::Reader<BufReader<Box<dyn io::Read + Send>>>, MetadataError> {
    let compress_reader = reader_from_file(path)?;
    Ok(create_xml_reader(BufReader::new(compress_reader)))
}

// TODO: maybe split this up so that it just configures the writer, but takes a Box<dyn Write> which can be pre-configured with compression
pub fn xml_writer_for_path(
    path: &Path,
    compression: CompressionType,
) -> Result<(PathBuf, quick_xml::Writer<Box<dyn io::Write + Send>>), MetadataError> {
    let (filename, inner_writer) = writer_to_file(path, compression)?;
    let writer = create_xml_writer(inner_writer);
    Ok((filename, writer))
}

pub fn apply_compression_suffix(path: &Path, compression: CompressionType) -> PathBuf {
    let extension = compression.to_file_extension();
    // TODO: easier way to do this?
    let mut filename = path.as_os_str().to_owned();
    filename.push(&extension);
    PathBuf::from(&filename)
}

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
    };
    let writer = niffler::send::to_path(&filename, format, niffler::Level::Nine)?;
    Ok((filename, writer))
}
