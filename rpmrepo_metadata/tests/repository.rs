use pretty_assertions::assert_eq;
use rpmrepo_metadata::{
    MetadataError, Package, Repository, RepositoryOptions, RepositoryReader, RepositoryWriter,
};
use tempdir::TempDir;
mod common;

static COMPLEX_REPO_PATH: &str = "./tests/assets/fixture_repos/complex_repo/";

#[test]
fn test_repository_complex_repo() -> Result<(), MetadataError> {
    use pretty_assertions::assert_eq;
    let repo = Repository::load_from_directory(COMPLEX_REPO_PATH.as_ref())?;

    assert_eq!(repo.packages().len(), 4);

    assert_eq!(
        repo.packages()
            .get(common::COMPLEX_PACKAGE.pkgid())
            .unwrap(),
        &*common::COMPLEX_PACKAGE
    );
    assert_eq!(
        repo.packages().get(common::RPM_EMPTY.pkgid()).unwrap(),
        &*common::RPM_EMPTY
    );
    assert_eq!(
        repo.packages()
            .get(common::RPM_WITH_INVALID_CHARS.pkgid())
            .unwrap(),
        &*common::RPM_WITH_INVALID_CHARS
    );
    assert_eq!(
        repo.packages()
            .get(common::RPM_WITH_NON_ASCII.pkgid())
            .unwrap(),
        &*common::RPM_WITH_NON_ASCII
    );

    // repo.to_directory("./tests/assets/test_repo/".as_ref())?;

    Ok(())
}

#[test]
fn test_repository_reader_iter_packages() -> Result<(), MetadataError> {
    use pretty_assertions::assert_eq;

    let repo = RepositoryReader::new_from_directory(COMPLEX_REPO_PATH.as_ref())?;
    let mut package_iter = repo.iter_packages()?;

    assert_eq!(package_iter.remaining_packages(), 4);
    assert_eq!(package_iter.total_packages(), 4);

    assert_eq!(
        &package_iter.parse_package()?.unwrap(),
        &*common::COMPLEX_PACKAGE
    );
    assert_eq!(&package_iter.parse_package()?.unwrap(), &*common::RPM_EMPTY);
    assert_eq!(
        &package_iter.parse_package()?.unwrap(),
        &*common::RPM_WITH_INVALID_CHARS
    );
    assert_eq!(
        &package_iter.parse_package()?.unwrap(),
        &*common::RPM_WITH_NON_ASCII
    );

    assert_eq!(package_iter.remaining_packages(), 0);
    assert_eq!(package_iter.total_packages(), 4);

    Ok(())
}

#[test]
fn test_read_write_uncompressed() -> Result<(), MetadataError> {
    let tmp_dir = TempDir::new("test_repository_writer")?;
    let test_repodata_dir = tmp_dir.path().join("repodata");

    let options = RepositoryOptions::default()
        .metadata_checksum_type(rpmrepo_metadata::ChecksumType::Sha1)
        .metadata_compression_type(rpmrepo_metadata::CompressionType::None);
    let mut repo_writer = RepositoryWriter::new_with_options(&tmp_dir.path(), 1, options)?;
    repo_writer.add_package(&*common::COMPLEX_PACKAGE)?;
    repo_writer.finish()?;

    assert!(
        test_repodata_dir.exists(),
        "A repodata/ directory wasn't created for the repo"
    );
    assert!(
        test_repodata_dir.join("primary.xml").exists(),
        "primary.xml is missing"
    );
    assert!(
        test_repodata_dir.join("filelists.xml").exists(),
        "filelists.xml is missing"
    );
    assert!(
        test_repodata_dir.join("other.xml").exists(),
        "other.xml is missing"
    );

    let repo = Repository::load_from_directory(&tmp_dir.path())?;
    let mut packages_iter = repo.packages().iter().map(|(_, p)| p);

    assert_eq!(packages_iter.next(), Some(&*common::COMPLEX_PACKAGE));

    Ok(())
}

#[test]
fn test_read_write_xz_compressed() -> Result<(), MetadataError> {
    let tmp_dir = TempDir::new("test_repository_writer")?;
    let test_repodata_dir = tmp_dir.path().join("repodata");

    let options = RepositoryOptions::default()
        .metadata_checksum_type(rpmrepo_metadata::ChecksumType::Sha1)
        .metadata_compression_type(rpmrepo_metadata::CompressionType::Xz);
    let mut repo_writer = RepositoryWriter::new_with_options(&tmp_dir.path(), 1, options)?;
    repo_writer.add_package(&*common::COMPLEX_PACKAGE)?;
    repo_writer.finish()?;

    assert!(
        test_repodata_dir.exists(),
        "A repodata/ directory wasn't created for the repo"
    );
    assert!(
        test_repodata_dir.join("primary.xml.xz").exists(),
        "primary.xml.xz is missing"
    );
    assert!(
        test_repodata_dir.join("filelists.xml.xz").exists(),
        "filelists.xml.xz is missing"
    );
    assert!(
        test_repodata_dir.join("other.xml.xz").exists(),
        "other.xml.xz is missing"
    );

    let repo = Repository::load_from_directory(&tmp_dir.path())?;
    let mut packages_iter = repo.packages().iter().map(|(_, p)| p);

    assert_eq!(packages_iter.next(), Some(&*common::COMPLEX_PACKAGE));

    Ok(())
}

#[test]
fn test_read_write_bz2_compressed() -> Result<(), MetadataError> {
    let tmp_dir = TempDir::new("test_repository_writer")?;
    let test_repodata_dir = tmp_dir.path().join("repodata");

    let options = RepositoryOptions::default()
        .metadata_checksum_type(rpmrepo_metadata::ChecksumType::Sha1)
        .metadata_compression_type(rpmrepo_metadata::CompressionType::Bz2);
    let mut repo_writer = RepositoryWriter::new_with_options(&tmp_dir.path(), 1, options)?;
    repo_writer.add_package(&*common::COMPLEX_PACKAGE)?;
    repo_writer.finish()?;

    assert!(
        test_repodata_dir.exists(),
        "A repodata/ directory wasn't created for the repo"
    );
    assert!(
        test_repodata_dir.join("primary.xml.bz2").exists(),
        "primary.xml.bz2 is missing"
    );
    assert!(
        test_repodata_dir.join("filelists.xml.bz2").exists(),
        "filelists.xml.bz2 is missing"
    );
    assert!(
        test_repodata_dir.join("other.xml.bz2").exists(),
        "other.xml.bz2 is missing"
    );

    let repo = Repository::load_from_directory(&tmp_dir.path())?;
    let mut packages_iter = repo.packages().iter().map(|(_, p)| p);

    assert_eq!(packages_iter.next(), Some(&*common::COMPLEX_PACKAGE));
    Ok(())
}

// TODO: these tests need to be specific about what is panicking

#[test]
#[should_panic]
fn test_repository_writer_not_enough_packages() {
    let tmp_dir = TempDir::new("test_repository_writer").unwrap();

    let mut repo_writer = RepositoryWriter::new(&tmp_dir.path(), 1).unwrap();
    repo_writer.finish().unwrap();
}

#[test]
#[should_panic]
fn test_repository_writer_too_many_packages() {
    let tmp_dir = TempDir::new("test_repository_writer").unwrap();

    let mut repo_writer = RepositoryWriter::new(&tmp_dir.path(), 0).unwrap();
    repo_writer.add_package(&*common::COMPLEX_PACKAGE).unwrap();
    repo_writer.finish().unwrap();
}
