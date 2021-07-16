use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use hex;
use ring::digest;

use crate::{Checksum, ChecksumType, MetadataError};

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
