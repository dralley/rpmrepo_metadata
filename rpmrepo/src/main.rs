pub mod commands;

use anyhow::Result;
use commands::handle_command;

fn main() -> Result<()> {
    handle_command()
}
