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

    static FIXTURE_REPOMD_PATH: &str =
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
            record.checksum = Checksum::Sha256(String::from(
                "e6104a05bf3101c01321a5af9098d569ff974a8e6a8f72c5982bf074efbaf036",
            ));
            record.open_checksum = Some(Checksum::Sha256(String::from(
                "03fb79ab50c4ac35db2ca86964047c68a3561e0978e380be7f4fbc0ac4d6c530",
            )));
            record.timestamp = 1639195237;
            record.size = Some(1971);
            record.open_size = Some(6527);
            record.location_href = PathBuf::from("repodata/primary.xml.gz");
            repomd.add_record(record);
            let mut record = RepomdRecord::default();
            record.metadata_name = String::from("filelists");
            record.checksum = Checksum::Sha256(String::from(
                "128398aea7338ada2735e3d9340c16e5915040133b77bd8f4498d22ace6e5a0e",
            ));
            record.open_checksum = Some(Checksum::Sha256(String::from(
                "4077fb59f51db93dc3414850564d18e8ccd4ae6acb8358272e174a84d1b1ba1e",
            )));
            record.timestamp = 1639195237;
            record.size = Some(524);
            record.open_size = Some(1099);
            record.location_href = PathBuf::from("repodata/filelists.xml.gz");
            repomd.add_record(record);
            let mut record = RepomdRecord::default();
            record.metadata_name = String::from("other");
            record.checksum = Checksum::Sha256(String::from(
                "9b34aaa221ed94e916f385c0b891c0114c394948140d736bd10ec5127c2ea4e5",
            ));
            record.open_checksum = Some(Checksum::Sha256(String::from(
                "1851cd11e50372c89303851655ccc032b35468854e6c7401eb02d31fd7e77a6e",
            )));
            record.timestamp = 1639195237;
            record.size = Some(680);
            record.open_size = Some(1277);
            record.location_href = PathBuf::from("repodata/other.xml.gz");
            repomd.add_record(record);
            repomd
        })
    }

    #[test]
    fn test_deserialization() -> Result<(), MetadataError> {
        let actual =
            RepomdXml::read_data(utils::xml_reader_from_file(Path::new(FIXTURE_REPOMD_PATH))?)?;
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

    /// Test Serialization on a real repomd.xml
    #[test]
    fn test_serialization() -> Result<(), MetadataError> {
        let mut buffer = Vec::new();
        RepomdXml::write_data(fixture_data(), &mut utils::create_xml_writer(&mut buffer))?;
        let actual = std::str::from_utf8(&buffer)?;
        let mut expected = String::new();
        File::open(FIXTURE_REPOMD_PATH)?.read_to_string(&mut expected)?;

        assert_eq!(&expected, &actual);

        Ok(())
    }

    /// Test roundtrip (serialize + deserialize) on a real repomd.xml
    #[test]
    fn test_roundtrip() -> Result<(), MetadataError> {
        let mut first_buffer = Vec::new();
        let mut second_buffer = Vec::new();
        let first_repomd =
            RepomdXml::read_data(utils::xml_reader_from_file(Path::new(FIXTURE_REPOMD_PATH))?)?;
        RepomdXml::write_data(
            &first_repomd,
            &mut utils::create_xml_writer(&mut first_buffer),
        )?;
        let second_repomd = RepomdXml::read_data(utils::create_xml_reader(&*first_buffer))?;
        RepomdXml::write_data(
            &second_repomd,
            &mut utils::create_xml_writer(&mut second_buffer),
        )?;

        assert_eq!(first_repomd, second_repomd);
        assert_eq!(first_buffer, second_buffer);

        Ok(())
    }
}
