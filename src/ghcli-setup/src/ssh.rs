use anyhow::{Context, Result};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use crate::config::atomic_write;
use crate::paths::{MANAGED_SSH_BLOCK_END, MANAGED_SSH_BLOCK_START};

/// Clear the managed GitHub SSH config block from ~/.ssh/config.
pub fn clear_managed_github_ssh_config(ssh_config: &Path) -> Result<()> {
    if !ssh_config.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(ssh_config)
        .with_context(|| format!("reading {}", ssh_config.display()))?;

    let mut output = String::new();
    let mut skip = false;

    for line in content.lines() {
        if line == MANAGED_SSH_BLOCK_START {
            skip = true;
            continue;
        }
        if line == MANAGED_SSH_BLOCK_END {
            skip = false;
            continue;
        }
        if !skip {
            output.push_str(line);
            output.push('\n');
        }
    }

    atomic_write(ssh_config, output.as_bytes(), 0o600)
}

/// Install the managed GitHub SSH config block (using ssh.github.com:443).
pub fn install_managed_github_ssh_config(ssh_config: &Path, key_path: &Path) -> Result<()> {
    let ssh_dir = ssh_config.parent().context("ssh config has no parent dir")?;
    fs::create_dir_all(ssh_dir)?;
    fs::set_permissions(ssh_dir, fs::Permissions::from_mode(0o700)).ok();

    // First remove any existing managed block
    clear_managed_github_ssh_config(ssh_config)?;

    // Read existing content
    let existing = if ssh_config.exists() {
        fs::read_to_string(ssh_config)?
    } else {
        String::new()
    };

    let quoted_key = ssh_config_quote(&key_path.to_string_lossy());
    let block = format!(
        "{start}\n\
         Host github.com\n\
         \x20 HostName ssh.github.com\n\
         \x20 Port 443\n\
         \x20 User git\n\
         \x20 IdentitiesOnly yes\n\
         \x20 IdentityFile {key}\n\
         {end}\n",
        start = MANAGED_SSH_BLOCK_START,
        key = quoted_key,
        end = MANAGED_SSH_BLOCK_END,
    );

    let content = format!("{}{}", existing, block);
    atomic_write(ssh_config, content.as_bytes(), 0o600)
}

/// Quote a string for SSH config (double quotes with escaping).
fn ssh_config_quote(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

/// Run an SSH smoke test against a specific host:port.
fn ssh_smoke_test_route(key_path: &Path, host: &str, port: u16) -> Result<bool> {
    let output = Command::new("ssh")
        .args([
            "-T",
            "-o", "BatchMode=yes",
            "-o", "ConnectTimeout=10",
            "-o", "IdentitiesOnly=yes",
            "-o", &format!("IdentityFile={}", key_path.display()),
            "-o", "PreferredAuthentications=publickey",
            "-o", "StrictHostKeyChecking=accept-new",
            "-p", &port.to_string(),
            &format!("git@{}", host),
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output();

    match output {
        Ok(o) => {
            let combined = format!(
                "{}\n{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
            Ok(combined.to_lowercase().contains("successfully authenticated"))
        }
        Err(_) => Ok(false),
    }
}

/// The route used for SSH (host:port).
#[derive(Debug, Clone)]
pub struct SshRoute {
    pub host: String,
    pub port: u16,
}

impl std::fmt::Display for SshRoute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}

/// Run SSH smoke test, trying github.com:22 first, then ssh.github.com:443.
pub fn ssh_smoke_test(key_path: &Path) -> Result<Option<SshRoute>> {
    if ssh_smoke_test_route(key_path, "github.com", 22)? {
        return Ok(Some(SshRoute {
            host: "github.com".to_string(),
            port: 22,
        }));
    }
    if ssh_smoke_test_route(key_path, "ssh.github.com", 443)? {
        return Ok(Some(SshRoute {
            host: "ssh.github.com".to_string(),
            port: 443,
        }));
    }
    Ok(None)
}

/// Generate an SSH keypair using ssh-keygen.
pub fn generate_ssh_keypair(key_path: &Path, passphrase: &str, comment: &str) -> Result<()> {
    let key_dir = key_path.parent().context("key path has no parent")?;
    fs::create_dir_all(key_dir)?;
    fs::set_permissions(key_dir, fs::Permissions::from_mode(0o700))?;

    let status = Command::new("ssh-keygen")
        .args([
            "-t", "ed25519",
            "-f", &key_path.to_string_lossy(),
            "-N", passphrase,
            "-C", comment,
        ])
        .stdout(std::process::Stdio::null())
        .status()
        .context("running ssh-keygen")?;

    if !status.success() {
        anyhow::bail!("ssh-keygen failed");
    }

    fs::set_permissions(key_path, fs::Permissions::from_mode(0o600))?;
    // ssh-keygen creates key_path.pub alongside key_path
    let pub_path_str = format!("{}.pub", key_path.display());
    let pub_path = Path::new(&pub_path_str);
    if pub_path.exists() {
        fs::set_permissions(pub_path, fs::Permissions::from_mode(0o644))?;
    }

    Ok(())
}

/// Try to derive the public key from a private key.
pub fn derive_public_key(key_path: &Path) -> Result<()> {
    let pub_path = format!("{}.pub", key_path.display());
    let output = Command::new("ssh-keygen")
        .args(["-y", "-f", &key_path.to_string_lossy()])
        .output()
        .context("running ssh-keygen -y")?;

    if !output.status.success() {
        anyhow::bail!("ssh-keygen -y failed");
    }

    fs::write(&pub_path, &output.stdout)?;
    fs::set_permissions(Path::new(&pub_path), fs::Permissions::from_mode(0o644))?;
    Ok(())
}

/// Try to add the SSH key to the running agent.
pub fn maybe_add_to_agent(key_path: &Path) -> Result<()> {
    if which("ssh-add").is_none() {
        return Ok(());
    }
    if std::env::var("SSH_AUTH_SOCK").is_err() {
        eprintln!("Warning: SSH agent is not active in this shell; add the key later with ssh-add if needed.");
        return Ok(());
    }
    let status = Command::new("ssh-add")
        .arg(key_path.to_string_lossy().as_ref())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    if let Ok(s) = status {
        if !s.success() {
            eprintln!("Warning: Could not add the SSH key to the running agent.");
        }
    }
    Ok(())
}

/// Get the short hostname.
pub fn host_shortname() -> String {
    let output = Command::new("hostname")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    output.split('.').next().unwrap_or("unknown").to_string()
}

/// Check if a command exists on PATH.
fn which(cmd: &str) -> Option<String> {
    Command::new("which")
        .arg(cmd)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}
