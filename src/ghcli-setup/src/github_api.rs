use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};

const GITHUB_API_BASE: &str = "https://api.github.com";
const GITHUB_API_VERSION: &str = "2022-11-28";

/// Validate a GitHub token by calling GET /user.
/// Returns Ok(false) on any error (including network), matching bash `|| true` behavior.
pub fn validate_token(token: &str) -> Result<bool> {
    let token: String = token.chars().filter(|c| !c.is_whitespace()).collect();
    if token.is_empty() {
        return Ok(false);
    }
    let resp = ureq::get(&format!("{}/user", GITHUB_API_BASE))
        .set("Authorization", &format!("Bearer {}", token))
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", GITHUB_API_VERSION)
        .call();

    match resp {
        Ok(r) => Ok(r.status() == 200),
        Err(_) => Ok(false),
    }
}

/// Validate basic auth (username:password) against GitHub.
pub fn validate_basic_auth(username: &str, password: &str) -> Result<bool> {
    let resp = ureq::get(&format!("{}/user", GITHUB_API_BASE))
        .set("Authorization", &format!("Basic {}", base64_encode(&format!("{}:{}", username, password))))
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", GITHUB_API_VERSION)
        .call();

    match resp {
        Ok(r) => Ok(r.status() == 200),
        Err(_) => Ok(false),
    }
}

#[derive(Debug, Deserialize)]
pub struct SshKey {
    pub id: u64,
    pub key: String,
    #[allow(dead_code)]
    pub title: String,
}

/// List all SSH keys for the authenticated user, handling pagination.
pub fn list_ssh_keys(token: &str) -> Result<Vec<SshKey>> {
    let mut all_keys = Vec::new();
    let mut url = format!("{}/user/keys?per_page=100", GITHUB_API_BASE);

    loop {
        let resp = ureq::get(&url)
            .set("Authorization", &format!("Bearer {}", token))
            .set("Accept", "application/vnd.github+json")
            .set("X-GitHub-Api-Version", GITHUB_API_VERSION)
            .call()
            .context("listing SSH keys")?;

        // Parse Link header for pagination
        let link_header = resp.header("Link").map(String::from);
        let keys: Vec<SshKey> = resp.into_json()?;
        all_keys.extend(keys);

        match link_header.and_then(|h| parse_next_link(&h)) {
            Some(next) => url = next,
            None => break,
        }
    }

    Ok(all_keys)
}

/// Find an SSH key ID by its public key content.
pub fn find_ssh_key_id(keys: &[SshKey], pubkey: &str) -> Option<u64> {
    let pubkey = pubkey.trim();
    keys.iter().find(|k| k.key.trim() == pubkey).map(|k| k.id)
}

#[derive(Serialize)]
struct UploadKeyRequest<'a> {
    title: &'a str,
    key: &'a str,
}

/// Upload an SSH public key to GitHub. Returns the key ID.
pub fn upload_ssh_key(token: &str, pubkey: &str, title: &str) -> Result<u64> {
    let body = UploadKeyRequest {
        title,
        key: pubkey.trim(),
    };

    let resp = ureq::post(&format!("{}/user/keys", GITHUB_API_BASE))
        .set("Authorization", &format!("Bearer {}", token))
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", GITHUB_API_VERSION)
        .send_json(&body)
        .context("uploading SSH key")?;

    let result: serde_json::Value = resp.into_json()?;
    result
        .get("id")
        .and_then(|v| v.as_u64())
        .context("SSH key upload response missing id")
}

/// Delete an SSH key by ID.
pub fn delete_ssh_key(token: &str, id: u64) -> Result<()> {
    let _ = ureq::delete(&format!("{}/user/keys/{}", GITHUB_API_BASE, id))
        .set("Authorization", &format!("Bearer {}", token))
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", GITHUB_API_VERSION)
        .call()
        .context("deleting SSH key")?;
    Ok(())
}

/// Parse the `next` URL from a Link header.
fn parse_next_link(header: &str) -> Option<String> {
    let re = Regex::new(r#"<([^>]+)>;\s*rel="next""#).ok()?;
    re.captures(header)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
}

/// Simple base64 encoding for basic auth.
fn base64_encode(input: &str) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut result = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i] as u32;
        let b1 = if i + 1 < bytes.len() { bytes[i + 1] as u32 } else { 0 };
        let b2 = if i + 2 < bytes.len() { bytes[i + 2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if i + 1 < bytes.len() {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if i + 2 < bytes.len() {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        i += 3;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_next_link() {
        let header = r#"<https://api.github.com/user/keys?page=2&per_page=100>; rel="next", <https://api.github.com/user/keys?page=3&per_page=100>; rel="last""#;
        assert_eq!(
            parse_next_link(header),
            Some("https://api.github.com/user/keys?page=2&per_page=100".to_string())
        );
    }

    #[test]
    fn test_parse_next_link_none() {
        let header = r#"<https://api.github.com/user/keys?page=3&per_page=100>; rel="last""#;
        assert_eq!(parse_next_link(header), None);
    }

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode("user:pass"), "dXNlcjpwYXNz");
        assert_eq!(base64_encode("a"), "YQ==");
        assert_eq!(base64_encode("ab"), "YWI=");
        assert_eq!(base64_encode("abc"), "YWJj");
    }
}
