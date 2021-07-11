extern crate rpmrepo_metadata;

use rpmrepo_metadata::{MetadataError, Repository};
mod common;

// #[test]
// fn repomd() -> Result<(), MetadataError> {
//     // let fixture_path = "./tests/assets/complex_repo/";
//     let fixture_path = "../test_repo/";

//     let repo = Repository::load_from_directory(fixture_path.as_ref())?;

//     assert_eq!(repo.packages().len(), 10700);
//     // assert_eq!(repo.packages.len(), 3);

//     // repo.to_directory("./tests/assets/test_repo/".as_ref())?;

//     Ok(())
// }

// #[test]
// fn centos7() -> Result<(), DeError> {
//     let xml = include_str!("assets/centos7/os/x86_64/repodata/repomd.xml");
//     let repomd: RepoMd = from_str(xml)?;
//     assert_eq!(&repomd.revision, "1587512243");
//     Ok(())
// }

// #[test]
// fn centos8_appstream() -> Result<(), DeError> {
//     let xml = include_str!("assets/centos8/AppStream/x86_64/os/repodata/repomd.xml");
//     let repomd: RepoMd = from_str(xml)?;
//     assert_eq!(&repomd.revision, "8.2.2004");
//     Ok(())
// }

// #[test]
// fn centos8_baseos() -> Result<(), DeError> {
//     let xml = include_str!("assets/centos8/BaseOS/x86_64/os/repodata/repomd.xml");
//     let repomd: RepoMd = from_str(xml)?;
//     assert_eq!(&repomd.revision, "8.2.2004");
//     Ok(())
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::metadata::MetadataIO;
//     use once_cell::sync::OnceCell;
//     use pretty_assertions::assert_eq;
//     use std::path::Path;

//     const FIXTURE_REPOMD_PATH: &str = "./tests/assets/complex_repo/repodata/repomd.xml";

//     /// Fixture should cover all fields / tag types for repomd.xml
//     /// Started w/ Fedora 33 updates repodata, added repo, content, distro tags
//     /// FilelistsDb covers standard fields + database_version, UpdateInfoZck covers header_size, header_checksum
//     fn fixture_data() -> &'static RepoMd {
//         static INSTANCE: OnceCell<RepoMd> = OnceCell::new();
//         INSTANCE.get_or_init(|| {
//             let mut repomd = RepoMd::default();
//             repomd.set_revision(String::from("1615686706"));
//             repomd.add_repo_tag(String::from("Fedora"));
//             repomd.add_repo_tag(String::from("Fedora-Updates"));
//             repomd.add_content_tag(String::from("binary-x86_64"));
//             repomd.add_distro_tag(
//                 String::from("Fedora 33"),
//                 Some(String::from("cpe:/o:fedoraproject:fedora:33")),
//             );
//             repomd
//         })
//     }

//     /// Test deserialization of repomd with full coverage of all fields of RepoMd and RepoMdRecord
//     #[test]
//     fn test_deserialization() -> Result<(), MetadataError> {
//         let actual = &RepoMd::from_file(Path::new(FIXTURE_REPOMD_PATH))?;
//         let expected = fixture_data();

//         assert_eq!(actual.revision(), expected.revision());
//         assert_eq!(actual.repo_tags(), expected.repo_tags());
//         assert_eq!(actual.content_tags(), expected.content_tags());
//         assert_eq!(actual.distro_tags(), expected.distro_tags());

//         // TODO
//         // assert_eq!(
//         //     actual.get_record("filelists_db"),
//         //     expected.get_record("filelists_db")
//         // );
//         // assert_eq!(
//         //     actual.get_record("updateinfo_zck"),
//         //     expected.get_record("updateinfo_zck")
//         // );

//         // assert_eq!(actual.records().len(), 17);
//         // let expected_types = vec![
//         //     "primary",
//         //     "filelists",
//         //     "other",
//         //     "primary_db",
//         //     "filelists_db",
//         //     "other_db",
//         //     "primary_zck",
//         //     "filelists_zck",
//         //     "other_zck",
//         // ];
//         // let actual_types = actual
//         //     .records()
//         //     .iter()
//         //     .map(|r| r.mdtype.as_str())
//         //     .collect::<Vec<&str>>();
//         // assert_eq!(actual_types, expected_types);

//         Ok(())
//     }

//     // /// Test Serialization on a real repomd.xml (Fedora 33 x86_64 release "everything")
//     // #[test]
//     // fn test_serialization() -> Result<(), MetadataError> {
//     //     let actual = fixture_data().to_string()?;

//     //     let mut expected = String::new();
//     //     File::open(FIXTURE_REPOMD_PATH)?.read_to_string(&mut expected)?;

//     //     assert_eq!(&expected, &actual);

//     //     Ok(())
//     // }

//     /// Test roundtrip (serialize + deserialize) on a real repomd.xml (Fedora 33 x86_64 release "everything")
//     #[test]
//     fn test_roundtrip() -> Result<(), MetadataError> {
//         let first_deserialize = RepoMd::from_file(Path::new(FIXTURE_REPOMD_PATH))?;
//         let first_serialize = first_deserialize.to_string()?;
//         let second_deserialize = RepoMd::from_str(&first_serialize)?;
//         let second_serialize = second_deserialize.to_string()?;

//         assert_eq!(first_deserialize, second_deserialize);
//         assert_eq!(first_serialize, second_serialize);

//         Ok(())
//     }
// }
