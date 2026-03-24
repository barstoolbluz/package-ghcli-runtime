use clap::CommandFactory;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    fs::create_dir_all(&out_dir).expect("failed to create output directory");

    // ghcli-setup(1)
    let cmd = ghcli_setup::cli::Cli::command();
    let man = clap_mangen::Man::new(cmd);
    let mut buf = Vec::new();
    man.render(&mut buf).expect("failed to render ghcli-setup man page");
    fs::write(out_dir.join("ghcli-setup.1"), buf).expect("failed to write ghcli-setup.1");

    // ghcli-reset(1)
    let cmd = ghcli_setup::reset_cli::ResetCli::command();
    let man = clap_mangen::Man::new(cmd);
    let mut buf = Vec::new();
    man.render(&mut buf).expect("failed to render ghcli-reset man page");
    fs::write(out_dir.join("ghcli-reset.1"), buf).expect("failed to write ghcli-reset.1");

    eprintln!("Generated man pages in {}", out_dir.display());
}
