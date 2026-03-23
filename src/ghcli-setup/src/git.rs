use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;
use std::process::Command;

/// Get a git config value (global scope).
pub fn config_get(key: &str) -> Result<Option<String>> {
    let output = Command::new("git")
        .args(["config", "--global", "--get", key])
        .output()
        .context("running git config --get")?;

    if output.status.success() {
        Ok(Some(String::from_utf8_lossy(&output.stdout).trim().to_string()))
    } else {
        Ok(None)
    }
}

/// Set a git config value (global scope).
pub fn config_set(key: &str, value: &str) -> Result<()> {
    let status = Command::new("git")
        .args(["config", "--global", key, value])
        .status()
        .context("running git config --global set")?;
    if !status.success() {
        anyhow::bail!("git config --global {} failed", key);
    }
    Ok(())
}

/// Unset all values of a git config key (global scope).
pub fn config_unset_all(key: &str) -> Result<()> {
    let _ = Command::new("git")
        .args(["config", "--global", "--unset-all", key])
        .status();
    Ok(())
}

/// Get all values of a multi-valued git config key.
pub fn config_get_all(key: &str) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["config", "--global", "--get-all", key])
        .output()
        .context("running git config --get-all")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect())
    } else {
        Ok(Vec::new())
    }
}

/// Add a value to a multi-valued git config key.
pub fn config_add(key: &str, value: &str) -> Result<()> {
    let status = Command::new("git")
        .args(["config", "--global", "--add", key, value])
        .status()
        .context("running git config --global --add")?;
    if !status.success() {
        anyhow::bail!("git config --global --add {} failed", key);
    }
    Ok(())
}

/// Shell-quote a string for embedding in a shell command.
fn shell_single_quote(s: &str) -> String {
    s.replace('\'', "'\\''")
}

/// Generate the shell snippet for the credential helper.
pub fn helper_shell_snippet(helper_path: &Path) -> String {
    let quoted = shell_single_quote(&helper_path.to_string_lossy());
    format!(
        "!f(){{ helper=$1; shift; \"$helper\" \"$@\"; }}; f '{}' \"$@\"",
        quoted
    )
}

/// Install the flox git credential helper for github.com.
pub fn configure_github_https_helper(helper_path: &Path) -> Result<()> {
    let key = "credential.https://github.com.helper";
    config_unset_all(key)?;
    config_add(key, &helper_shell_snippet(helper_path))
}

/// Clear the github.com credential helper from git config.
pub fn clear_github_https_git_config() -> Result<()> {
    config_unset_all("credential.https://github.com.helper")
}

/// Backup a git config key's values.
pub fn backup_config_key(key: &str) -> Result<Vec<String>> {
    config_get_all(key)
}

/// Restore a git config key's values from backup.
pub fn restore_config_key(key: &str, values: &[String]) -> Result<()> {
    config_unset_all(key)?;
    for v in values {
        if !v.is_empty() {
            config_add(key, v)?;
        }
    }
    Ok(())
}

/// Get the current gh git_protocol for github.com.
pub fn current_gh_git_protocol() -> Result<Option<String>> {
    let output = Command::new("gh")
        .args(["config", "get", "git_protocol", "--host", "github.com"])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if v.is_empty() {
                Ok(None)
            } else {
                Ok(Some(v))
            }
        }
        _ => Ok(None),
    }
}

/// Set the gh git_protocol for github.com.
pub fn set_gh_git_protocol(mode: &str) -> Result<bool> {
    let status = Command::new("gh")
        .args(["config", "set", "git_protocol", mode, "--host", "github.com"])
        .status();

    match status {
        Ok(s) if s.success() => {
            // Verify
            match current_gh_git_protocol()? {
                Some(v) if v == mode => Ok(true),
                _ => Ok(false),
            }
        }
        _ => Ok(false),
    }
}

/// Verify git credential helper resolves correct credentials.
pub fn healthcheck_git_https(username: &str, password: &str) -> Result<bool> {
    let input = format!("protocol=https\nhost=github.com\nusername={}\n\n", username);
    let output = Command::new("git")
        .args(["credential", "fill"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                stdin.write_all(input.as_bytes()).ok();
            }
            child.wait_with_output()
        });

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let has_user = stdout.lines().any(|l| l == format!("username={}", username));
            let has_pass = stdout.lines().any(|l| l == format!("password={}", password));
            if has_user && has_pass {
                crate::github_api::validate_basic_auth(username, password)
            } else {
                Ok(false)
            }
        }
        Err(_) => Ok(false),
    }
}

/// Rewrite a GitHub remote URL between HTTPS and SSH formats.
pub fn rewrite_github_remote_url(mode: &str, url: &str) -> String {
    let https_re = Regex::new(r"^https://github\.com/(.+)$").unwrap();
    let ssh_scp_re = Regex::new(r"^git@github\.com:(.+)$").unwrap();
    let ssh_url_re = Regex::new(r"^ssh://git@github\.com/(.+)$").unwrap();

    match mode {
        "ssh" => {
            if let Some(caps) = https_re.captures(url) {
                return format!("git@github.com:{}", &caps[1]);
            }
            if let Some(caps) = ssh_url_re.captures(url) {
                return format!("git@github.com:{}", &caps[1]);
            }
        }
        "https" => {
            if let Some(caps) = ssh_scp_re.captures(url) {
                return format!("https://github.com/{}", &caps[1]);
            }
            if let Some(caps) = ssh_url_re.captures(url) {
                return format!("https://github.com/{}", &caps[1]);
            }
        }
        _ => {}
    }
    url.to_string()
}

/// Rewrite all GitHub remotes in a repository.
pub fn rewrite_repo_remotes(repo: &Path, mode: &str) -> Result<()> {
    // Check it's a git repo
    let check = Command::new("git")
        .args(["-C", &repo.to_string_lossy(), "rev-parse", "--is-inside-work-tree"])
        .output();
    match check {
        Ok(o) if o.status.success() => {}
        _ => {
            eprintln!("Warning: Skipping non-repo path: {}", repo.display());
            return Ok(());
        }
    }

    let output = Command::new("git")
        .args(["-C", &repo.to_string_lossy(), "remote"])
        .output()
        .context("listing git remotes")?;

    let remotes: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect();

    for remote in &remotes {
        let fetch_url = get_remote_url(repo, remote, false)?;
        let push_url = get_remote_url(repo, remote, true)?;

        if let Some(ref url) = fetch_url {
            let new_url = rewrite_github_remote_url(mode, url);
            if new_url != *url {
                set_remote_url(repo, remote, &new_url, false)?;
            }
        }
        if let Some(ref url) = push_url {
            let new_url = rewrite_github_remote_url(mode, url);
            if new_url != *url {
                set_remote_url(repo, remote, &new_url, true)?;
            }
        }
    }

    Ok(())
}

fn get_remote_url(repo: &Path, remote: &str, push: bool) -> Result<Option<String>> {
    let repo_str = repo.to_string_lossy();
    let mut args = vec!["-C", &*repo_str, "remote", "get-url"];
    if push {
        args.push("--push");
    }
    args.push(remote);

    let output = Command::new("git")
        .args(&args)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let url = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if url.is_empty() {
                Ok(None)
            } else {
                Ok(Some(url))
            }
        }
        _ => Ok(None),
    }
}

fn set_remote_url(repo: &Path, remote: &str, url: &str, push: bool) -> Result<()> {
    let repo_str = repo.to_string_lossy();
    let mut args = vec!["-C", &*repo_str, "remote", "set-url"];
    if push {
        args.push("--push");
    }
    args.push(remote);
    args.push(url);

    Command::new("git")
        .args(&args)
        .status()
        .context("setting remote URL")?;
    Ok(())
}

/// Run `gh api user --jq .login` with a token to healthcheck gh.
pub fn healthcheck_gh(token: &str) -> Result<bool> {
    let output = Command::new("gh")
        .args(["api", "user", "--jq", ".login"])
        .env("GH_TOKEN", token)
        .env("GITHUB_TOKEN", token)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    match output {
        Ok(s) => Ok(s.success()),
        Err(_) => Ok(false),
    }
}

/// Get GitHub login from token via gh CLI.
pub fn github_login_from_token(token: &str) -> Result<Option<String>> {
    let output = Command::new("gh")
        .args(["api", "user", "--jq", ".login"])
        .env("GH_TOKEN", token)
        .env("GITHUB_TOKEN", token)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let login = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if login.is_empty() {
                Ok(None)
            } else {
                Ok(Some(login))
            }
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_https_to_ssh() {
        assert_eq!(
            rewrite_github_remote_url("ssh", "https://github.com/user/repo.git"),
            "git@github.com:user/repo.git"
        );
    }

    #[test]
    fn test_rewrite_ssh_to_https() {
        assert_eq!(
            rewrite_github_remote_url("https", "git@github.com:user/repo.git"),
            "https://github.com/user/repo.git"
        );
    }

    #[test]
    fn test_rewrite_ssh_url_to_https() {
        assert_eq!(
            rewrite_github_remote_url("https", "ssh://git@github.com/user/repo.git"),
            "https://github.com/user/repo.git"
        );
    }

    #[test]
    fn test_rewrite_noop() {
        assert_eq!(
            rewrite_github_remote_url("ssh", "git@github.com:user/repo.git"),
            "git@github.com:user/repo.git"
        );
        assert_eq!(
            rewrite_github_remote_url("https", "https://github.com/user/repo.git"),
            "https://github.com/user/repo.git"
        );
    }

    #[test]
    fn test_rewrite_non_github() {
        assert_eq!(
            rewrite_github_remote_url("ssh", "https://gitlab.com/user/repo.git"),
            "https://gitlab.com/user/repo.git"
        );
    }

    #[test]
    fn test_helper_shell_snippet() {
        let path = Path::new("/home/user/.config/gh/flox/git-credential-flox-helper");
        let snippet = helper_shell_snippet(path);
        assert!(snippet.contains("git-credential-flox-helper"));
        assert!(snippet.starts_with("!f(){"));
    }
}
