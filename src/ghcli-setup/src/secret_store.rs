use anyhow::{bail, Context, Result};
use std::path::Path;

use crate::cli::FallbackPolicy;
use crate::config::ConfigFile;
use crate::crypto;
use crate::keyring_backend;
use crate::paths::Paths;
use crate::ui::Ui;

/// Storage method for secrets.
#[derive(Debug, Clone, PartialEq)]
pub enum StorageMethod {
    Keyring,
    File,
}

impl StorageMethod {
    pub fn as_str(&self) -> &str {
        match self {
            StorageMethod::Keyring => "keyring",
            StorageMethod::File => "file",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            StorageMethod::Keyring => "keyring",
            StorageMethod::File => "encrypted file",
        }
    }
}

/// Stage-and-promote: write to a staging location, verify, then promote to final.
/// This is the core reliability pattern replacing the bash backup/restore globals.

/// Store a token using keyring with stage-and-promote.
pub fn stage_and_promote_token_keyring(
    service: &str,
    token: &str,
    user: &str,
) -> Result<()> {
    let stage_service = format!("{}-stage-{}", service, std::process::id());

    // Stage
    keyring_backend::store(&stage_service, user, token)
        .context("staging token to keyring")?;

    // Verify staged
    let staged = keyring_backend::retrieve(&stage_service, user)?
        .context("staged token not found in keyring")?;
    if staged != token {
        keyring_backend::clear(&stage_service, user).ok();
        bail!("staged token mismatch");
    }

    // Promote
    keyring_backend::store(service, user, token)
        .context("promoting token to keyring")?;

    // Verify promoted
    let promoted = keyring_backend::retrieve(service, user)?
        .context("promoted token not found in keyring")?;
    if promoted != token {
        keyring_backend::clear(&stage_service, user).ok();
        bail!("promoted token mismatch");
    }

    // Clean up staging
    keyring_backend::clear(&stage_service, user).ok();
    Ok(())
}

/// Store a token using encrypted file with stage-and-promote.
pub fn stage_and_promote_token_file(
    token: &str,
    enc_file: &Path,
    key_file: &Path,
) -> Result<()> {
    let password = crypto::ensure_local_key(key_file)?;
    let parent = enc_file.parent().context("encrypted file path has no parent")?;
    let stage_file = parent.join(format!(".token-stage-{}.enc", std::process::id()));

    // Stage
    crypto::encrypt_to_file(token.as_bytes(), &stage_file, &password)?;

    // Verify staged
    let staged = crypto::decrypt_from_file(&stage_file, &password)
        .context("verifying staged encrypted token")?;
    if staged != token.as_bytes() {
        std::fs::remove_file(&stage_file).ok();
        bail!("staged encrypted token mismatch");
    }

    // Promote
    std::fs::rename(&stage_file, enc_file)
        .with_context(|| format!("promoting {} to {}", stage_file.display(), enc_file.display()))?;

    // Verify promoted
    let promoted = crypto::decrypt_from_file(enc_file, &password)
        .context("verifying promoted encrypted token")?;
    if promoted != token.as_bytes() {
        bail!("promoted encrypted token mismatch");
    }

    Ok(())
}

/// The default host attribute used when storing git credentials in the keyring.
/// This matches the `host github.com` attribute the bash helpers pass to
/// `secret-tool` on Linux, ensuring cross-implementation compatibility.
const GIT_KEYRING_HOST: &str = "github.com";

/// Store git credentials (username:password) using keyring with stage-and-promote.
///
/// Uses the git-specific keyring functions that shell out to `security` / `secret-tool`
/// directly so the extra `host` attribute is included, matching the bash helper behavior.
pub fn stage_and_promote_git_keyring(
    service: &str,
    username: &str,
    password: &str,
    user: &str,
) -> Result<()> {
    let payload = format!("{}:{}", username, password);
    let stage_service = format!("{}-stage-{}", service, std::process::id());

    // Stage
    keyring_backend::store_git(&stage_service, user, &payload, GIT_KEYRING_HOST)
        .context("staging git credentials to keyring")?;

    // Verify staged
    let staged = keyring_backend::retrieve_git(&stage_service, user, GIT_KEYRING_HOST)?
        .context("staged git credentials not found in keyring")?;
    if staged != payload {
        keyring_backend::clear_git(&stage_service, user, GIT_KEYRING_HOST).ok();
        bail!("staged git credentials mismatch");
    }

    // Promote
    keyring_backend::store_git(service, user, &payload, GIT_KEYRING_HOST)
        .context("promoting git credentials to keyring")?;

    // Verify promoted
    let promoted = keyring_backend::retrieve_git(service, user, GIT_KEYRING_HOST)?
        .context("promoted git credentials not found in keyring")?;
    if promoted != payload {
        keyring_backend::clear_git(&stage_service, user, GIT_KEYRING_HOST).ok();
        bail!("promoted git credentials mismatch");
    }

    // Clean up staging
    keyring_backend::clear_git(&stage_service, user, GIT_KEYRING_HOST).ok();
    Ok(())
}

/// Store git credentials using encrypted file with stage-and-promote.
pub fn stage_and_promote_git_file(
    username: &str,
    password: &str,
    enc_file: &Path,
    key_file: &Path,
) -> Result<()> {
    let payload = format!("{}:{}", username, password);
    stage_and_promote_token_file(&payload, enc_file, key_file)
}

/// Try keyring first, fall back to file if allowed.
pub fn store_token(
    token: &str,
    paths: &Paths,
    fallback_policy: &FallbackPolicy,
    ui: &Ui,
) -> Result<StorageMethod> {
    let user = keyring_backend::whoami();
    let mut keyring_attempted = false;

    // Try keyring first
    if keyring_backend::available() {
        keyring_attempted = true;
        match stage_and_promote_token_keyring(
            crate::paths::GITHUB_TOKEN_SERVICE,
            token,
            &user,
        ) {
            Ok(()) => return Ok(StorageMethod::Keyring),
            Err(e) => {
                ui.warn(&format!("Keyring storage failed: {}", e));
            }
        }
    }

    // Fall back to file
    if file_fallback_allowed(fallback_policy, ui)? {
        stage_and_promote_token_file(token, &paths.token_enc_file, &paths.local_key_file)?;
        return Ok(StorageMethod::File);
    }

    if keyring_attempted {
        bail!("Keyring storage was unavailable or unusable, and file fallback is disabled.");
    }
    bail!("No secret storage backend available.");
}

/// Try keyring first, fall back to file if allowed, for git credentials.
pub fn store_git_credentials(
    username: &str,
    password: &str,
    paths: &Paths,
    fallback_policy: &FallbackPolicy,
    ui: &Ui,
) -> Result<StorageMethod> {
    let user = keyring_backend::whoami();
    let mut keyring_attempted = false;

    if keyring_backend::available() {
        keyring_attempted = true;
        match stage_and_promote_git_keyring(
            crate::paths::GITHUB_GIT_SERVICE,
            username,
            password,
            &user,
        ) {
            Ok(()) => return Ok(StorageMethod::Keyring),
            Err(e) => {
                ui.warn(&format!("Keyring storage for Git failed: {}", e));
            }
        }
    }

    if file_fallback_allowed(fallback_policy, ui)? {
        stage_and_promote_git_file(
            username,
            password,
            &paths.git_credentials_enc_file,
            &paths.local_key_file,
        )?;
        return Ok(StorageMethod::File);
    }

    if keyring_attempted {
        bail!("Keyring storage for Git HTTPS was unavailable or unusable, and file fallback is disabled.");
    }
    bail!("No secret storage backend available for Git credentials.");
}

/// Retrieve the stored GitHub token.
///
/// When `TOKEN_STORAGE` is present in config, uses that backend directly.
/// When the key is missing (e.g. config was partially corrupted), probes
/// keyring first, then encrypted file, so an existing token is not lost.
pub fn retrieve_token(paths: &Paths, config: &ConfigFile) -> Result<Option<String>> {
    let method = config.get("TOKEN_STORAGE")?;
    match method.as_deref() {
        Some("keyring") => {
            let user = keyring_backend::whoami();
            keyring_backend::retrieve(crate::paths::GITHUB_TOKEN_SERVICE, &user)
        }
        Some("file") => {
            retrieve_token_from_file(paths)
        }
        _ => {
            // Config missing or corrupt — probe both backends.
            // Swallow keyring errors so we still try the file backend.
            if keyring_backend::available() {
                let user = keyring_backend::whoami();
                if let Ok(Some(t)) = keyring_backend::retrieve(crate::paths::GITHUB_TOKEN_SERVICE, &user) {
                    if !t.is_empty() {
                        return Ok(Some(t));
                    }
                }
            }
            retrieve_token_from_file(paths)
        }
    }
}

/// Try to retrieve a token from the encrypted file backend.
fn retrieve_token_from_file(paths: &Paths) -> Result<Option<String>> {
    if !paths.token_enc_file.exists() || !paths.local_key_file.exists() {
        return Ok(None);
    }
    let password = std::fs::read_to_string(&paths.local_key_file)?
        .trim()
        .to_string();
    match crypto::decrypt_from_file(&paths.token_enc_file, &password) {
        Ok(data) => Ok(Some(String::from_utf8(data)?)),
        Err(_) => Ok(None),
    }
}

/// Purge a token backend (keyring or file).
pub fn purge_token_backend(method: &str, paths: &Paths) {
    let user = keyring_backend::whoami();
    match method {
        "keyring" => {
            keyring_backend::clear(crate::paths::GITHUB_TOKEN_SERVICE, &user).ok();
        }
        "file" => {
            std::fs::remove_file(&paths.token_enc_file).ok();
        }
        _ => {}
    }
}

/// Purge a git credential backend.
pub fn purge_git_backend(method: &str, paths: &Paths) {
    let user = keyring_backend::whoami();
    match method {
        "keyring" => {
            keyring_backend::clear_git(
                crate::paths::GITHUB_GIT_SERVICE,
                &user,
                GIT_KEYRING_HOST,
            ).ok();
        }
        "file" => {
            std::fs::remove_file(&paths.git_credentials_enc_file).ok();
        }
        _ => {}
    }
}

/// Check if file fallback is allowed based on policy.
fn file_fallback_allowed(policy: &FallbackPolicy, ui: &Ui) -> Result<bool> {
    match policy {
        FallbackPolicy::Always => Ok(true),
        FallbackPolicy::Never => Ok(false),
        FallbackPolicy::Prompt => {
            ui.info("Encrypted local-file fallback:");
            ui.info("  - This is a local fallback when the OS secret store is not usable.");
            ui.info("  - The encrypted data and the local key live under the same account.");
            ui.info("  - Use this on a trusted machine only.");
            ui.confirm("Use encrypted local-file fallback?", false)
        }
    }
}
