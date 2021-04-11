pub mod create;
pub mod download;

use anyhow::Result;
use std::ffi::OsString;

use argh::FromArgs;

pub fn handle_command() -> Result<()> {
    let execution_config: RpmRepoExecConfig = argh::from_env();

    match execution_config.subcommand {
        Subcommands::Download(c) => download::download(c),
        Subcommands::Create(c) => create::create(c),
        ///
        _ => unimplemented!(),
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
    Download(DownloadCommand),
    Create(CreateCommand),
    // Modify(ModifyCommand),
    // Merge(MergeCommand),
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "clone")]
/// Download repo
pub struct DownloadCommand {
    /// the URL of the repository to download
    #[argh(positional)]
    url: String,

    /// how many files can be downloaded in parallel
    #[argh(option)]
    concurrency: Option<u8>,

    /// directory containing RPMs
    #[argh(positional)]
    destination: OsString,

    /// download metadata only
    #[argh(switch, short = 'm')]
    only_metadata: bool
}

#[derive(FromArgs, PartialEq, Debug)]
/// Init repo subcommand
#[argh(subcommand, name = "create")]
pub struct CreateCommand {
    /// directory containing RPMs
    #[argh(positional)]
    destination: OsString,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Init repo subcommand
#[argh(subcommand, name = "test")]
pub struct TestCommand {}
