use clap::Parser;

fn main() {
    let cli = ghcli_setup::cli::Cli::parse();

    if let Err(e) = ghcli_setup::orchestrator::run(&cli) {
        eprintln!("Error: {:?}", e);
        std::process::exit(1);
    }
}
