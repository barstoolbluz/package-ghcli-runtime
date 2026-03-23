use anyhow::{bail, Context, Result};
use std::process::Command;

/// Store a token in the OS keyring by shelling out to `security` / `secret-tool`.
///
/// We bypass the `keyring` crate because its D-Bus Secret Service backend
/// is unreliable in Nix-built binaries — `set_password` succeeds but
/// `get_password` returns `NoEntry`.  Shelling out to `secret-tool` matches
/// the generated bash helpers and works everywhere.
pub fn store(service: &str, user: &str, secret: &str) -> Result<()> {
    match os_type() {
        "macos" => {
            let output = Command::new("security")
                .args([
                    "add-generic-password",
                    "-U",
                    "-s", service,
                    "-a", user,
                    "-w", secret,
                ])
                .output()
                .context("running security add-generic-password")?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!("security add-generic-password failed: {}", stderr.trim());
            }
            Ok(())
        }
        "linux" => {
            let mut child = Command::new("secret-tool")
                .args([
                    "store",
                    "--label", &format!("Flox {}", service),
                    "service", service,
                    "user", user,
                ])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .context("spawning secret-tool store")?;
            {
                use std::io::Write;
                let stdin = child.stdin.as_mut().context("secret-tool stdin")?;
                stdin.write_all(secret.as_bytes())?;
                stdin.flush()?;
            }
            drop(child.stdin.take());
            let output = child.wait_with_output().context("waiting for secret-tool")?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!("secret-tool store failed: {}", stderr.trim());
            }
            Ok(())
        }
        other => bail!("unsupported OS for keyring: {}", other),
    }
}

/// Retrieve a token from the OS keyring. Returns None if not found.
pub fn retrieve(service: &str, user: &str) -> Result<Option<String>> {
    match os_type() {
        "macos" => {
            let output = Command::new("security")
                .args([
                    "find-generic-password",
                    "-s", service,
                    "-a", user,
                    "-w",
                ])
                .output()
                .context("running security find-generic-password")?;
            if output.status.success() {
                let secret = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if secret.is_empty() { Ok(None) } else { Ok(Some(secret)) }
            } else {
                Ok(None)
            }
        }
        "linux" => {
            let output = Command::new("secret-tool")
                .args([
                    "lookup",
                    "service", service,
                    "user", user,
                ])
                .output()
                .context("running secret-tool lookup")?;
            if output.status.success() {
                let secret = String::from_utf8_lossy(&output.stdout)
                    .trim_end_matches('\n')
                    .to_string();
                if secret.is_empty() { Ok(None) } else { Ok(Some(secret)) }
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

/// Clear a token from the OS keyring.
pub fn clear(service: &str, user: &str) -> Result<()> {
    match os_type() {
        "macos" => {
            let _ = Command::new("security")
                .args(["delete-generic-password", "-s", service, "-a", user])
                .output();
            Ok(())
        }
        "linux" => {
            let _ = Command::new("secret-tool")
                .args(["clear", "service", service, "user", user])
                .output();
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Detect the OS type.
fn os_type() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unsupported"
    }
}

/// Check if a command exists on the system.
fn command_exists(cmd: &str) -> bool {
    // Use 'which' to avoid shell injection; safe even if cmd contains special chars.
    Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Check if keyring storage is available on this system.
///
/// Instead of doing a destructive store+delete roundtrip, just check whether
/// the platform secret-store command (`security` on macOS, `secret-tool` on
/// Linux) is present.  This matches the bash helper behaviour.
pub fn available() -> bool {
    match os_type() {
        "macos" => command_exists("security"),
        "linux" => command_exists("secret-tool"),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Git-credential-specific functions
//
// The bash helpers store git credentials with an extra `host github.com`
// attribute when calling `secret-tool` on Linux.  The `keyring` crate's
// `Entry::new(service, user)` API has no way to pass extra attributes, so
// secrets written by bash would not be found by Rust and vice-versa.
//
// To maintain full compatibility we bypass the crate for git credentials and
// shell out to `security` / `secret-tool` directly, exactly as the generated
// bash scripts do.
// ---------------------------------------------------------------------------

/// Store a git credential in the OS keyring, including the host attribute.
pub fn store_git(service: &str, user: &str, secret: &str, host: &str) -> Result<()> {
    match os_type() {
        "macos" => {
            // security add-generic-password -U -s SERVICE -a USER -w SECRET
            // -U updates if exists
            let status = Command::new("security")
                .args([
                    "add-generic-password",
                    "-U",
                    "-s", service,
                    "-a", user,
                    "-w", secret,
                ])
                .output()
                .context("running security add-generic-password")?;
            if !status.status.success() {
                let stderr = String::from_utf8_lossy(&status.stderr);
                bail!("security add-generic-password failed: {}", stderr.trim());
            }
            Ok(())
        }
        "linux" => {
            // printf '%s' "$secret" | secret-tool store --label '...' service SERVICE user USER host HOST
            let mut child = Command::new("secret-tool")
                .args([
                    "store",
                    "--label", "Flox GitHub Git Credentials",
                    "service", service,
                    "user", user,
                    "host", host,
                ])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .context("spawning secret-tool store")?;

            {
                use std::io::Write;
                let stdin = child.stdin.as_mut()
                    .context("secret-tool stdin not available")?;
                stdin.write_all(secret.as_bytes())
                    .context("writing secret to secret-tool stdin")?;
                stdin.flush()
                    .context("flushing secret to secret-tool stdin")?;
            }
            // Drop stdin so secret-tool sees EOF
            drop(child.stdin.take());

            let output = child.wait_with_output()
                .context("waiting for secret-tool store")?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!("secret-tool store failed: {}", stderr.trim());
            }
            Ok(())
        }
        other => bail!("unsupported OS for git keyring: {}", other),
    }
}

/// Retrieve a git credential from the OS keyring, using the host attribute.
/// Returns None if the credential is not found.
pub fn retrieve_git(service: &str, user: &str, host: &str) -> Result<Option<String>> {
    match os_type() {
        "macos" => {
            // security find-generic-password -s SERVICE -a USER -w
            let output = Command::new("security")
                .args([
                    "find-generic-password",
                    "-s", service,
                    "-a", user,
                    "-w",
                ])
                .output()
                .context("running security find-generic-password")?;
            if output.status.success() {
                let secret = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .to_string();
                if secret.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(secret))
                }
            } else {
                Ok(None)
            }
        }
        "linux" => {
            // secret-tool lookup service SERVICE user USER host HOST
            let output = Command::new("secret-tool")
                .args([
                    "lookup",
                    "service", service,
                    "user", user,
                    "host", host,
                ])
                .output()
                .context("running secret-tool lookup")?;
            if output.status.success() {
                let secret = String::from_utf8_lossy(&output.stdout)
                    .trim_end_matches('\n')
                    .to_string();
                if secret.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(secret))
                }
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

/// Clear a git credential from the OS keyring, using the host attribute.
pub fn clear_git(service: &str, user: &str, host: &str) -> Result<()> {
    match os_type() {
        "macos" => {
            // security delete-generic-password -s SERVICE -a USER
            let _ = Command::new("security")
                .args([
                    "delete-generic-password",
                    "-s", service,
                    "-a", user,
                ])
                .output();
            Ok(())
        }
        "linux" => {
            // secret-tool clear service SERVICE user USER host HOST
            let _ = Command::new("secret-tool")
                .args([
                    "clear",
                    "service", service,
                    "user", user,
                    "host", host,
                ])
                .output();
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Get the current username.
pub fn whoami() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}
