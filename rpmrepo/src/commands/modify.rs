use std::env;

use anyhow::Result;
use dialoguer::Confirm;

use super::CreateCommand;

pub fn create(config: CreateCommand) -> Result<()> {

    let repository_path = env::current_dir()?.join(config.destination);

    // TODO: Create a whole repository object, enumerate RPMs, populate it with RPMs (including parsing),
    // write metadata

    if repository_path.exists() {
        if Confirm::new()
            .with_prompt("A directory with this name already exists. Overwrite it?")
            .interact()?
        {
            std::fs::remove_dir_all(&repository_path)?;
        } else {
            std::process::exit(0);
        }
    }

    Ok(())
}
