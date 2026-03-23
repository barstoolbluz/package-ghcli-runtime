use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, ValueEnum)]
pub enum GitMode {
    Https,
    Ssh,
    Skip,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum FallbackPolicy {
    Never,
    Prompt,
    Always,
}

#[derive(Parser, Debug)]
#[command(name = "ghcli-setup", about = "Flox GitHub setup wizard")]
pub struct Cli {
    /// Disable prompts; missing required inputs become errors.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,

    /// Accept default yes/no prompts where the default is yes.
    #[arg(long)]
    pub yes: bool,

    /// GitHub token for gh.
    #[arg(long, env = "FLOX_GITHUB_TOKEN")]
    pub token: Option<String>,

    /// Discard any existing stored token and prompt for (or accept) a new one.
    #[arg(long = "replace-token", env = "FLOX_REPLACE_TOKEN")]
    pub replace_token: bool,

    /// Git mode for github.com.
    #[arg(long = "git-mode", env = "FLOX_GIT_MODE")]
    pub git_mode: Option<GitMode>,

    /// GitHub username for HTTPS Git.
    #[arg(long = "git-username", env = "FLOX_GIT_USERNAME")]
    pub git_username: Option<String>,

    /// Token for HTTPS Git. Defaults to --token when omitted.
    #[arg(long = "git-password", env = "FLOX_GIT_PASSWORD")]
    pub git_password: Option<String>,

    /// SSH private key path.
    #[arg(long = "ssh-key-path", env = "FLOX_SSH_KEY_PATH")]
    pub ssh_key_path: Option<String>,

    /// SSH key title for upload.
    #[arg(long = "ssh-key-title", env = "FLOX_SSH_KEY_TITLE")]
    pub ssh_key_title: Option<String>,

    /// Set git config --global user.name.
    #[arg(long = "git-user-name", env = "FLOX_GIT_USER_NAME")]
    pub git_user_name: Option<String>,

    /// Set git config --global user.email.
    #[arg(long = "git-user-email", env = "FLOX_GIT_USER_EMAIL")]
    pub git_user_email: Option<String>,

    /// Policy for encrypted local-file fallback.
    #[arg(long = "file-fallback", env = "FLOX_FILE_FALLBACK_POLICY")]
    pub file_fallback: Option<FallbackPolicy>,

    /// Rewrite GitHub remotes in the listed repos after setup.
    #[arg(long = "rewrite-remotes")]
    pub rewrite_remotes: bool,

    /// Repo path to rewrite. Repeat this flag for multiple repos.
    #[arg(long = "repo")]
    pub repo: Vec<String>,
}

impl Cli {
    /// Resolve fallback policy: explicit > non-interactive default > prompt
    pub fn resolved_fallback_policy(&self) -> FallbackPolicy {
        if let Some(ref p) = self.file_fallback {
            p.clone()
        } else if self.non_interactive {
            FallbackPolicy::Never
        } else {
            FallbackPolicy::Prompt
        }
    }
}
