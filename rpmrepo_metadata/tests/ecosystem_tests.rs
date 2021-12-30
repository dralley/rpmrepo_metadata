use pretty_assertions::assert_eq;
use rpmrepo_metadata::{
    MetadataError, Package, Repository, RepositoryOptions, RepositoryReader, RepositoryWriter,
};
use tempdir::TempDir;

mod common;

#[test]
fn load_cs9_baseos_repo() -> Result<(), MetadataError> {
    let fixture_path = "./tests/assets/external_repos/cs9-baseos/";
    let repo = Repository::load_from_directory(fixture_path.as_ref())?;

    // repo.to_directory("./tests/assets/test_repo/".as_ref())?;
    Ok(())
}

// #[test]
// fn load_cs9_baseos_aarch64_repo() -> Result<(), MetadataError> {
//     let fixture_path = "./tests/assets/external_repos/cs9-baseos-aarch64/";
//     let repo = Repository::load_from_directory(fixture_path.as_ref())?;

//     // repo.to_directory("./tests/assets/test_repo/".as_ref())?;
//     Ok(())
// }

// #[test]
// fn load_cs9_baseos_ppc64le_repo() -> Result<(), MetadataError> {
//     let fixture_path = "./tests/assets/external_repos/cs9-baseos-ppc64le/";
//     let repo = Repository::load_from_directory(fixture_path.as_ref())?;

//     // repo.to_directory("./tests/assets/test_repo/".as_ref())?;
//     Ok(())
// }

// #[test]
// fn load_cs9_baseos_s390x_repo() -> Result<(), MetadataError> {
//     let fixture_path = "./tests/assets/external_repos/cs9-baseos-s390x/";
//     let repo = Repository::load_from_directory(fixture_path.as_ref())?;

//     // repo.to_directory("./tests/assets/test_repo/".as_ref())?;
//     Ok(())
// }

// #[test]
// fn load_cs9_baseos_source_repo() -> Result<(), MetadataError> {
//     let fixture_path = "./tests/assets/external_repos/cs9-baseos-src/";
//     let repo = Repository::load_from_directory(fixture_path.as_ref())?;

//     // repo.to_directory("./tests/assets/test_repo/".as_ref())?;
//     Ok(())
// }

#[test]
fn load_cs9_appstream_repo() -> Result<(), MetadataError> {
    let fixture_path = "./tests/assets/external_repos/cs9-appstream/";
    let repo = Repository::load_from_directory(fixture_path.as_ref())?;

    // repo.to_directory("./tests/assets/test_repo/".as_ref())?;
    Ok(())
}

// #[test]
// fn load_alma8_baseos_repo() -> Result<(), MetadataError> {
//     let fixture_path = "./tests/assets/external_repos/alma8-baseos/";
//     let repo = Repository::load_from_directory(fixture_path.as_ref())?;

//     // repo.to_directory("./tests/assets/test_repo/".as_ref())?;

//     Ok(())
// }

// #[test]
// fn load_alma8_appstream_repo() -> Result<(), MetadataError> {
//     let fixture_path = "./tests/assets/external_repos/alma8-appstream/";
//     let repo = Repository::load_from_directory(fixture_path.as_ref())?;

//     // repo.to_directory("./tests/assets/test_repo/".as_ref())?;

//     Ok(())
// }

// #[test]
// fn load_opensuse_tumbleweed_repo() -> Result<(), MetadataError> {
//     let fixture_path = "./tests/assets/external_repos/opensuse-tumbleweed/";
//     let repo = Repository::load_from_directory(fixture_path.as_ref())?;

//     // repo.to_directory("./tests/assets/test_repo/".as_ref())?;

//     Ok(())
// }

// #[test]
// fn load_ol7_repo() -> Result<(), MetadataError> {
//     let fixture_path = "./tests/assets/external_repos/ol7/";
//     let repo = Repository::load_from_directory(fixture_path.as_ref())?;

//     // repo.to_directory("./tests/assets/test_repo/".as_ref())?;

//     Ok(())
// }

// #[test]
// fn load_fedora35_aarch64_repo() -> Result<(), MetadataError> {
//     let fixture_path = "./tests/assets/external_repos/fedora35-aarch64/";
//     let repo = Repository::load_from_directory(fixture_path.as_ref())?;

//     // repo.to_directory("./tests/assets/test_repo/".as_ref())?;

//     Ok(())
// }

// #[test]
// fn load_centos6_repo() -> Result<(), MetadataError> {
//     let fixture_path = "./tests/assets/external_repos/centos6/";
//     let repo = Repository::load_from_directory(fixture_path.as_ref())?;

//     // repo.to_directory("./tests/assets/test_repo/".as_ref())?;

//     Ok(())
// }

// #[test]
// fn load_rpmfusion_f35_repo() -> Result<(), MetadataError> {
//     let fixture_path = "./tests/assets/external_repos/rpmfusion-f35/";
//     let repo = Repository::load_from_directory(fixture_path.as_ref())?;

//     // repo.to_directory("./tests/assets/test_repo/".as_ref())?;

//     Ok(())
// }
