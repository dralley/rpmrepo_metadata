use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use hex;
use quick_xml;
use ring::digest;

use crate::{Checksum, ChecksumType, CompressionType, MetadataError};

pub fn checksum_file(path: &Path, checksum_type: ChecksumType) -> Result<Checksum, MetadataError> {
    let reader = &mut BufReader::new(File::open(path).unwrap());

    let mut context = match checksum_type {
        ChecksumType::Sha1 => digest::Context::new(&digest::SHA1_FOR_LEGACY_USE_ONLY),
        ChecksumType::Sha256 => digest::Context::new(&digest::SHA256),
        ChecksumType::Sha384 => digest::Context::new(&digest::SHA384),
        ChecksumType::Sha512 => digest::Context::new(&digest::SHA512),
        ChecksumType::Unknown => panic!("Cannot create digest using type Checksum::Unknown"),
    };
    let mut buffer = [0; 4096];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        context.update(&buffer[..count]);
    }
    let digest = context.finish();
    let checksum = hex::encode(digest.as_ref());
    let result = match checksum_type {
        ChecksumType::Sha1 => Checksum::Sha1(checksum),
        ChecksumType::Sha256 => Checksum::Sha256(checksum),
        ChecksumType::Sha384 => Checksum::Sha384(checksum),
        ChecksumType::Sha512 => Checksum::Sha512(checksum),
        ChecksumType::Unknown => panic!(),
    };
    Ok(result)
}
// TODO: clean this up, deduplicate
// TODO: not efficient to iterate the file twice

pub fn checksum_inner_file(
    path: &Path,
    checksum_type: ChecksumType,
) -> Result<Option<Checksum>, MetadataError> {
    let (mut reader, format) = niffler::from_path(path)?;

    if format == niffler::Format::No {
        return Ok(None);
    }

    let mut context = match checksum_type {
        ChecksumType::Sha1 => digest::Context::new(&digest::SHA1_FOR_LEGACY_USE_ONLY),
        ChecksumType::Sha256 => digest::Context::new(&digest::SHA256),
        ChecksumType::Sha384 => digest::Context::new(&digest::SHA384),
        ChecksumType::Sha512 => digest::Context::new(&digest::SHA512),
        ChecksumType::Unknown => panic!("Cannot create digest using type Checksum::Unknown"),
    };
    let mut buffer = [0; 4096];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        context.update(&buffer[..count]);
    }
    let digest = context.finish();
    let checksum = hex::encode(digest.as_ref());
    let result = match checksum_type {
        ChecksumType::Sha1 => Checksum::Sha1(checksum),
        ChecksumType::Sha256 => Checksum::Sha256(checksum),
        ChecksumType::Sha384 => Checksum::Sha384(checksum),
        ChecksumType::Sha512 => Checksum::Sha512(checksum),
        ChecksumType::Unknown => panic!(),
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

pub(crate) fn configure_xml_reader<R: BufRead>(reader: &mut quick_xml::Reader<R>) {
    reader.expand_empty_elements(true).trim_text(true);
}

pub fn xml_reader_from_path(
    path: &Path,
) -> Result<quick_xml::Reader<BufReader<Box<dyn std::io::Read>>>, MetadataError> {
    let file = File::open(path)?;
    let (compression_wrapper_reader, _compression) = niffler::get_reader(Box::new(file))?;
    let mut xml_reader = quick_xml::Reader::from_reader(BufReader::new(compression_wrapper_reader));
    configure_xml_reader(&mut xml_reader);
    Ok(xml_reader)
}

// TODO: maybe split this up so that it just configures the writer, but takes a Box<dyn Write> which can be pre-configured with compression
pub fn create_xml_writer(
    path: &Path,
    compression: CompressionType,
) -> Result<(PathBuf, quick_xml::Writer<Box<dyn Write>>), MetadataError> {
    let extension = compression.to_file_extension();

    // TODO: easier way to do this?
    let mut filename = path.as_os_str().to_owned();
    filename.push(&extension);
    let filename = PathBuf::from(&filename);

    let file = BufWriter::new(File::create(&filename)?);

    let inner_writer = match compression {
        CompressionType::None => Box::new(file),
        CompressionType::Gzip => niffler::get_writer(
            Box::new(file),
            niffler::compression::Format::Gzip,
            niffler::Level::Nine,
        )?,
        CompressionType::Bz2 => niffler::get_writer(
            Box::new(file),
            niffler::compression::Format::Bzip,
            niffler::Level::Nine,
        )?,
        CompressionType::Xz => niffler::get_writer(
            Box::new(file),
            niffler::compression::Format::Lzma,
            niffler::Level::Nine,
        )?,
        _ => unimplemented!(),
    };
    let writer = quick_xml::Writer::new_with_indent(inner_writer, b' ', 2);
    Ok((filename, writer))
}
