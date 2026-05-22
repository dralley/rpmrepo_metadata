use quick_xml;

pub(crate) const XML_VERSION: quick_xml::XmlVersion = quick_xml::XmlVersion::Implicit1_0;

/// Names of records in the metadata
pub mod mdrecord {
    pub const MD_PRIMARY: &str = "primary";
    pub const MD_FILELISTS: &str = "filelists";
    pub const MD_OTHER: &str = "other";
    pub const MD_UPDATEINFO: &str = "updateinfo";
    pub const MD_GROUP: &str = "group";
    pub const MD_GROUP_GZ: &str = "group_gz";
    pub const MD_GROUP_XZ: &str = "group_xz";
}

pub(crate) mod xmlns {
    /// Default namespace for primary.xml
    pub const NS_COMMON: &str = "http://linux.duke.edu/metadata/common";
    /// Default namespace for filelists.xml
    pub const NS_FILELISTS: &str = "http://linux.duke.edu/metadata/filelists";
    /// Default namespace for other.xml
    pub const NS_OTHER: &str = "http://linux.duke.edu/metadata/other";
    /// Default namespace for repomd.xml
    pub const NS_REPO: &str = "http://linux.duke.edu/metadata/repo";
    /// Namespace for rpm (used in primary.xml and repomd.xml)
    pub const NS_RPM: &str = "http://linux.duke.edu/metadata/rpm";
}

// ── Tag constants ───────────────────────────────────────────────────────

pub(crate) mod tag {
    pub const TAG_METADATA: &str = "metadata";
    pub const TAG_PACKAGE: &str = "package";
    pub const TAG_NAME: &str = "name";
    pub const TAG_VERSION: &str = "version";
    pub const TAG_CHECKSUM: &str = "checksum";
    pub const TAG_ARCH: &str = "arch";
    pub const TAG_SUMMARY: &str = "summary";
    pub const TAG_DESCRIPTION: &str = "description";
    pub const TAG_PACKAGER: &str = "packager";
    pub const TAG_URL: &str = "url";
    pub const TAG_TIME: &str = "time";
    pub const TAG_SIZE: &str = "size";
    pub const TAG_LOCATION: &str = "location";
    pub const TAG_FORMAT: &str = "format";
    pub const TAG_RPM_LICENSE: &str = "rpm:license";
    pub const TAG_RPM_VENDOR: &str = "rpm:vendor";
    pub const TAG_RPM_GROUP: &str = "rpm:group";
    pub const TAG_RPM_BUILDHOST: &str = "rpm:buildhost";
    pub const TAG_RPM_SOURCERPM: &str = "rpm:sourcerpm";
    pub const TAG_RPM_HEADER_RANGE: &str = "rpm:header-range";
    pub const TAG_RPM_ENTRY: &str = "rpm:entry";
    pub const TAG_RPM_PROVIDES: &str = "rpm:provides";
    pub const TAG_RPM_REQUIRES: &str = "rpm:requires";
    pub const TAG_RPM_CONFLICTS: &str = "rpm:conflicts";
    pub const TAG_RPM_OBSOLETES: &str = "rpm:obsoletes";
    pub const TAG_RPM_SUGGESTS: &str = "rpm:suggests";
    pub const TAG_RPM_ENHANCES: &str = "rpm:enhances";
    pub const TAG_RPM_RECOMMENDS: &str = "rpm:recommends";
    pub const TAG_RPM_SUPPLEMENTS: &str = "rpm:supplements";
    pub const TAG_FILE: &str = "file";
    pub const TAG_FILELISTS: &str = "filelists";
    pub const TAG_OTHER: &str = "otherdata";
    pub const TAG_CHANGELOG: &str = "changelog";

    // Updateinfo constants
    pub const TAG_UPDATES: &str = "updates";
    pub const TAG_UPDATE: &str = "update";
    pub const TAG_ID: &str = "id";
    pub const TAG_TITLE: &str = "title";
    pub const TAG_RELEASE: &str = "release";
    pub const TAG_SEVERITY: &str = "severity";
    pub const TAG_ISSUED: &str = "issued";
    pub const TAG_UPDATED: &str = "updated";
    pub const TAG_RIGHTS: &str = "rights";
    pub const TAG_SOLUTION: &str = "solution";
    pub const TAG_PUSHCOUNT: &str = "pushcount";
    pub const TAG_REFERENCES: &str = "references";
    pub const TAG_REFERENCE: &str = "reference";
    pub const TAG_PKGLIST: &str = "pkglist";
    pub const TAG_COLLECTION: &str = "collection";
    pub const TAG_MODULE: &str = "module";
    pub const TAG_FILENAME: &str = "filename";
    pub const TAG_SUM: &str = "sum";
    pub const TAG_REBOOT_SUGGESTED: &str = "reboot_suggested";
    pub const TAG_RESTART_SUGGESTED: &str = "restart_suggested";
    pub const TAG_RELOGIN_SUGGESTED: &str = "relogin_suggested";

    // Comps constants
    pub const TAG_COMPS: &str = "comps";
    pub const TAG_GROUP: &str = "group";
    pub const TAG_CATEGORY: &str = "category";
    pub const TAG_ENVIRONMENT: &str = "environment";
    pub const TAG_DEFAULT: &str = "default";
    pub const TAG_USERVISIBLE: &str = "uservisible";
    pub const TAG_BIARCHONLY: &str = "biarchonly";
    pub const TAG_LANGONLY: &str = "langonly";
    pub const TAG_DISPLAY_ORDER: &str = "display_order";
    pub const TAG_PACKAGELIST: &str = "packagelist";
    pub const TAG_PACKAGEREQ: &str = "packagereq";
    pub const TAG_GROUPLIST: &str = "grouplist";
    pub const TAG_OPTIONLIST: &str = "optionlist";
    pub const TAG_GROUPID: &str = "groupid";
    pub const TAG_LANGPACKS: &str = "langpacks";
    pub const TAG_MATCH: &str = "match";
}
