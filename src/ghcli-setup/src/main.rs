mod cli;
mod config;
mod crypto;
mod generated_scripts;
mod git;
mod github_api;
mod keyring_backend;
mod orchestrator;
mod paths;
mod secret_store;
mod ssh;
mod transaction;
mod ui;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();

    if let Err(e) = orchestrator::run(&cli) {
        eprintln!("Error: {:?}", e);
        std::process::exit(1);
    }
}
