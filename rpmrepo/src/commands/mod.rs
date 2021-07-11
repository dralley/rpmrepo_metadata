pub mod create;
// pub mod download;
// pub mod sync;

use anyhow::Result;
use std::ffi::{OsStr, OsString};

use argh::FromArgs;

pub fn handle_command() -> Result<()> {
    let execution_config: RpmRepoExecConfig = argh::from_env();

    match execution_config.subcommand {
        // Subcommands::Download(c) => download::download(c),
        Subcommands::Create(c) => create::create(c),
        // Subcommands::Sync(c) => sync::sync(c),
    }
}

#[derive(FromArgs, PartialEq, Debug)]
/// Top-level command.
pub struct RpmRepoExecConfig {
    #[argh(subcommand)]
    subcommand: Subcommands,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum Subcommands {
    // Download(DownloadCommand),
    Create(CreateCommand),
    // Sync(SyncCommand),
    // Modify(ModifyCommand),
    // Merge(MergeCommand),
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "download")]
/// Download a repository
pub struct DownloadCommand {
    /// the URL of the repository to download
    #[argh(positional)]
    url: String,

    /// how many files can be downloaded in parallel
    #[argh(option)]
    concurrency: Option<u8>,

    /// specify a TLS CA cert location (if not present in system trust store)
    #[argh(option)]
    tls_ca_cert: Option<String>,

    /// specify a TLS client cert location (.pem, .crt, .cert)
    #[argh(option)]
    tls_client_cert: Option<String>,

    /// specify a TLS client key location (.pem, .key). If not provided, value of client_cert will be checked
    #[argh(option)]
    tls_client_cert_key: Option<String>,

    /// disable TLS server certificate verification
    #[argh(switch)]
    no_check_certificate: bool,

    /// directory containing RPMs
    #[argh(positional)]
    destination: OsString,

    /// download metadata only
    #[argh(switch)]
    only_metadata: bool,

    /// re-use existing metadata, only download
    #[argh(switch)]
    update: bool,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "sync")]
/// Sync all system-enabled repositories
pub struct SyncCommand {
    /// individual name(s) of repository(ies) to download
    #[argh(option)]
    name: Vec<String>,

    /// how many files can be downloaded in parallel
    #[argh(option)]
    concurrency: Option<u8>,

    /// path to a directory where .repo files are located
    #[argh(option)]
    reposdir: Option<OsString>,

    /// re-use existing metadata, only download
    #[argh(switch)]
    update: bool,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Create a new repository
#[argh(subcommand, name = "create")]
pub struct CreateCommand {
    /// directory containing RPMs
    #[argh(positional)]
    destination: OsString,

    /// distro tag with optional cpeid: --distro "name,cpeid"
    #[argh(option)]
    distro_tag: Option<String>,

    /// tags that describe the content in the repository
    #[argh(option)]
    content_tags: Option<String>,

    /// tags that describe the repository
    #[argh(option)]
    repo_tags: Option<String>,

    /// metadata compression type
    #[argh(option)]
    metadata_compression_type: Option<String>,

    /// metadata checksum type
    #[argh(option)]
    metadata_checksum_type: Option<String>,

    /// package checksum type
    #[argh(option)]
    package_checksum_type: Option<String>,

    /// path to a list of RPM packages to add to the repo
    #[argh(option)]
    add_package_list: Option<String>,
}
