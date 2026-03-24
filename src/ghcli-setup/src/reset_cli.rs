use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "ghcli-reset", about = "Reset GitHub CLI setup state")]
pub struct ResetCli {
    /// Reset the stored GitHub CLI token.
    #[arg(long)]
    pub token: bool,

    /// Reset Git HTTPS credentials and config.
    #[arg(long)]
    pub git: bool,

    /// Reset managed SSH configuration.
    #[arg(long)]
    pub ssh: bool,

    /// Full reset: token + git + ssh + generated scripts + config file.
    #[arg(long)]
    pub all: bool,

    /// Disable prompts; require explicit flags.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,

    /// Auto-confirm destructive prompts.
    #[arg(long)]
    pub yes: bool,
}
