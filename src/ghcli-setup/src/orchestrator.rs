use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::{Cli, FallbackPolicy, GitMode};
use crate::config::ConfigFile;
use crate::generated_scripts;
use crate::git;
use crate::github_api;
use crate::paths::Paths;
use crate::secret_store::{self, StorageMethod};
use crate::ssh;
use crate::transaction::TransactionGuard;
use crate::ui::Ui;

/// Acquire a directory-based lock to prevent concurrent setup runs.
fn acquire_lock(lock_dir: &Path) -> Result<()> {
    match fs::create_dir(lock_dir) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            bail!(
                "Another setup instance appears to be running (lock exists: {}). \
                 If this is stale, remove it manually.",
                lock_dir.display()
            );
        }
        Err(e) => Err(e).with_context(|| format!("creating lock dir {}", lock_dir.display())),
    }
}

/// Release the directory-based lock.
fn release_lock(lock_dir: &Path) {
    fs::remove_dir(lock_dir).ok();
}

/// Check if a user input string is an exit/quit request.
fn is_exit(s: &str) -> bool {
    matches!(s, "exit" | "quit")
}

/// Main orchestrator for the setup wizard.
pub fn run(cli: &Cli) -> Result<()> {
    let paths = Paths::new()?;
    let config = ConfigFile::new(paths.config_file.clone());
    let ui = Ui::new(cli.non_interactive, cli.yes);
    let fallback_policy = cli.resolved_fallback_policy();

    // Ensure config dir exists and is safe
    std::fs::create_dir_all(&paths.config_dir)?;
    crate::config::set_permissions(&paths.config_dir, 0o700).ok();
    verify_config_dir_safe(&paths.config_dir)?;

    // Acquire lock to prevent concurrent setup runs
    acquire_lock(&paths.lock_dir)?;
    let result = run_inner(cli, &paths, &config, &ui, &fallback_policy);
    release_lock(&paths.lock_dir);
    result
}

/// Verify the config directory is not a symlink and is owned by us.
/// Prevents a symlink attack where an adversary pre-creates a symlink
/// at ~/.config/gh/flox pointing to a directory they control.
fn verify_config_dir_safe(config_dir: &Path) -> Result<()> {
    use std::os::unix::fs::MetadataExt;

    // Get our uid via `id -u` (works on Linux + macOS, no libc dependency)
    let my_uid = std::process::Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<u32>().ok());

    // Check the config dir and its immediate parents (flox, gh, .config)
    let mut path = config_dir.to_path_buf();
    for _ in 0..3 {
        if path.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false) {
            bail!(
                "Security error: '{}' is a symlink. \
                 Remove it and re-run setup.",
                path.display()
            );
        }
        if let (Some(uid), Ok(meta)) = (my_uid, path.metadata()) {
            if meta.uid() != uid {
                bail!(
                    "Security error: '{}' is not owned by you. \
                     Fix ownership and re-run setup.",
                    path.display()
                );
            }
        }
        if !path.pop() {
            break;
        }
    }
    Ok(())
}

/// Verify that required external tools are on PATH.
fn check_required_tools() -> Result<()> {
    for tool in &["git", "gh"] {
        let found = std::process::Command::new("which")
            .arg(tool)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !found {
            bail!("Required tool '{}' not found on PATH. Install it and try again.", tool);
        }
    }
    Ok(())
}

fn run_inner(
    cli: &Cli,
    paths: &Paths,
    config: &ConfigFile,
    ui: &Ui,
    fallback_policy: &FallbackPolicy,
) -> Result<()> {
    check_required_tools()?;

    // Write runtime assets early
    generated_scripts::write_all(paths)?;

    // Welcome
    if !cli.non_interactive {
        ui.clear();
        show_welcome_message(ui);
        if !ui.confirm("Continue with setup?", true)? {
            ui.info("Setup cancelled.");
            bail!("Setup cancelled.");
        }
    }

    // Step 1: Token
    let (github_token, token_storage) =
        setup_token(cli, paths, config, ui, fallback_policy)?;

    // Apply git user.name / user.email
    apply_git_user_info(cli, ui)?;

    // Step 2: Git mode
    let selected_mode = prompt_for_git_mode(cli, ui)?;
    let (git_mode, git_storage) = match selected_mode {
        GitMode::Https => {
            let result = setup_git_https(
                &github_token,
                cli,
                paths,
                config,
                ui,
                fallback_policy,
            )?;
            // Rewrite remotes if requested (HTTPS mode)
            if cli.rewrite_remotes {
                rewrite_remotes(cli, "https")?;
            }
            result
        }
        GitMode::Ssh => {
            let result = setup_git_ssh(&github_token, cli, paths, config, ui)?;
            // Rewrite remotes if requested (SSH mode)
            if cli.rewrite_remotes {
                rewrite_remotes(cli, "ssh")?;
            }
            result
        }
        GitMode::Skip => {
            let mode = config
                .get("GIT_MODE")?
                .or_else(|| git::current_gh_git_protocol().ok().flatten())
                .unwrap_or_else(|| "not configured".to_string());
            let storage = config
                .get("GIT_CREDENTIAL_STORAGE")?
                .map(|s| match s.as_str() {
                    "keyring" => "keyring".to_string(),
                    "file" => "encrypted file".to_string(),
                    _ => "not configured".to_string(),
                })
                .unwrap_or_else(|| "not configured".to_string());
            (mode, storage)
        }
    };

    // Signal to the shell profile that the gh wrapper should be sourced
    config.set("NEEDS_WRAPPER", "true")?;

    // Completion
    show_completion_message(&token_storage, &git_mode, &git_storage, paths);

    Ok(())
}

/// Rewrite remotes for the given repos with the specified mode.
fn rewrite_remotes(cli: &Cli, mode_str: &str) -> Result<()> {
    let repos: Vec<PathBuf> = if cli.repo.is_empty() {
        vec![std::env::current_dir()?]
    } else {
        cli.repo.iter().map(PathBuf::from).collect()
    };
    for repo in &repos {
        git::rewrite_repo_remotes(repo, mode_str)?;
    }
    Ok(())
}

/// Setup token - check existing or prompt for new.
fn setup_token(
    cli: &Cli,
    paths: &Paths,
    config: &ConfigFile,
    ui: &Ui,
    fallback_policy: &FallbackPolicy,
) -> Result<(String, String)> {
    // Check existing token unless CLI provides one or --replace-token is set
    let skip_existing = cli.replace_token || cli.token.is_some();
    if !skip_existing {
        if let Some(existing) = check_existing_token(paths, config)? {
            let storage = config
                .get("TOKEN_STORAGE")?
                .map(|s| match s.as_str() {
                    "keyring" => "keyring",
                    "file" => "encrypted file",
                    _ => "not configured",
                })
                .unwrap_or("not configured")
                .to_string();
            if cli.non_interactive {
                return Ok((existing, storage));
            }
            ui.info("A valid stored GitHub token was found.\n");
            if ui.confirm("Keep the existing token?", true)? {
                return Ok((existing, storage));
            }
            // User chose to replace — fall through
        }
    }

    // Need to get a new token
    if !cli.non_interactive {
        show_token_instructions(ui);
    }
    let token = prompt_for_token(cli, ui)?;

    // Snapshot current token state for rollback restore
    let old_token_storage = config.get("TOKEN_STORAGE")?;
    let old_token = if old_token_storage.is_some() {
        secret_store::retrieve_token(paths, config).ok().flatten()
    } else {
        None
    };

    // Store it
    let method = secret_store::store_token(&token, paths, fallback_policy, ui)
        .context("storing GitHub token")?;

    config.set("TOKEN_STORAGE", method.as_str())?;
    config.set("GITHUB_TOKEN_STORED", "true")?;

    // Re-write runtime assets with storage config in place
    generated_scripts::write_all(paths)?;

    // Verify via gh healthcheck
    if !git::healthcheck_gh(&token)? {
        // Roll back: purge the just-stored token
        secret_store::purge_token_backend(method.as_str(), paths);

        // Restore previous token if we had one
        if let (Some(ref old_method), Some(ref old_tok)) = (&old_token_storage, &old_token) {
            config.set("TOKEN_STORAGE", old_method)?;
            config.set("GITHUB_TOKEN_STORED", "true")?;
            // Best-effort restore of old token
            let _ = match old_method.as_str() {
                "keyring" => secret_store::stage_and_promote_token_keyring(
                    crate::paths::GITHUB_TOKEN_SERVICE,
                    old_tok,
                    &crate::keyring_backend::whoami(),
                ),
                "file" => secret_store::stage_and_promote_token_file(
                    old_tok,
                    &paths.token_enc_file,
                    &paths.local_key_file,
                ),
                _ => Ok(()),
            };
        } else {
            config.unset("TOKEN_STORAGE")?;
            config.unset("GITHUB_TOKEN_STORED")?;
        }
        bail!("Token stored but gh could not use it. Rolled back token storage.");
    }

    // Purge the other backend
    match method {
        StorageMethod::Keyring => secret_store::purge_token_backend("file", paths),
        StorageMethod::File => secret_store::purge_token_backend("keyring", paths),
    }

    // Warn if git HTTPS credentials may be stale after token replacement
    if old_token.is_some() && !cli.non_interactive {
        if config.get("GIT_MODE")?.as_deref() == Some("https")
            && config.get("GIT_CREDENTIAL_STORAGE")?.is_some()
        {
            ui.warn("Your Git HTTPS credentials may use the previous token. Consider updating them in the Git setup step.");
        }
    }

    Ok((token, method.display_name().to_string()))
}

/// Check if an existing stored token is still valid.
///
/// Does not require `GITHUB_TOKEN_STORED` in config — `retrieve_token`
/// now probes backends itself when the config key is missing.
fn check_existing_token(paths: &Paths, config: &ConfigFile) -> Result<Option<String>> {
    let token = match secret_store::retrieve_token(paths, config)? {
        Some(t) if !t.is_empty() => t,
        _ => return Ok(None),
    };

    match github_api::validate_token(&token) {
        Ok(true) => {
            if git::healthcheck_gh(&token).unwrap_or(false) {
                generated_scripts::write_all(paths)?;
                Ok(Some(token))
            } else {
                Ok(None)
            }
        }
        _ => {
            eprintln!("Warning: Stored GitHub token is invalid or expired.");
            Ok(None)
        }
    }
}

/// Prompt for a GitHub token (from CLI arg or interactive).
fn prompt_for_token(cli: &Cli, ui: &Ui) -> Result<String> {
    if let Some(ref token) = cli.token {
        return Ok(token.clone());
    }

    if cli.non_interactive {
        bail!("GitHub token is required (use --token or FLOX_GITHUB_TOKEN)");
    }

    loop {
        let token = match ui.secret("GitHub token:")? {
            Some(t) if is_exit(t.trim()) => bail!("Setup cancelled"),
            Some(t) => t.trim().to_string(),
            None => bail!("Setup cancelled"),
        };

        if token.is_empty() {
            ui.info("Token cannot be empty.");
            continue;
        }

        ui.info("Validating token...");
        match github_api::validate_token(&token) {
            Ok(true) => {
                if git::healthcheck_gh(&token)? {
                    ui.info("Token works with the GitHub API and gh.");
                    return Ok(token);
                }
                ui.error("Token is valid, but gh could not use it in this env.");
            }
            Ok(false) => {
                ui.error("Token is invalid. Please try again.");
            }
            Err(e) => {
                ui.error(&format!("Token validation failed: {}", e));
            }
        }
    }
}

/// Apply --git-user-name / --git-user-email, or prompt if missing.
fn apply_git_user_info(cli: &Cli, ui: &Ui) -> Result<()> {
    let name = if let Some(ref n) = cli.git_user_name {
        git::config_set("user.name", n)?;
        Some(n.clone())
    } else {
        git::config_get("user.name")?
    };

    let email = if let Some(ref e) = cli.git_user_email {
        git::config_set("user.email", e)?;
        Some(e.clone())
    } else {
        git::config_get("user.email")?
    };

    if cli.non_interactive {
        return Ok(());
    }

    if name.is_some() && email.is_some() {
        return Ok(());
    }

    if !ui.confirm("Set Git user.name and user.email now?", true)? {
        return Ok(());
    }

    if name.is_none() {
        loop {
            match ui.input("Git full name:", "Jane Doe")? {
                None => break,
                Some(n) if is_exit(&n) => break,
                Some(n) if n.is_empty() => {
                    ui.info("Name cannot be empty.");
                }
                Some(n) => {
                    git::config_set("user.name", &n)?;
                    break;
                }
            }
        }
    }

    if email.is_none() {
        loop {
            match ui.input("Git email:", "jane@example.com")? {
                None => break,
                Some(e) if is_exit(&e) => break,
                Some(e) if e.is_empty() => {
                    ui.info("Email cannot be empty.");
                }
                Some(e) => {
                    // Basic email validation
                    if e.contains('@') && e.contains('.') {
                        git::config_set("user.email", &e)?;
                        break;
                    }
                    ui.info("Email format looks invalid.");
                }
            }
        }
    }

    Ok(())
}

/// Prompt for git mode selection.
fn prompt_for_git_mode(cli: &Cli, ui: &Ui) -> Result<GitMode> {
    if let Some(ref mode) = cli.git_mode {
        return Ok(mode.clone());
    }

    if cli.non_interactive {
        return Ok(GitMode::Skip);
    }

    let choice = ui.select(
        "Choose Git mode for github.com",
        &["HTTPS", "SSH", "Skip Git setup"],
    )?;

    match choice.as_deref() {
        Some("HTTPS") => Ok(GitMode::Https),
        Some("SSH") => Ok(GitMode::Ssh),
        _ => Ok(GitMode::Skip),
    }
}

/// Setup Git HTTPS mode with transactional rollback.
fn setup_git_https(
    github_token: &str,
    cli: &Cli,
    paths: &Paths,
    config: &ConfigFile,
    ui: &Ui,
    fallback_policy: &FallbackPolicy,
) -> Result<(String, String)> {
    if !cli.non_interactive {
        show_token_instructions(ui);
    }

    // Get username
    let suggested = git::github_login_from_token(github_token)?.unwrap_or_default();
    let username = prompt_for_git_username(cli, ui, &suggested)?;

    // Get password
    let password = if let Some(ref p) = cli.git_password {
        p.clone()
    } else if cli.non_interactive || cli.token.is_some() {
        github_token.to_string()
    } else if ui.confirm("Reuse the GitHub CLI token for Git HTTPS?", true)? {
        github_token.to_string()
    } else {
        prompt_for_git_password(cli, ui)?
    };

    // Transaction: backup existing state
    let mut guard = TransactionGuard::new();

    let old_helper = git::backup_config_key("credential.https://github.com.helper")?;
    let old_protocol = git::current_gh_git_protocol()?;
    let old_storage = config.get("GIT_CREDENTIAL_STORAGE")?;
    let old_mode = config.get("GIT_MODE")?;

    // Register rollbacks
    {
        let oh = old_helper.clone();
        guard.push_rollback(move || {
            git::restore_config_key("credential.https://github.com.helper", &oh).ok();
        });
    }
    if let Some(ref p) = old_protocol {
        let p = p.clone();
        guard.push_rollback(move || {
            git::set_gh_git_protocol(&p).ok();
        });
    }
    {
        let os = old_storage.clone();
        let cf = config.path().to_path_buf();
        guard.push_rollback(move || {
            let c = ConfigFile::new(cf);
            match os {
                Some(ref v) => { c.set("GIT_CREDENTIAL_STORAGE", v).ok(); }
                None => { c.unset("GIT_CREDENTIAL_STORAGE").ok(); }
            }
        });
    }
    {
        let om = old_mode.clone();
        let cf = config.path().to_path_buf();
        guard.push_rollback(move || {
            let c = ConfigFile::new(cf);
            match om {
                Some(ref v) => { c.set("GIT_MODE", v).ok(); }
                None => { c.unset("GIT_MODE").ok(); }
            }
        });
    }

    // Store credentials
    let method = secret_store::store_git_credentials(
        &username, &password, paths, fallback_policy, ui,
    ).context("Git HTTPS credential storage failed")?;

    // Register rollback for the just-stored credentials
    {
        let m = method.as_str().to_string();
        let cred_file = paths.git_credentials_enc_file.clone();
        guard.push_rollback(move || {
            match m.as_str() {
                "keyring" => {
                    crate::keyring_backend::clear_git(
                        crate::paths::GITHUB_GIT_SERVICE,
                        &crate::keyring_backend::whoami(),
                        "github.com",
                    ).ok();
                }
                "file" => {
                    std::fs::remove_file(&cred_file).ok();
                }
                _ => {}
            }
        });
    }

    config.set("GIT_CREDENTIAL_STORAGE", method.as_str())?;
    git::configure_github_https_helper(&paths.git_helper)?;

    if !git::set_gh_git_protocol("https")? {
        bail!("Failed to set git protocol to https");
    }

    // Healthcheck
    if !git::healthcheck_git_https(&username, &password)? {
        bail!("Git HTTPS setup failed its GitHub auth check. Existing Git config was kept.");
    }

    // Success - commit transaction
    config.set("GIT_MODE", "https")?;
    config.unset("SSH_KEY_PATH")?;
    config.unset("SSH_ROUTE")?;
    ssh::clear_managed_github_ssh_config(&paths.ssh_config_file)?;

    // Purge the other backend
    match method {
        StorageMethod::Keyring => secret_store::purge_git_backend("file", paths),
        StorageMethod::File => secret_store::purge_git_backend("keyring", paths),
    }

    guard.commit();
    Ok(("https".to_string(), method.display_name().to_string()))
}

/// Setup Git SSH mode with transactional rollback.
fn setup_git_ssh(
    github_token: &str,
    cli: &Cli,
    paths: &Paths,
    config: &ConfigFile,
    ui: &Ui,
) -> Result<(String, String)> {
    let ssh_key_path = prompt_for_ssh_key_path(cli, ui, &paths.default_ssh_key_path)?;
    let ssh_key_path = prepare_ssh_keypair(&ssh_key_path, cli, ui)?;

    let key_title = cli.ssh_key_title.clone().unwrap_or_else(|| {
        let basename = ssh_key_path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| "key".to_string());
        format!("flox-{}-{}", ssh::host_shortname(), basename)
    });

    let mut guard = TransactionGuard::new();

    // Backup state
    let old_helper = git::backup_config_key("credential.https://github.com.helper")?;
    let old_protocol = git::current_gh_git_protocol()?;
    let old_mode = config.get("GIT_MODE")?;
    let old_storage = config.get("GIT_CREDENTIAL_STORAGE")?;
    let old_ssh_key_path = config.get("SSH_KEY_PATH")?;
    let old_ssh_route = config.get("SSH_ROUTE")?;
    let old_ssh_config = if paths.ssh_config_file.exists() {
        Some(std::fs::read_to_string(&paths.ssh_config_file)?)
    } else {
        None
    };

    // Track uploaded key for rollback
    let mut uploaded_key_id: Option<u64> = None;

    // Check if key is already on GitHub
    let pub_path = format!("{}.pub", ssh_key_path.display());
    let pubkey = std::fs::read_to_string(&pub_path)
        .with_context(|| format!("reading public key {}", pub_path))?;

    let keys = github_api::list_ssh_keys(github_token).unwrap_or_default();
    let existing_id = github_api::find_ssh_key_id(&keys, &pubkey);

    if existing_id.is_none() {
        let choice = if cli.non_interactive {
            // In non-interactive mode, skip upload (matching bash behavior)
            Some("Skip upload (key is already on GitHub)".to_string())
        } else {
            ui.select(
                "SSH key not found on your GitHub account",
                &["Upload key to GitHub now", "Skip upload (key is already on GitHub)"],
            )?
        };

        if choice.as_deref() == Some("Upload key to GitHub now") {
            match github_api::upload_ssh_key(github_token, &pubkey, &key_title) {
                Ok(id) => {
                    uploaded_key_id = Some(id);
                }
                Err(e) => {
                    ui.warn(&format!("SSH key upload failed: {}. Check that your token has admin:public_key scope.", e));
                    bail!("SSH key upload failed");
                }
            }
        }
    }

    // Register rollbacks
    {
        let token = github_token.to_string();
        let kid = uploaded_key_id;
        guard.push_rollback(move || {
            if let Some(id) = kid {
                if let Err(e) = github_api::delete_ssh_key(&token, id) {
                    eprintln!("Warning: Uploaded SSH key could not be deleted after rollback: {}", e);
                }
            }
        });
    }
    {
        let osc = old_ssh_config.clone();
        let scf = paths.ssh_config_file.clone();
        guard.push_rollback(move || {
            match osc {
                Some(content) => {
                    crate::config::atomic_write(&scf, content.as_bytes(), 0o600).ok();
                }
                None => {
                    std::fs::remove_file(&scf).ok();
                }
            }
        });
    }
    {
        let oh = old_helper.clone();
        guard.push_rollback(move || {
            git::restore_config_key("credential.https://github.com.helper", &oh).ok();
        });
    }
    if let Some(ref p) = old_protocol {
        let p = p.clone();
        guard.push_rollback(move || { git::set_gh_git_protocol(&p).ok(); });
    }
    {
        let om = old_mode.clone();
        let os = old_storage.clone();
        let okp = old_ssh_key_path.clone();
        let osr = old_ssh_route.clone();
        let cf = config.path().to_path_buf();
        guard.push_rollback(move || {
            let c = ConfigFile::new(cf);
            match om { Some(ref v) => { c.set("GIT_MODE", v).ok(); } None => { c.unset("GIT_MODE").ok(); } }
            match os { Some(ref v) => { c.set("GIT_CREDENTIAL_STORAGE", v).ok(); } None => { c.unset("GIT_CREDENTIAL_STORAGE").ok(); } }
            match okp { Some(ref v) => { c.set("SSH_KEY_PATH", v).ok(); } None => { c.unset("SSH_KEY_PATH").ok(); } }
            match osr { Some(ref v) => { c.set("SSH_ROUTE", v).ok(); } None => { c.unset("SSH_ROUTE").ok(); } }
        });
    }

    // Set protocol
    if !git::set_gh_git_protocol("ssh")? {
        bail!("Failed to set git protocol to SSH.");
    }

    // Add key to agent
    ssh::maybe_add_to_agent(&ssh_key_path)?;

    // SSH smoke test
    let route = ssh::ssh_smoke_test(&ssh_key_path)?;
    match route {
        Some(ref r) if r.to_string() == "ssh.github.com:443" => {
            ssh::install_managed_github_ssh_config(&paths.ssh_config_file, &ssh_key_path)?;
        }
        Some(_) => {
            ssh::clear_managed_github_ssh_config(&paths.ssh_config_file)?;
        }
        None => {
            ui.warn(&format!(
                "SSH smoke test failed. Could not connect to github.com via SSH with key '{}'.",
                ssh_key_path.display()
            ));
            bail!("SSH setup failed. The script rolled back local state and tried to delete any SSH key uploaded in this run.");
        }
    }

    // Success
    git::clear_github_https_git_config()?;
    secret_store::purge_git_backend("keyring", paths);
    secret_store::purge_git_backend("file", paths);
    config.set("GIT_MODE", "ssh")?;
    config.set("SSH_KEY_PATH", &ssh_key_path.to_string_lossy())?;
    if let Some(ref r) = route {
        config.set("SSH_ROUTE", &r.to_string())?;
    }
    config.unset("GIT_CREDENTIAL_STORAGE")?;

    guard.commit();
    Ok(("ssh".to_string(), "ssh key".to_string()))
}

fn prompt_for_git_username(cli: &Cli, ui: &Ui, default: &str) -> Result<String> {
    if let Some(ref u) = cli.git_username {
        return Ok(u.clone());
    }
    if cli.non_interactive {
        if !default.is_empty() {
            return Ok(default.to_string());
        }
        bail!("GitHub username is required for HTTPS mode");
    }

    loop {
        match ui.input("GitHub username:", default)? {
            None => bail!("Setup cancelled"),
            Some(u) if is_exit(&u) => bail!("Setup cancelled"),
            Some(u) if u.is_empty() => {
                if !default.is_empty() {
                    return Ok(default.to_string());
                }
                ui.info("Username cannot be empty.");
            }
            Some(u) => return Ok(u),
        }
    }
}

fn prompt_for_git_password(cli: &Cli, ui: &Ui) -> Result<String> {
    if let Some(ref p) = cli.git_password {
        return Ok(p.clone());
    }
    if cli.non_interactive {
        bail!("Git token is required for HTTPS mode");
    }

    loop {
        match ui.secret("GitHub token for Git HTTPS:")? {
            None => bail!("Setup cancelled"),
            Some(p) if is_exit(&p) => bail!("Setup cancelled"),
            Some(p) if p.is_empty() => {
                ui.info("Token cannot be empty.");
            }
            Some(p) => return Ok(p),
        }
    }
}

fn prompt_for_ssh_key_path(cli: &Cli, ui: &Ui, default: &PathBuf) -> Result<PathBuf> {
    if let Some(ref p) = cli.ssh_key_path {
        return Ok(PathBuf::from(p));
    }
    if cli.non_interactive {
        return Ok(default.clone());
    }

    let default_str = default.to_string_lossy();
    match ui.input("SSH private key path:", &default_str)? {
        Some(p) if is_exit(&p) => bail!("Setup cancelled"),
        Some(p) if !p.is_empty() => Ok(PathBuf::from(p)),
        _ => Ok(default.clone()),
    }
}

fn prepare_ssh_keypair(key_path: &PathBuf, cli: &Cli, ui: &Ui) -> Result<PathBuf> {
    let pub_path = format!("{}.pub", key_path.display());

    // Both exist
    if key_path.exists() && std::path::Path::new(&pub_path).exists() {
        return Ok(key_path.clone());
    }

    // Private exists, derive public
    if key_path.exists() && !std::path::Path::new(&pub_path).exists() {
        ssh::derive_public_key(key_path)?;
        return Ok(key_path.clone());
    }

    // Partially occupied
    if key_path.exists() || std::path::Path::new(&pub_path).exists() {
        bail!("Key path is partially occupied. Clean it up or choose a different path.");
    }

    // Generate new keypair
    let passphrase = if cli.non_interactive {
        String::new()
    } else {
        if !ui.confirm("Generate a new SSH key pair at that path?", true)? {
            bail!("SSH key generation cancelled");
        }
        match ui.secret("SSH key passphrase (blank allowed):")? {
            Some(p) if is_exit(&p) => bail!("Setup cancelled"),
            Some(p) => p,
            None => bail!("Setup cancelled"),
        }
    };

    let user = crate::keyring_backend::whoami();
    let comment = format!("{}@{}-flox-github", user, ssh::host_shortname());
    ssh::generate_ssh_keypair(key_path, &passphrase, &comment)?;
    Ok(key_path.clone())
}

fn show_welcome_message(ui: &Ui) {
    ui.info("Flox GitHub Setup\n");
    ui.info("This wizard will walk you through two steps:\n");
    ui.info("  Step 1 — Store a GitHub personal access token for CLI access.");
    ui.info("  Step 2 — Choose how Git connects to github.com (HTTPS, SSH, or skip).\n");
    ui.info("Notes:");
    ui.info("  - HTTPS Git should use a personal access token, not an account password.");
    ui.info("  - SSH mode can register an SSH key with GitHub.");
    ui.info("  - Press Ctrl+C or type 'exit' at any prompt to stop.\n");
}

fn show_token_instructions(ui: &Ui) {
    ui.info("Create or choose a GitHub personal access token before continuing.\n");
    ui.info("  1. Go to https://github.com/settings/tokens");
    ui.info("  2. Click 'Generate new token' (classic)");
    ui.info("  3. Select scopes: repo, read:org (minimum)\n");
    ui.info("Typical uses:");
    ui.info("  - GitHub CLI access: token with the API rights you need");
    ui.info("  - Git HTTPS: repo scope for classic PATs, or matching access for fine-grained PATs");
    ui.info("  - SSH key upload: token may also need admin:public_key scope\n");
}

fn show_completion_message(
    token_storage: &str,
    git_mode: &str,
    git_storage: &str,
    paths: &Paths,
) {
    println!("Setup complete.\n");
    println!("GitHub CLI token storage: {}", token_storage);
    println!("Git mode for github.com:  {}", git_mode);
    println!("Git secret storage:       {}\n", git_storage);
    println!("To load the gh wrapper in Bash:");
    println!("  source \"{}\"\n", paths.bash_wrapper.display());
    println!("To load the gh wrapper in Zsh:");
    println!("  source \"{}\"\n", paths.zsh_wrapper.display());
    println!("To load the gh wrapper in Fish:");
    println!("  source \"{}\"\n", paths.fish_wrapper.display());
}
