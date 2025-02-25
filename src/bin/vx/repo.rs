use clap::{Args, Subcommand};
use std::collections::HashMap;
use vx::core::repo::Repo;

#[derive(Args, Debug)]
pub(super) struct RepoArgs {
    #[command(subcommand)]
    cmd: RepoCommands,
}

#[derive(Debug, Subcommand)]
enum RepoCommands {
    New { name: String },
}

pub(super) fn exec(args: &RepoArgs) {
    // TODO: Handle errors
    match &args.cmd {
        RepoCommands::New { name } => {
            new(name);
        }
    }
}

fn new(name: &str) {
    match Repo::new(name.to_string(), HashMap::new()) {
        Ok((repo, _)) => eprintln!("Created new repository: {}", repo.name),
        Err(e) => eprintln!("Failed to create new repository: {:?}", e),
    }
}
