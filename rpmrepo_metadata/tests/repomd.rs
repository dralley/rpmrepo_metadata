use std::fs::File;

use rpmrepo_metadata::{utils, MetadataError, RepomdData, RepomdRecord, RepomdXml};

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::OnceCell;
    use pretty_assertions::assert_eq;
    use rpmrepo_metadata::Checksum;
    use std::{
        io::Read,
        path::{Path, PathBuf},
    };

    const FIXTURE_REPOMD_PATH: &str =
        "./tests/assets/fixture_repos/complex_repo/repodata/repomd.xml";

    /// Fixture should cover all fields / tag types for repomd.xml
    /// Started w/ Fedora 33 updates repodata, added repo, content, distro tags
    /// FilelistsDb covers standard fields + database_version, UpdateInfoZck covers header_size, header_checksum
    fn fixture_data() -> &'static RepomdData {
        static INSTANCE: OnceCell<RepomdData> = OnceCell::new();
        INSTANCE.get_or_init(|| {
            let mut repomd = RepomdData::default();
            repomd.set_revision("1615686706");
            repomd.add_repo_tag(String::from("Fedora"));
            repomd.add_repo_tag(String::from("Fedora-Updates"));
            repomd.add_content_tag(String::from("binary-x86_64"));
            repomd.add_distro_tag(
                String::from("Fedora 33"),
                Some(String::from("cpe:/o:fedoraproject:fedora:33")),
            );
            let mut record = RepomdRecord::default();
            record.metadata_name = String::from("primary");
            record.location_href = PathBuf::from("repomd/primary.xml");
            repomd.add_record(record);
            let mut record = RepomdRecord::default();
            record.metadata_name = String::from("filelists");
            record.location_href = PathBuf::from("repomd/filelists.xml");
            repomd.add_record(record);
            let mut record = RepomdRecord::default();
            record.metadata_name = String::from("other");
            record.location_href = PathBuf::from("repomd/other.xml");
            repomd.add_record(record);
            repomd
        })
    }

    #[test]
    fn test_deserialization() -> Result<(), MetadataError> {
        let actual = RepomdXml::read_data(utils::xml_reader_from_file(Path::new(FIXTURE_REPOMD_PATH))?)?;
        let expected = fixture_data();

        assert_eq!(actual.revision(), expected.revision());
        assert_eq!(actual.repo_tags(), expected.repo_tags());
        assert_eq!(actual.content_tags(), expected.content_tags());
        assert_eq!(actual.distro_tags(), expected.distro_tags());

        // TODO
        // assert_eq!(
        //     actual.get_record("filelists_db"),
        //     expected.get_record("filelists_db")
        // );
        // assert_eq!(
        //     actual.get_record("updateinfo_zck"),
        //     expected.get_record("updateinfo_zck")
        // );

        // assert_eq!(actual.records().len(), 17);
        // let expected_types = vec![
        //     "primary",
        //     "filelists",
        //     "other",
        //     "primary_db",
        //     "filelists_db",
        //     "other_db",
        //     "primary_zck",
        //     "filelists_zck",
        //     "other_zck",
        // ];
        // let actual_types = actual
        //     .records()
        //     .iter()
        //     .map(|r| r.mdtype.as_str())
        //     .collect::<Vec<&str>>();
        // assert_eq!(actual_types, expected_types);

        Ok(())
    }

    /// Test Serialization on a real repomd.xml (Fedora 33 x86_64 release "everything")
    #[test]
    fn test_serialization() -> Result<(), MetadataError> {
        let inner = Vec::new();
        let mut writer = utils::create_xml_writer(inner);
        RepomdXml::write_data(&mut writer, fixture_data())?;
        let inner = writer.into_inner();
        let actual = std::str::from_utf8(&inner)?;
        let mut expected = String::new();
        File::open(FIXTURE_REPOMD_PATH)?.read_to_string(&mut expected)?;

        assert_eq!(&expected, &actual);

        Ok(())
    }

    // /// Test roundtrip (serialize + deserialize) on a real repomd.xml (Fedora 33 x86_64 release "everything")
    // #[test]
    // fn test_roundtrip() -> Result<(), MetadataError> {
    //     let first_deserialize = RepomdXml::from_file(Path::new(FIXTURE_REPOMD_PATH))?;
    //     let first_serialize = first_deserialize.to_string()?;
    //     let second_deserialize = RepomdXml::from_str(&first_serialize)?;
    //     let second_serialize = second_deserialize.to_string()?;

    //     assert_eq!(first_deserialize, second_deserialize);
    //     assert_eq!(first_serialize, second_serialize);

    //     Ok(())
    // }
}
