use anyhow::{bail, Result};
use std::fs;

use crate::config::ConfigFile;
use crate::git;
use crate::keyring_backend;
use crate::paths::Paths;
use crate::reset_cli::ResetCli;
use crate::secret_store;
use crate::ssh;
use crate::ui::Ui;

pub fn run(cli: &ResetCli) -> Result<()> {
    let paths = Paths::new()?;
    let config = ConfigFile::new(paths.config_file.clone());
    let ui = Ui::new(cli.non_interactive, cli.yes);

    let any_flag = cli.token || cli.git || cli.ssh || cli.all;

    if !any_flag && cli.non_interactive {
        bail!("No reset option specified. Use --token, --git, --ssh, or --all.");
    }

    // Determine what to reset
    let (do_token, do_git, do_ssh, do_all) = if cli.all {
        (true, true, true, true)
    } else if any_flag {
        (cli.token, cli.git, cli.ssh, false)
    } else {
        // Interactive menu
        prompt_reset_menu(&ui)?
    };

    if !(do_token || do_git || do_ssh || do_all) {
        ui.info("Nothing selected.");
        return Ok(());
    }

    // Confirm before proceeding
    if !cli.yes && !cli.non_interactive {
        let mut actions = Vec::new();
        if do_token { actions.push("GitHub CLI token"); }
        if do_git { actions.push("Git HTTPS credentials"); }
        if do_ssh { actions.push("SSH configuration"); }
        if do_all { actions.push("all generated scripts and config"); }
        let msg = format!("This will reset: {}. Continue?", actions.join(", "));
        if !ui.confirm(&msg, false)? {
            ui.info("Cancelled.");
            return Ok(());
        }
    }

    // Snapshot config state before any modifications so we can decide
    // whether .local_key is safe to delete.
    let token_storage = config.get("TOKEN_STORAGE").ok().flatten();
    let git_storage = config.get("GIT_CREDENTIAL_STORAGE").ok().flatten();

    if do_token {
        reset_token(&paths, &config, &ui, &token_storage)?;
    }
    if do_git {
        reset_git(&paths, &config, &ui, &git_storage)?;
    }
    if do_ssh {
        reset_ssh(&paths, &config, &ui)?;
    }

    // Clean up .local_key if both backends that could use it were reset,
    // or if the remaining backend doesn't use file storage.
    if (do_token || do_git) && paths.local_key_file.exists() {
        let token_needs_key = !do_token && token_storage.as_deref() == Some("file");
        let git_needs_key = !do_git && git_storage.as_deref() == Some("file");
        if !token_needs_key && !git_needs_key {
            fs::remove_file(&paths.local_key_file).ok();
        }
    }
    if do_all {
        reset_generated_scripts(&paths, &ui)?;
        // Delete the config file itself
        if paths.config_file.exists() {
            fs::remove_file(&paths.config_file).ok();
            ui.info("Deleted config file.");
        }
        // Remove stale lock
        if paths.lock_dir.exists() {
            fs::remove_dir(&paths.lock_dir).ok();
        }
        // Clear FLOX_ENV_CACHE if set
        if let Ok(cache) = std::env::var("FLOX_ENV_CACHE") {
            let cache_path = std::path::Path::new(&cache);
            if cache_path.exists() && cache_path.is_dir() {
                if cli.yes || ui.confirm("Clear Flox environment cache?", false).unwrap_or(false) {
                    fs::remove_dir_all(cache_path).ok();
                    ui.info("Cleared Flox environment cache.");
                }
            }
        }
        // Try to remove the now-empty config dir
        fs::remove_dir(&paths.config_dir).ok();
    }

    ui.info("\nReset complete. Re-activate your shell to pick up the changes.");
    Ok(())
}

fn reset_token(
    paths: &Paths,
    config: &ConfigFile,
    ui: &Ui,
    token_storage: &Option<String>,
) -> Result<()> {
    // Clear keyring
    if token_storage.as_deref() == Some("keyring") || token_storage.is_none() {
        let user = keyring_backend::whoami();
        keyring_backend::clear(crate::paths::GITHUB_TOKEN_SERVICE, &user).ok();
    }

    // Delete encrypted token file
    if token_storage.as_deref() == Some("file") || token_storage.is_none() {
        if paths.token_enc_file.exists() {
            fs::remove_file(&paths.token_enc_file).ok();
        }
    }

    config.unset("TOKEN_STORAGE").ok();
    config.unset("GITHUB_TOKEN_STORED").ok();
    config.unset("NEEDS_WRAPPER").ok();

    ui.info("Reset GitHub CLI token.");
    Ok(())
}

fn reset_git(
    paths: &Paths,
    config: &ConfigFile,
    ui: &Ui,
    git_storage: &Option<String>,
) -> Result<()> {
    // Clear keyring
    if git_storage.as_deref() == Some("keyring") || git_storage.is_none() {
        let user = keyring_backend::whoami();
        secret_store::purge_git_backend("keyring", paths);
        // Also try direct clear in case purge_git_backend doesn't cover it
        keyring_backend::clear_git(
            crate::paths::GITHUB_GIT_SERVICE,
            &user,
            "github.com",
        ).ok();
    }

    // Delete encrypted credentials file
    if git_storage.as_deref() == Some("file") || git_storage.is_none() {
        if paths.git_credentials_enc_file.exists() {
            fs::remove_file(&paths.git_credentials_enc_file).ok();
        }
    }

    // Clear git global config
    git::clear_github_https_git_config().ok();

    // Reset gh git protocol to default
    git::set_gh_git_protocol("https").ok();

    config.unset("GIT_CREDENTIAL_STORAGE").ok();
    config.unset("GIT_MODE").ok();

    ui.info("Reset Git HTTPS credentials.");
    Ok(())
}

fn reset_ssh(paths: &Paths, config: &ConfigFile, ui: &Ui) -> Result<()> {
    ssh::clear_managed_github_ssh_config(&paths.ssh_config_file).ok();
    config.unset("SSH_KEY_PATH").ok();
    config.unset("SSH_ROUTE").ok();

    ui.info("Reset SSH configuration. (SSH key files were not deleted.)");
    Ok(())
}

fn reset_generated_scripts(paths: &Paths, ui: &Ui) -> Result<()> {
    for path in &[
        &paths.token_helper,
        &paths.git_helper,
        &paths.bash_wrapper,
        &paths.zsh_wrapper,
        &paths.fish_wrapper,
    ] {
        if path.exists() {
            fs::remove_file(path).ok();
        }
    }
    ui.info("Deleted generated scripts.");
    Ok(())
}

fn prompt_reset_menu(ui: &Ui) -> Result<(bool, bool, bool, bool)> {
    let options = &[
        "GitHub CLI token",
        "Git HTTPS credentials",
        "SSH configuration",
        "Full reset (everything)",
    ];

    let selected = ui.multi_select("Select what to reset:", options)?;

    if selected.is_empty() {
        return Ok((false, false, false, false));
    }

    // "Full reset" overrides individual selections
    if selected.contains(&3) {
        return Ok((true, true, true, true));
    }

    Ok((
        selected.contains(&0),
        selected.contains(&1),
        selected.contains(&2),
        false,
    ))
}
