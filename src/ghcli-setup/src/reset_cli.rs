use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "ghcli-reset",
    about = "Reset GitHub CLI setup state",
    long_about = "Selectively or fully reset GitHub CLI authentication state.\n\n\
        With no flags, presents an interactive multi-select menu.\n\
        With explicit flags, resets the specified components without a menu.\n\n\
        What each mode clears:\n\
        --token   Keyring entry (flox-github), encrypted token file, config keys\n\
        --git     Keyring entry (flox-github-git), encrypted credentials file,\n\
                  credential.https://github.com.helper from git global config,\n\
                  gh git_protocol setting\n\
        --ssh     Managed block in ~/.ssh/config, config keys (SSH key files\n\
                  are never deleted)\n\
        --all     All of the above + generated scripts + config file +\n\
                  Flox environment cache (with prompt unless --yes)",
    after_long_help = "EXAMPLES:\n    \
        ghcli-reset                      Interactive menu\n    \
        ghcli-reset --token              Reset token only\n    \
        ghcli-reset --token --git        Reset token and git credentials\n    \
        ghcli-reset --all                Full reset (with confirmation)\n    \
        ghcli-reset --all --yes          Full reset, no prompts\n\n\
        ENVIRONMENT TRIGGER:\n    \
        RESET=1 flox activate            Triggers ghcli-reset --all --yes\n\n\
        SEE ALSO:\n    \
        ghcli-setup(1)"
)]
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
