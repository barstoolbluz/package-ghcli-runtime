use anyhow::Result;
use std::path::Path;

use crate::config::atomic_write;

/// Write the gh-token-helper script.
pub fn write_token_helper(path: &Path) -> Result<()> {
    atomic_write(path, TOKEN_HELPER.as_bytes(), 0o700)
}

/// Write the git-credential-flox-helper script.
pub fn write_git_helper(path: &Path) -> Result<()> {
    atomic_write(path, GIT_HELPER.as_bytes(), 0o700)
}

/// Write all three shell wrappers.
pub fn write_shell_wrappers(bash: &Path, zsh: &Path, fish: &Path) -> Result<()> {
    atomic_write(bash, BASH_WRAPPER.as_bytes(), 0o600)?;
    atomic_write(zsh, ZSH_WRAPPER.as_bytes(), 0o600)?;
    atomic_write(fish, FISH_WRAPPER.as_bytes(), 0o600)?;
    Ok(())
}

/// Write all runtime assets (token helper, git helper, shell wrappers).
pub fn write_all(paths: &crate::paths::Paths) -> Result<()> {
    write_token_helper(&paths.token_helper)?;
    write_git_helper(&paths.git_helper)?;
    write_shell_wrappers(&paths.bash_wrapper, &paths.zsh_wrapper, &paths.fish_wrapper)?;
    Ok(())
}

const TOKEN_HELPER: &str = r#"#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/gh/flox"
CONFIG_FILE="$CONFIG_DIR/github_config"
TOKEN_ENC_FILE="$CONFIG_DIR/github_token.enc"
LOCAL_KEY_FILE="$CONFIG_DIR/.local_key"
TOKEN_SERVICE="flox-github"

config_get() {
    local key="$1"
    [[ -f "$CONFIG_FILE" ]] || return 1
    awk -F= -v key="$key" '$1 == key { print substr($0, index($0, "=") + 1) }' "$CONFIG_FILE" | tail -n1
}

local_key() {
    [[ -f "$LOCAL_KEY_FILE" ]] || return 1
    cat "$LOCAL_KEY_FILE"
}

os_type() {
    case "$(uname -s)" in
        Darwin) printf 'macos\n' ;;
        Linux) printf 'linux\n' ;;
        *) printf 'unsupported\n' ;;
    esac
}

get_keyring_token() {
    case "$(os_type)" in
        macos)
            security find-generic-password -s "$TOKEN_SERVICE" -a "$USER" -w 2>/dev/null || true
            ;;
        linux)
            if command -v secret-tool >/dev/null 2>&1; then
                secret-tool lookup service "$TOKEN_SERVICE" user "$USER" 2>/dev/null || true
            fi
            ;;
    esac
}

get_file_token() {
    [[ -f "$TOKEN_ENC_FILE" ]] || return 1
    [[ -f "$LOCAL_KEY_FILE" ]] || return 1
    openssl enc -aes-256-cbc -d -pbkdf2 -iter 10000 -md sha256 -pass "pass:$(local_key)" -in "$TOKEN_ENC_FILE" 2>/dev/null || true
}

main() {
    local storage_method=''
    storage_method="$(config_get TOKEN_STORAGE 2>/dev/null || true)"
    local token=''
    case "$storage_method" in
        keyring) token="$(get_keyring_token)" ;;
        file)    token="$(get_file_token)" ;;
        *)
            # Config missing or corrupt — probe both backends
            token="$(get_keyring_token)"
            if [[ -z "$token" ]]; then
                token="$(get_file_token)" || true
            fi
            ;;
    esac
    if [[ -n "$token" ]]; then
        printf '%s' "$token"
    else
        exit 1
    fi
}

main "$@"
"#;

const GIT_HELPER: &str = r#"#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/gh/flox"
CONFIG_FILE="$CONFIG_DIR/github_config"
GIT_CREDENTIALS_ENC_FILE="$CONFIG_DIR/git_credentials.enc"
LOCAL_KEY_FILE="$CONFIG_DIR/.local_key"
GIT_SERVICE="flox-github-git"

mkdir -p "$CONFIG_DIR"
chmod 700 "$CONFIG_DIR"

config_get() {
    local key="$1"
    [[ -f "$CONFIG_FILE" ]] || return 1
    awk -F= -v key="$key" '$1 == key { print substr($0, index($0, "=") + 1) }' "$CONFIG_FILE" | tail -n1
}

os_type() {
    case "$(uname -s)" in
        Darwin) printf 'macos\n' ;;
        Linux) printf 'linux\n' ;;
        *) printf 'unsupported\n' ;;
    esac
}

local_key() {
    [[ -f "$LOCAL_KEY_FILE" ]] || return 1
    cat "$LOCAL_KEY_FILE"
}

init_local_key_file() {
    if [[ ! -f "$LOCAL_KEY_FILE" ]]; then
        command -v openssl >/dev/null 2>&1 || return 1
        umask 077
        openssl rand -hex 32 >"$LOCAL_KEY_FILE"
        chmod 600 "$LOCAL_KEY_FILE"
    fi
}

read_request() {
    local line=''
    while IFS= read -r line; do
        [[ -z "$line" ]] && break
        printf '%s\n' "$line"
    done
}

parse_field() {
    local body="$1"
    local key="$2"
    awk -F= -v key="$key" '$1 == key { print substr($0, index($0, "=") + 1); exit }' <<<"$body"
}

clear_keyring_creds() {
    case "$(os_type)" in
        macos)
            security delete-generic-password -s "$GIT_SERVICE" -a "$USER" >/dev/null 2>&1 || true
            ;;
        linux)
            if command -v secret-tool >/dev/null 2>&1; then
                secret-tool clear service "$GIT_SERVICE" user "$USER" host github.com >/dev/null 2>&1 || true
            fi
            ;;
    esac
}

read_keyring_creds() {
    case "$(os_type)" in
        macos)
            security find-generic-password -s "$GIT_SERVICE" -a "$USER" -w 2>/dev/null || true
            ;;
        linux)
            if command -v secret-tool >/dev/null 2>&1; then
                secret-tool lookup service "$GIT_SERVICE" user "$USER" host github.com 2>/dev/null || true
            fi
            ;;
    esac
}

write_keyring_creds() {
    local payload="$1"
    case "$(os_type)" in
        macos)
            security add-generic-password -U -s "$GIT_SERVICE" -a "$USER" -w "$payload" >/dev/null
            ;;
        linux)
            command -v secret-tool >/dev/null 2>&1 || return 1
            printf '%s' "$payload" | secret-tool store --label 'Flox GitHub Git Credentials' service "$GIT_SERVICE" user "$USER" host github.com >/dev/null
            ;;
        *)
            return 1
            ;;
    esac
}

clear_file_creds() {
    rm -f "$GIT_CREDENTIALS_ENC_FILE" >/dev/null 2>&1 || true
}

write_file_creds() {
    local payload="$1"
    local tmp=''

    command -v openssl >/dev/null 2>&1 || return 1
    init_local_key_file || return 1
    tmp="$(mktemp "$CONFIG_DIR/.git-creds.XXXXXX")"
    printf '%s' "$payload" | openssl enc -aes-256-cbc -salt -pbkdf2 -iter 10000 -md sha256 -pass "pass:$(local_key)" -out "$tmp"
    chmod 600 "$tmp"
    mv -f "$tmp" "$GIT_CREDENTIALS_ENC_FILE"
}

read_file_creds() {
    [[ -f "$GIT_CREDENTIALS_ENC_FILE" ]] || return 1
    [[ -f "$LOCAL_KEY_FILE" ]] || return 1
    openssl enc -aes-256-cbc -d -pbkdf2 -iter 10000 -md sha256 -pass "pass:$(local_key)" -in "$GIT_CREDENTIALS_ENC_FILE" 2>/dev/null || true
}

emit_get_response() {
    local storage_method="$1"
    local request_body="$2"
    local host=''
    local requested_user=''
    local creds=''
    local stored_user=''
    local stored_pass=''

    host="$(parse_field "$request_body" host)"
    requested_user="$(parse_field "$request_body" username)"
    [[ "$host" == 'github.com' ]] || exit 0

    case "$storage_method" in
        keyring) creds="$(read_keyring_creds)" ;;
        file) creds="$(read_file_creds)" ;;
        *) exit 0 ;;
    esac

    [[ "$creds" == *:* ]] || exit 0
    stored_user="${creds%%:*}"
    stored_pass="${creds#*:}"

    if [[ -n "$requested_user" && "$requested_user" != "$stored_user" ]]; then
        exit 0
    fi

    printf 'username=%s\n' "$stored_user"
    printf 'password=%s\n' "$stored_pass"
}

handle_store() {
    local storage_method="$1"
    local request_body="$2"
    local host=''
    local username=''
    local password=''
    local payload=''

    host="$(parse_field "$request_body" host)"
    username="$(parse_field "$request_body" username)"
    password="$(parse_field "$request_body" password)"
    [[ "$host" == 'github.com' ]] || exit 0
    [[ -n "$username" && -n "$password" ]] || exit 0
    payload="${username}:${password}"

    case "$storage_method" in
        keyring) write_keyring_creds "$payload" || true ;;
        file) write_file_creds "$payload" || true ;;
        *) ;;
    esac
}

handle_erase() {
    local storage_method="$1"
    local request_body="$2"
    local host=''

    host="$(parse_field "$request_body" host)"
    [[ "$host" == 'github.com' ]] || exit 0

    case "$storage_method" in
        keyring) clear_keyring_creds ;;
        file) clear_file_creds ;;
        *) ;;
    esac
}

main() {
    local action="${1:-}"
    local storage_method=''
    local request_body=''

    storage_method="$(config_get GIT_CREDENTIAL_STORAGE 2>/dev/null || true)"
    request_body="$(read_request || true)"

    case "$action" in
        get) emit_get_response "$storage_method" "$request_body" ;;
        store) handle_store "$storage_method" "$request_body" ;;
        erase) handle_erase "$storage_method" "$request_body" ;;
        *) exit 0 ;;
    esac
}

main "$@"
"#;

const BASH_WRAPPER: &str = r#"gh() {
    local helper="${XDG_CONFIG_HOME:-$HOME/.config}/gh/flox/gh-token-helper"
    local token=''

    token="$("$helper" 2>/dev/null || true)"
    if [[ -z "$token" ]]; then
        printf 'Error: GitHub token not available. Run the setup wizard again.\n' >&2
        return 1
    fi

    GH_TOKEN="$token" GITHUB_TOKEN="$token" command gh "$@"
}
"#;

const ZSH_WRAPPER: &str = r#"gh() {
    local helper="${XDG_CONFIG_HOME:-$HOME/.config}/gh/flox/gh-token-helper"
    local token=''

    token="$("$helper" 2>/dev/null || true)"
    if [[ -z "$token" ]]; then
        printf 'Error: GitHub token not available. Run the setup wizard again.\n' >&2
        return 1
    fi

    GH_TOKEN="$token" GITHUB_TOKEN="$token" command gh "$@"
}
"#;

const FISH_WRAPPER: &str = r#"function gh
    if test -n "$XDG_CONFIG_HOME"
        set -l config_base "$XDG_CONFIG_HOME"
    else
        set -l config_base "$HOME/.config"
    end

    set -l helper "$config_base/gh/flox/gh-token-helper"
    set -l token ("$helper" 2>/dev/null)

    if test -z "$token"
        echo "Error: GitHub token not available. Run the setup wizard again." >&2
        return 1
    end

    env GH_TOKEN="$token" GITHUB_TOKEN="$token" command gh $argv
end
"#;
