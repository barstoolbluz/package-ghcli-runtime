use anyhow::{Context, Result};
use std::path::PathBuf;

/// All path constants, derived from XDG_CONFIG_HOME or ~/.config
pub struct Paths {
    pub config_dir: PathBuf,
    pub config_file: PathBuf,
    pub token_enc_file: PathBuf,
    pub git_credentials_enc_file: PathBuf,
    pub local_key_file: PathBuf,
    pub token_helper: PathBuf,
    pub git_helper: PathBuf,
    pub bash_wrapper: PathBuf,
    pub zsh_wrapper: PathBuf,
    pub fish_wrapper: PathBuf,
    #[allow(dead_code)]
    pub lock_file: PathBuf,
    pub lock_dir: PathBuf,
    pub default_ssh_key_path: PathBuf,
    pub ssh_config_file: PathBuf,
}

pub const GITHUB_TOKEN_SERVICE: &str = "flox-github";
pub const GITHUB_GIT_SERVICE: &str = "flox-github-git";
pub const MANAGED_SSH_BLOCK_START: &str = "# >>> flox github ssh >>>";
pub const MANAGED_SSH_BLOCK_END: &str = "# <<< flox github ssh <<<";

impl Paths {
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir()
            .context("cannot determine home directory — set HOME or XDG_CONFIG_HOME")?;
        let config_base = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home.join(".config"));
        let config_dir = config_base.join("gh").join("flox");

        Ok(Self {
            config_file: config_dir.join("github_config"),
            token_enc_file: config_dir.join("github_token.enc"),
            git_credentials_enc_file: config_dir.join("git_credentials.enc"),
            local_key_file: config_dir.join(".local_key"),
            token_helper: config_dir.join("gh-token-helper"),
            git_helper: config_dir.join("git-credential-flox-helper"),
            bash_wrapper: config_dir.join("gh_wrapper.bash"),
            zsh_wrapper: config_dir.join("gh_wrapper.zsh"),
            fish_wrapper: config_dir.join("gh_wrapper.fish"),
            lock_file: config_dir.join(".setup.lock"),
            lock_dir: config_dir.join(".setup.lock.d"),
            default_ssh_key_path: home.join(".ssh").join("id_ed25519_flox_github"),
            ssh_config_file: home.join(".ssh").join("config"),
            config_dir,
        })
    }
}
