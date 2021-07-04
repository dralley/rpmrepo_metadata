extern crate rpmrepo;

use rpmrepo::metadata::{MetadataError, Package, Repository};
mod common;

#[test]
fn complex_repo() -> Result<(), MetadataError> {
    use pretty_assertions::assert_eq;

    let fixture_path = "./tests/assets/complex_repo/";

    let repo = Repository::load_from_directory(fixture_path.as_ref())?;

    assert_eq!(repo.packages().len(), 4);
    let packages: Vec<&Package> = repo.packages().into_iter().map(|(_, y)| y).collect();

    assert_eq!(packages[1], &*common::COMPLEX_PACKAGE);
    assert_eq!(packages[2], &*common::RPM_EMPTY);
    assert_eq!(packages[0], &*common::RPM_WITH_INVALID_CHARS);
    assert_eq!(packages[3], &*common::RPM_WITH_NON_ASCII);

    // repo.to_directory("./tests/assets/test_repo/".as_ref())?;

    Ok(())
}
