use clap::Parser;

fn main() {
    let cli = ghcli_setup::reset_cli::ResetCli::parse();

    if let Err(e) = ghcli_setup::reset::run(&cli) {
        eprintln!("Error: {:?}", e);
        std::process::exit(1);
    }
}
