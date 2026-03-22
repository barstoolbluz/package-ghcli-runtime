#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'
umask 077

CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/gh/flox"
CONFIG_FILE="$CONFIG_DIR/github_config"
TOKEN_ENC_FILE="$CONFIG_DIR/github_token.enc"
GIT_CREDENTIALS_ENC_FILE="$CONFIG_DIR/git_credentials.enc"
LOCAL_KEY_FILE="$CONFIG_DIR/.local_key"
TOKEN_HELPER="$CONFIG_DIR/gh-token-helper"
GIT_HELPER="$CONFIG_DIR/git-credential-flox-helper"
BASH_WRAPPER="$CONFIG_DIR/gh_wrapper.bash"
ZSH_WRAPPER="$CONFIG_DIR/gh_wrapper.zsh"
FISH_WRAPPER="$CONFIG_DIR/gh_wrapper.fish"
LOCK_FILE="$CONFIG_DIR/.setup.lock"
LOCK_DIR="$CONFIG_DIR/.setup.lock.d"
DEFAULT_SSH_KEY_PATH="$HOME/.ssh/id_ed25519_flox_github"
SSH_CONFIG_FILE="$HOME/.ssh/config"
GITHUB_TOKEN_SERVICE="flox-github"
GITHUB_GIT_SERVICE="flox-github-git"
MANAGED_SSH_BLOCK_START='# >>> flox github ssh >>>'
MANAGED_SSH_BLOCK_END='# <<< flox github ssh <<<'

readonly CONFIG_DIR CONFIG_FILE TOKEN_ENC_FILE GIT_CREDENTIALS_ENC_FILE LOCAL_KEY_FILE
readonly TOKEN_HELPER GIT_HELPER BASH_WRAPPER ZSH_WRAPPER FISH_WRAPPER LOCK_FILE LOCK_DIR
readonly DEFAULT_SSH_KEY_PATH SSH_CONFIG_FILE GITHUB_TOKEN_SERVICE GITHUB_GIT_SERVICE
readonly MANAGED_SSH_BLOCK_START MANAGED_SSH_BLOCK_END

CURRENT_TMP_FILE=""
LOCK_DIR_CREATED=0
STAGED_TOKEN_SERVICE=""
STAGED_GIT_SERVICE=""
TOKEN_FILE_BACKUP=""
TOKEN_FILE_HAD_OLD=0
GIT_FILE_BACKUP=""
GIT_FILE_HAD_OLD=0
TOKEN_KEYRING_BACKUP=""
TOKEN_KEYRING_HAD_OLD=0
GIT_KEYRING_BACKUP=""
GIT_KEYRING_HAD_OLD=0
SSH_UPLOADED_KEY_ID=""
SSH_SMOKE_ROUTE=""

NONINTERACTIVE=0
AUTO_YES=0
FILE_FALLBACK_POLICY="${FLOX_FILE_FALLBACK_POLICY:-}"
CLI_TOKEN="${FLOX_GITHUB_TOKEN:-}"
CLI_GIT_MODE="${FLOX_GIT_MODE:-}"
CLI_GIT_USERNAME="${FLOX_GIT_USERNAME:-}"
CLI_GIT_PASSWORD="${FLOX_GIT_PASSWORD:-}"
CLI_SSH_KEY_PATH="${FLOX_SSH_KEY_PATH:-}"
CLI_SSH_KEY_TITLE="${FLOX_SSH_KEY_TITLE:-}"
CLI_GIT_USER_NAME="${FLOX_GIT_USER_NAME:-}"
CLI_GIT_USER_EMAIL="${FLOX_GIT_USER_EMAIL:-}"
REWRITE_REMOTES=0
REPO_PATHS=()

cleanup() {
    if [[ -n "$CURRENT_TMP_FILE" && -e "$CURRENT_TMP_FILE" ]]; then
        rm -f "$CURRENT_TMP_FILE" || true
    fi
    if [[ -n "$STAGED_TOKEN_SERVICE" ]]; then
        clear_keyring_secret "$STAGED_TOKEN_SERVICE" || true
    fi
    if [[ -n "$STAGED_GIT_SERVICE" ]]; then
        clear_keyring_secret "$STAGED_GIT_SERVICE" github.com || true
    fi
    if [[ -n "$TOKEN_FILE_BACKUP" && -e "$TOKEN_FILE_BACKUP" ]]; then
        rm -f "$TOKEN_FILE_BACKUP" || true
    fi
    if [[ -n "$GIT_FILE_BACKUP" && -e "$GIT_FILE_BACKUP" ]]; then
        rm -f "$GIT_FILE_BACKUP" || true
    fi
    if [[ "$LOCK_DIR_CREATED" -eq 1 && -d "$LOCK_DIR" ]]; then
        rmdir "$LOCK_DIR" >/dev/null 2>&1 || true
    fi
}

die() {
    printf 'Error: %s\n' "$*" >&2
    exit 1
}

warn() {
    printf 'Warning: %s\n' "$*" >&2
}

have_cmd() {
    command -v "$1" >/dev/null 2>&1
}

need_cmd() {
    have_cmd "$1" || die "$1 is required"
}

os_type() {
    case "$(uname -s)" in
        Darwin) printf 'macos\n' ;;
        Linux) printf 'linux\n' ;;
        *) printf 'unsupported\n' ;;
    esac
}

trim_whitespace() {
    sed 's/^[[:space:]]*//; s/[[:space:]]*$//'
}

same_dir_tmp() {
    local dir="$1"
    CURRENT_TMP_FILE="$(mktemp "$dir/.tmp.XXXXXX")"
    printf '%s\n' "$CURRENT_TMP_FILE"
}

atomic_write_file() {
    local target="$1"
    local mode="$2"
    local target_dir=''
    local tmp=''

    target_dir="$(dirname "$target")"
    mkdir -p "$target_dir"
    chmod 700 "$target_dir" 2>/dev/null || true
    tmp="$(same_dir_tmp "$target_dir")"
    cat >"$tmp"
    chmod "$mode" "$tmp"
    mv -f "$tmp" "$target"
    CURRENT_TMP_FILE=''
}

acquire_lock() {
    if have_cmd flock; then
        exec 9>"$LOCK_FILE"
        flock 9
        return 0
    fi

    while ! mkdir "$LOCK_DIR" >/dev/null 2>&1; do
        sleep 1
    done
    LOCK_DIR_CREATED=1
}

init_runtime() {
    mkdir -p "$CONFIG_DIR"
    chmod 700 "$CONFIG_DIR"
    acquire_lock
    trap cleanup EXIT
    trap 'exit 130' INT TERM
}

config_get() {
    local key="$1"
    [[ -f "$CONFIG_FILE" ]] || return 1
    awk -F= -v key="$key" '$1 == key { print substr($0, index($0, "=") + 1) }' "$CONFIG_FILE" | tail -n1
}

config_set() {
    local key="$1"
    local value="$2"
    local tmp=''

    tmp="$(same_dir_tmp "$CONFIG_DIR")"
    if [[ -f "$CONFIG_FILE" ]]; then
        awk -F= -v key="$key" '$1 != key { print }' "$CONFIG_FILE" >"$tmp"
    fi
    printf '%s=%s\n' "$key" "$value" >>"$tmp"
    chmod 600 "$tmp"
    mv -f "$tmp" "$CONFIG_FILE"
    CURRENT_TMP_FILE=''
}

config_unset() {
    local key="$1"
    local tmp=''

    [[ -f "$CONFIG_FILE" ]] || return 0
    tmp="$(same_dir_tmp "$CONFIG_DIR")"
    awk -F= -v key="$key" '$1 != key { print }' "$CONFIG_FILE" >"$tmp"
    chmod 600 "$tmp"
    mv -f "$tmp" "$CONFIG_FILE"
    CURRENT_TMP_FILE=''
}

init_local_key_file() {
    if [[ ! -f "$LOCAL_KEY_FILE" ]]; then
        need_cmd openssl
        openssl rand -hex 32 | atomic_write_file "$LOCAL_KEY_FILE" 600
    fi
}

local_key() {
    [[ -f "$LOCAL_KEY_FILE" ]] || return 1
    cat "$LOCAL_KEY_FILE"
}

encrypt_to_file() {
    local plaintext="$1"
    local outfile="$2"
    local tmp=''

    need_cmd openssl
    init_local_key_file
    tmp="$(same_dir_tmp "$(dirname "$outfile")")"
    printf '%s' "$plaintext" | openssl enc -aes-256-cbc -salt -pbkdf2 -md sha256 -pass "pass:$(local_key)" -out "$tmp"
    chmod 600 "$tmp"
    mv -f "$tmp" "$outfile"
    CURRENT_TMP_FILE=''
}

decrypt_from_file() {
    local infile="$1"

    [[ -f "$infile" ]] || return 1
    [[ -f "$LOCAL_KEY_FILE" ]] || return 1
    need_cmd openssl
    openssl enc -aes-256-cbc -d -pbkdf2 -md sha256 -pass "pass:$(local_key)" -in "$infile" 2>/dev/null
}

normalize_reply() {
    tr '[:upper:]' '[:lower:]'
}

ui_clear() {
    if [[ -t 1 && -n "${TERM:-}" ]] && have_cmd clear; then
        clear || true
    fi
}

ui_confirm() {
    local prompt="$1"
    local default="${2:-true}"
    local suffix='[Y/n/exit]'
    local reply=''
    local normalized=''

    if [[ "$default" == 'false' ]]; then
        suffix='[y/N/exit]'
    fi

    if [[ "$NONINTERACTIVE" -eq 1 ]]; then
        [[ "$AUTO_YES" -eq 1 && "$default" == 'true' ]]
        return $?
    fi

    if have_cmd gum; then
        while true; do
            reply="$(gum input --prompt "$prompt $suffix " --placeholder 'y, n, or exit' 2>/dev/null || true)"
            normalized="$(printf '%s' "$reply" | normalize_reply)"
            case "$normalized" in
                y|yes) return 0 ;;
                n|no|exit|quit) return 1 ;;
                '') [[ "$default" == 'true' ]] && return 0 || return 1 ;;
            esac
        done
    fi

    while true; do
        printf '%s %s ' "$prompt" "$suffix" >&2
        read -r reply || return 1
        normalized="$(printf '%s' "$reply" | normalize_reply)"
        case "$normalized" in
            y|yes) return 0 ;;
            n|no|exit|quit) return 1 ;;
            '') [[ "$default" == 'true' ]] && return 0 || return 1 ;;
        esac
    done
}

ui_input() {
    local prompt="$1"
    local placeholder="${2:-}"
    local value=''

    [[ "$NONINTERACTIVE" -eq 0 ]] || return 1

    if have_cmd gum; then
        value="$(gum input --prompt "$prompt " --placeholder "$placeholder" 2>/dev/null || true)"
        printf '%s\n' "$value"
        return 0
    fi

    printf '%s ' "$prompt" >&2
    read -r value || return 1
    printf '%s\n' "$value"
}

ui_secret() {
    local prompt="$1"
    local value=''

    [[ "$NONINTERACTIVE" -eq 0 ]] || return 1

    if have_cmd gum; then
        value="$(gum input --prompt "$prompt " --password 2>/dev/null || true)"
        printf '%s\n' "$value"
        return 0
    fi

    printf '%s ' "$prompt" >&2
    read -r -s value || return 1
    printf '\n' >&2
    printf '%s\n' "$value"
}

ui_select() {
    local prompt="$1"
    shift
    local options=("$@")
    local reply=''
    local i=''

    [[ "$NONINTERACTIVE" -eq 0 ]] || return 1

    if have_cmd gum; then
        gum choose "${options[@]}" --header "$prompt" 2>/dev/null || return 1
        return 0
    fi

    printf '%s\n' "$prompt" >&2
    for i in "${!options[@]}"; do
        printf '  %d) %s\n' "$((i + 1))" "${options[$i]}" >&2
    done
    printf 'Select a number or type exit: ' >&2
    read -r reply || return 1
    case "$reply" in
        exit|quit) return 1 ;;
    esac
    if [[ "$reply" =~ ^[0-9]+$ ]] && (( reply >= 1 && reply <= ${#options[@]} )); then
        printf '%s\n' "${options[$((reply - 1))]}"
        return 0
    fi
    return 1
}

show_usage() {
    cat <<'TEXT'
Usage: flox_github_setup_prod_v7.sh [options]

Options:
  --non-interactive           Disable prompts; missing required inputs become errors.
  --yes                       Accept default yes/no prompts where the default is yes.
  --token VALUE               GitHub token for gh.
  --git-mode https|ssh|skip   Git mode for github.com.
  --git-username NAME         GitHub username for HTTPS Git.
  --git-password VALUE        Token for HTTPS Git. Defaults to --token when omitted.
  --ssh-key-path PATH         SSH private key path.
  --ssh-key-title TITLE       SSH key title for upload.
  --git-user-name NAME        Set git config --global user.name.
  --git-user-email EMAIL      Set git config --global user.email.
  --file-fallback never|prompt|always
                              Policy for encrypted local-file fallback.
  --rewrite-remotes           Rewrite GitHub remotes in the listed repos after setup.
  --repo PATH                 Repo path to rewrite. Repeat this flag for multiple repos.
  --help                      Show this help.

Environment variables:
  FLOX_GITHUB_TOKEN
  FLOX_GIT_MODE
  FLOX_GIT_USERNAME
  FLOX_GIT_PASSWORD
  FLOX_SSH_KEY_PATH
  FLOX_SSH_KEY_TITLE
  FLOX_GIT_USER_NAME
  FLOX_GIT_USER_EMAIL
  FLOX_FILE_FALLBACK_POLICY
TEXT
}

parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --non-interactive)
                NONINTERACTIVE=1
                ;;
            --yes)
                AUTO_YES=1
                ;;
            --token)
                shift
                [[ $# -gt 0 ]] || die '--token requires a value'
                CLI_TOKEN="$1"
                ;;
            --git-mode)
                shift
                [[ $# -gt 0 ]] || die '--git-mode requires a value'
                CLI_GIT_MODE="$1"
                ;;
            --git-username)
                shift
                [[ $# -gt 0 ]] || die '--git-username requires a value'
                CLI_GIT_USERNAME="$1"
                ;;
            --git-password)
                shift
                [[ $# -gt 0 ]] || die '--git-password requires a value'
                CLI_GIT_PASSWORD="$1"
                ;;
            --ssh-key-path)
                shift
                [[ $# -gt 0 ]] || die '--ssh-key-path requires a value'
                CLI_SSH_KEY_PATH="$1"
                ;;
            --ssh-key-title)
                shift
                [[ $# -gt 0 ]] || die '--ssh-key-title requires a value'
                CLI_SSH_KEY_TITLE="$1"
                ;;
            --git-user-name)
                shift
                [[ $# -gt 0 ]] || die '--git-user-name requires a value'
                CLI_GIT_USER_NAME="$1"
                ;;
            --git-user-email)
                shift
                [[ $# -gt 0 ]] || die '--git-user-email requires a value'
                CLI_GIT_USER_EMAIL="$1"
                ;;
            --file-fallback)
                shift
                [[ $# -gt 0 ]] || die '--file-fallback requires a value'
                FILE_FALLBACK_POLICY="$1"
                ;;
            --rewrite-remotes)
                REWRITE_REMOTES=1
                ;;
            --repo)
                shift
                [[ $# -gt 0 ]] || die '--repo requires a value'
                REPO_PATHS+=("$1")
                ;;
            --help|-h)
                show_usage
                exit 0
                ;;
            *)
                die "Unknown option: $1"
                ;;
        esac
        shift
    done

    case "$CLI_GIT_MODE" in
        ''|https|ssh|skip) ;;
        *) die '--git-mode must be https, ssh, or skip' ;;
    esac

    if [[ -z "$FILE_FALLBACK_POLICY" ]]; then
        if [[ "$NONINTERACTIVE" -eq 1 ]]; then
            FILE_FALLBACK_POLICY='never'
        else
            FILE_FALLBACK_POLICY='prompt'
        fi
    fi

    case "$FILE_FALLBACK_POLICY" in
        never|prompt|always) ;;
        *) die '--file-fallback must be never, prompt, or always' ;;
    esac
}

show_welcome_message() {
    cat <<'TEXT'
Flox GitHub setup

This wizard can set up:
  1. GitHub CLI access through a stored token
  2. Git access for github.com over HTTPS or SSH

Notes:
  - GitHub CLI itself still uses a token.
  - HTTPS Git should use a personal access token, not an account password.
  - SSH mode changes Git transport to SSH for GitHub CLI and can register an SSH key.
  - Type 'exit' or 'quit' at any input prompt to stop.
TEXT
    printf '\n'
}

show_token_instructions() {
    cat <<'TEXT'
Create or choose a GitHub personal access token before running this wizard.

Typical cases:
  - GitHub CLI access: token with the API rights you need
  - Git HTTPS with the same token: repo scope for classic PATs, or matching repo access for fine-grained PATs
  - Git SSH key upload through this wizard: token may need SSH-key rights
TEXT
    printf '\n'
}

show_file_storage_warning() {
    cat <<'TEXT'
Encrypted local-file fallback:
  - This is a local fallback when the OS secret store is not usable.
  - The encrypted data and the local key live under the same account.
  - Use this on a trusted machine only.
TEXT
    printf '\n'
}

show_completion_message() {
    local token_storage="$1"
    local git_mode="$2"
    local git_storage="$3"

    cat <<TEXT
Setup complete.

GitHub CLI token storage: $token_storage
Git mode for github.com:  $git_mode
Git secret storage:       $git_storage

To load the gh wrapper in Bash:
  source "$BASH_WRAPPER"

To load the gh wrapper in Zsh:
  source "$ZSH_WRAPPER"

To load the gh wrapper in Fish:
  source "$FISH_WRAPPER"
TEXT
    printf '\n'
}

write_token_helper() {
    atomic_write_file "$TOKEN_HELPER" 700 <<'EOF_HELPER'
#!/usr/bin/env bash
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
    openssl enc -aes-256-cbc -d -pbkdf2 -md sha256 -pass "pass:$(local_key)" -in "$TOKEN_ENC_FILE" 2>/dev/null || true
}

main() {
    local storage_method=''
    storage_method="$(config_get TOKEN_STORAGE 2>/dev/null || true)"

    case "$storage_method" in
        keyring) get_keyring_token ;;
        file) get_file_token ;;
        *) exit 1 ;;
    esac
}

main "$@"
EOF_HELPER
}

write_git_helper() {
    atomic_write_file "$GIT_HELPER" 700 <<'EOF_HELPER'
#!/usr/bin/env bash
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
    printf '%s' "$payload" | openssl enc -aes-256-cbc -salt -pbkdf2 -md sha256 -pass "pass:$(local_key)" -out "$tmp"
    chmod 600 "$tmp"
    mv -f "$tmp" "$GIT_CREDENTIALS_ENC_FILE"
}

read_file_creds() {
    [[ -f "$GIT_CREDENTIALS_ENC_FILE" ]] || return 1
    [[ -f "$LOCAL_KEY_FILE" ]] || return 1
    openssl enc -aes-256-cbc -d -pbkdf2 -md sha256 -pass "pass:$(local_key)" -in "$GIT_CREDENTIALS_ENC_FILE" 2>/dev/null || true
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
EOF_HELPER
}

write_shell_wrappers() {
    atomic_write_file "$BASH_WRAPPER" 600 <<'EOF_BASH'
gh() {
    local helper="${XDG_CONFIG_HOME:-$HOME/.config}/gh/flox/gh-token-helper"
    local token=''

    token="$("$helper" 2>/dev/null || true)"
    if [[ -z "$token" ]]; then
        printf 'Error: GitHub token not available. Run the setup wizard again.\n' >&2
        return 1
    fi

    GH_TOKEN="$token" GITHUB_TOKEN="$token" command gh "$@"
}
EOF_BASH

    atomic_write_file "$ZSH_WRAPPER" 600 <<'EOF_ZSH'
gh() {
    local helper="${XDG_CONFIG_HOME:-$HOME/.config}/gh/flox/gh-token-helper"
    local token=''

    token="$("$helper" 2>/dev/null || true)"
    if [[ -z "$token" ]]; then
        printf 'Error: GitHub token not available. Run the setup wizard again.\n' >&2
        return 1
    fi

    GH_TOKEN="$token" GITHUB_TOKEN="$token" command gh "$@"
}
EOF_ZSH

    atomic_write_file "$FISH_WRAPPER" 600 <<'EOF_FISH'
function gh
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
EOF_FISH
}

write_runtime_assets() {
    write_token_helper
    write_git_helper
    write_shell_wrappers
}

require_basic_tools() {
    need_cmd bash
    need_cmd curl
    need_cmd git
    need_cmd gh
    need_cmd jq
    need_cmd flock
}

host_shortname() {
    hostname | cut -d'.' -f1
}

validate_github_token() {
    local token="$1"
    local code=''

    code="$(curl -fsS -o /dev/null -w '%{http_code}' -H "Authorization: Bearer $token" -H 'Accept: application/vnd.github+json' -H 'X-GitHub-Api-Version: 2022-11-28' https://api.github.com/user || true)"
    [[ "$code" == '200' ]]
}

validate_github_basic_auth() {
    local username="$1"
    local password="$2"
    local code=''

    code="$(curl -fsS -u "$username:$password" -o /dev/null -w '%{http_code}' -H 'Accept: application/vnd.github+json' -H 'X-GitHub-Api-Version: 2022-11-28' https://api.github.com/user || true)"
    [[ "$code" == '200' ]]
}

run_gh_with_token() {
    local token="$1"
    shift
    GH_TOKEN="$token" GITHUB_TOKEN="$token" command gh "$@"
}

healthcheck_gh() {
    local token="$1"
    run_gh_with_token "$token" api user --jq .login >/dev/null 2>&1
}

github_login_from_token() {
    local token="$1"
    run_gh_with_token "$token" api user --jq .login 2>/dev/null || true
}

json_escape() {
    local s="$1"
    s=${s//\\/\\\\}
    s=${s//\"/\\\"}
    s=${s//$'\n'/\\n}
    s=${s//$'\r'/\\r}
    s=${s//$'\t'/\\t}
    printf '%s' "$s"
}

github_api() {
    local token="$1"
    local method="$2"
    local url="$3"
    local data="${4:-}"

    if [[ -n "$data" ]]; then
        curl -fsS -X "$method" -H "Authorization: Bearer $token" -H 'Accept: application/vnd.github+json' -H 'X-GitHub-Api-Version: 2022-11-28' -H 'Content-Type: application/json' --data "$data" "$url"
    else
        curl -fsS -X "$method" -H "Authorization: Bearer $token" -H 'Accept: application/vnd.github+json' -H 'X-GitHub-Api-Version: 2022-11-28' "$url"
    fi
}

parse_next_link_from_headers() {
    local headers_file="$1"
    awk 'BEGIN { IGNORECASE=1 } /^Link:/ { print }' "$headers_file" | \
        sed -n 's/.*<\([^>]*\)>;[[:space:]]*rel="next".*/\1/p' | head -n1
}

list_ssh_keys_json() {
    local token="$1"
    local url='https://api.github.com/user/keys?per_page=100'
    local body_file=''
    local headers_file=''
    local merged='[]'
    local next_url=''

    while [[ -n "$url" ]]; do
        body_file="$(same_dir_tmp "$CONFIG_DIR")"
        headers_file="$(same_dir_tmp "$CONFIG_DIR")"
        curl -fsS -D "$headers_file" \
            -H "Authorization: Bearer $token" \
            -H 'Accept: application/vnd.github+json' \
            -H 'X-GitHub-Api-Version: 2022-11-28' \
            "$url" >"$body_file"
        merged="$(jq -c -s '.[0] + .[1]' <(printf '%s' "$merged") "$body_file")"
        next_url="$(parse_next_link_from_headers "$headers_file")"
        rm -f "$body_file" "$headers_file" >/dev/null 2>&1 || true
        CURRENT_TMP_FILE=''
        url="$next_url"
    done

    printf '%s\n' "$merged"
}

ssh_key_id_from_json_by_key() {
    local json="$1"
    local key="$2"

    jq -r --arg key "$key" '.[] | select(.key == $key) | .id | tostring' <<<"$json" | head -n1
}

ssh_public_key_registered() {
    local token="$1"
    local pubkey_path="$2"
    local pubkey=''
    local json=''

    [[ -f "$pubkey_path" ]] || return 1
    pubkey="$(cat "$pubkey_path")"
    json="$(list_ssh_keys_json "$token" 2>/dev/null || true)"
    [[ -n "$json" ]] || return 1
    ssh_key_id_from_json_by_key "$json" "$pubkey" >/dev/null
}

upload_ssh_key() {
    local token="$1"
    local pubkey_path="$2"
    local title="$3"
    local pubkey=''
    local payload=''
    local response=''
    local id=''

    pubkey="$(cat "$pubkey_path")"
    payload=$(printf '{"title":"%s","key":"%s"}' "$(json_escape "$title")" "$(json_escape "$pubkey")")
    response="$(github_api "$token" POST 'https://api.github.com/user/keys' "$payload")" || return 1
    id="$(jq -r '.id // empty' <<<"$response")"
    [[ -n "$id" ]] || return 1
    printf '%s\n' "$id"
}

delete_ssh_key_by_id() {
    local token="$1"
    local id="$2"
    [[ -n "$id" ]] || return 0
    github_api "$token" DELETE "https://api.github.com/user/keys/$id" >/dev/null
}

clear_keyring_secret() {
    local service="$1"
    local host_arg="${2:-}"

    case "$(os_type)" in
        macos)
            security delete-generic-password -s "$service" -a "$USER" >/dev/null 2>&1 || true
            ;;
        linux)
            if have_cmd secret-tool; then
                if [[ -n "$host_arg" ]]; then
                    secret-tool clear service "$service" user "$USER" host "$host_arg" >/dev/null 2>&1 || true
                else
                    secret-tool clear service "$service" user "$USER" >/dev/null 2>&1 || true
                fi
            fi
            ;;
    esac
}

store_keyring_secret() {
    local service="$1"
    local secret="$2"
    local host_arg="${3:-}"

    case "$(os_type)" in
        macos)
            security add-generic-password -U -s "$service" -a "$USER" -w "$secret" >/dev/null
            ;;
        linux)
            have_cmd secret-tool || return 1
            if [[ -n "$host_arg" ]]; then
                printf '%s' "$secret" | secret-tool store --label "Flox $service" service "$service" user "$USER" host "$host_arg" >/dev/null
            else
                printf '%s' "$secret" | secret-tool store --label "Flox $service" service "$service" user "$USER" >/dev/null
            fi
            ;;
        *)
            return 1
            ;;
    esac
}

retrieve_keyring_secret() {
    local service="$1"
    local host_arg="${2:-}"

    case "$(os_type)" in
        macos)
            security find-generic-password -s "$service" -a "$USER" -w 2>/dev/null || true
            ;;
        linux)
            have_cmd secret-tool || return 1
            if [[ -n "$host_arg" ]]; then
                secret-tool lookup service "$service" user "$USER" host "$host_arg" 2>/dev/null || true
            else
                secret-tool lookup service "$service" user "$USER" 2>/dev/null || true
            fi
            ;;
        *)
            return 1
            ;;
    esac
}

keyring_available() {
    case "$(os_type)" in
        macos) have_cmd security ;;
        linux) have_cmd secret-tool ;;
        *) return 1 ;;
    esac
}

file_fallback_allowed() {
    case "$FILE_FALLBACK_POLICY" in
        always) return 0 ;;
        never) return 1 ;;
        prompt)
            show_file_storage_warning
            ui_confirm 'Use encrypted local-file fallback?' false
            ;;
        *) return 1 ;;
    esac
}

backup_token_file_if_present() {
    if [[ -f "$TOKEN_ENC_FILE" ]]; then
        TOKEN_FILE_BACKUP="$(same_dir_tmp "$CONFIG_DIR")"
        cp -f "$TOKEN_ENC_FILE" "$TOKEN_FILE_BACKUP"
        chmod 600 "$TOKEN_FILE_BACKUP"
        TOKEN_FILE_HAD_OLD=1
        CURRENT_TMP_FILE=''
    else
        TOKEN_FILE_BACKUP=''
        TOKEN_FILE_HAD_OLD=0
    fi
}

restore_token_file_backup() {
    if [[ "$TOKEN_FILE_HAD_OLD" -eq 1 && -n "$TOKEN_FILE_BACKUP" && -f "$TOKEN_FILE_BACKUP" ]]; then
        mv -f "$TOKEN_FILE_BACKUP" "$TOKEN_ENC_FILE"
    else
        rm -f "$TOKEN_ENC_FILE" >/dev/null 2>&1 || true
        [[ -n "$TOKEN_FILE_BACKUP" && -e "$TOKEN_FILE_BACKUP" ]] && rm -f "$TOKEN_FILE_BACKUP" || true
    fi
    TOKEN_FILE_BACKUP=''
    TOKEN_FILE_HAD_OLD=0
}

commit_token_file_backup() {
    [[ -n "$TOKEN_FILE_BACKUP" && -e "$TOKEN_FILE_BACKUP" ]] && rm -f "$TOKEN_FILE_BACKUP" || true
    TOKEN_FILE_BACKUP=''
    TOKEN_FILE_HAD_OLD=0
}

backup_git_file_if_present() {
    if [[ -f "$GIT_CREDENTIALS_ENC_FILE" ]]; then
        GIT_FILE_BACKUP="$(same_dir_tmp "$CONFIG_DIR")"
        cp -f "$GIT_CREDENTIALS_ENC_FILE" "$GIT_FILE_BACKUP"
        chmod 600 "$GIT_FILE_BACKUP"
        GIT_FILE_HAD_OLD=1
        CURRENT_TMP_FILE=''
    else
        GIT_FILE_BACKUP=''
        GIT_FILE_HAD_OLD=0
    fi
}

restore_git_file_backup() {
    if [[ "$GIT_FILE_HAD_OLD" -eq 1 && -n "$GIT_FILE_BACKUP" && -f "$GIT_FILE_BACKUP" ]]; then
        mv -f "$GIT_FILE_BACKUP" "$GIT_CREDENTIALS_ENC_FILE"
    else
        rm -f "$GIT_CREDENTIALS_ENC_FILE" >/dev/null 2>&1 || true
        [[ -n "$GIT_FILE_BACKUP" && -e "$GIT_FILE_BACKUP" ]] && rm -f "$GIT_FILE_BACKUP" || true
    fi
    GIT_FILE_BACKUP=''
    GIT_FILE_HAD_OLD=0
}

commit_git_file_backup() {
    [[ -n "$GIT_FILE_BACKUP" && -e "$GIT_FILE_BACKUP" ]] && rm -f "$GIT_FILE_BACKUP" || true
    GIT_FILE_BACKUP=''
    GIT_FILE_HAD_OLD=0
}

backup_token_keyring_if_present() {
    TOKEN_KEYRING_BACKUP="$(retrieve_keyring_secret "$GITHUB_TOKEN_SERVICE" 2>/dev/null || true)"
    if [[ -n "$TOKEN_KEYRING_BACKUP" ]]; then
        TOKEN_KEYRING_HAD_OLD=1
    else
        TOKEN_KEYRING_HAD_OLD=0
    fi
}

restore_token_keyring_backup() {
    if [[ "$TOKEN_KEYRING_HAD_OLD" -eq 1 ]]; then
        store_keyring_secret "$GITHUB_TOKEN_SERVICE" "$TOKEN_KEYRING_BACKUP" || true
    else
        clear_keyring_secret "$GITHUB_TOKEN_SERVICE" || true
    fi
    TOKEN_KEYRING_BACKUP=''
    TOKEN_KEYRING_HAD_OLD=0
}

commit_token_keyring_backup() {
    TOKEN_KEYRING_BACKUP=''
    TOKEN_KEYRING_HAD_OLD=0
}

backup_git_keyring_if_present() {
    GIT_KEYRING_BACKUP="$(retrieve_keyring_secret "$GITHUB_GIT_SERVICE" github.com 2>/dev/null || true)"
    if [[ -n "$GIT_KEYRING_BACKUP" ]]; then
        GIT_KEYRING_HAD_OLD=1
    else
        GIT_KEYRING_HAD_OLD=0
    fi
}

restore_git_keyring_backup() {
    if [[ "$GIT_KEYRING_HAD_OLD" -eq 1 ]]; then
        store_keyring_secret "$GITHUB_GIT_SERVICE" "$GIT_KEYRING_BACKUP" github.com || true
    else
        clear_keyring_secret "$GITHUB_GIT_SERVICE" github.com || true
    fi
    GIT_KEYRING_BACKUP=''
    GIT_KEYRING_HAD_OLD=0
}

commit_git_keyring_backup() {
    GIT_KEYRING_BACKUP=''
    GIT_KEYRING_HAD_OLD=0
}

purge_token_backend() {
    local storage_method="$1"
    case "$storage_method" in
        keyring) clear_keyring_secret "$GITHUB_TOKEN_SERVICE" ;;
        file) rm -f "$TOKEN_ENC_FILE" >/dev/null 2>&1 || true ;;
    esac
}

purge_git_backend() {
    local storage_method="$1"
    case "$storage_method" in
        keyring) clear_keyring_secret "$GITHUB_GIT_SERVICE" github.com ;;
        file) rm -f "$GIT_CREDENTIALS_ENC_FILE" >/dev/null 2>&1 || true ;;
    esac
}

stage_and_promote_token_keyring() {
    local token="$1"
    local staged=''
    local promoted=''

    STAGED_TOKEN_SERVICE="${GITHUB_TOKEN_SERVICE}-stage-$$"
    store_keyring_secret "$STAGED_TOKEN_SERVICE" "$token"
    staged="$(retrieve_keyring_secret "$STAGED_TOKEN_SERVICE")"
    [[ "$staged" == "$token" ]] || return 1
    validate_github_token "$staged" || return 1

    store_keyring_secret "$GITHUB_TOKEN_SERVICE" "$staged"
    promoted="$(retrieve_keyring_secret "$GITHUB_TOKEN_SERVICE")"
    [[ "$promoted" == "$token" ]] || return 1
    validate_github_token "$promoted" || return 1

    clear_keyring_secret "$STAGED_TOKEN_SERVICE" || true
    STAGED_TOKEN_SERVICE=''
    printf 'keyring\n'
}

stage_and_promote_token_file() {
    local token="$1"
    local stage_file=''
    local staged=''

    stage_file="$(same_dir_tmp "$CONFIG_DIR")"
    CURRENT_TMP_FILE=''
    encrypt_to_file "$token" "$stage_file"
    staged="$(decrypt_from_file "$stage_file" 2>/dev/null || true)"
    [[ "$staged" == "$token" ]] || {
        rm -f "$stage_file" >/dev/null 2>&1 || true
        return 1
    }
    validate_github_token "$staged" || {
        rm -f "$stage_file" >/dev/null 2>&1 || true
        return 1
    }

    mv -f "$stage_file" "$TOKEN_ENC_FILE"
    staged="$(decrypt_from_file "$TOKEN_ENC_FILE" 2>/dev/null || true)"
    [[ "$staged" == "$token" ]]
}

stage_and_promote_git_keyring() {
    local username="$1"
    local password="$2"
    local payload="${username}:${password}"
    local staged=''
    local promoted=''

    STAGED_GIT_SERVICE="${GITHUB_GIT_SERVICE}-stage-$$"
    store_keyring_secret "$STAGED_GIT_SERVICE" "$payload" github.com
    staged="$(retrieve_keyring_secret "$STAGED_GIT_SERVICE" github.com)"
    [[ "$staged" == "$payload" ]] || return 1

    store_keyring_secret "$GITHUB_GIT_SERVICE" "$payload" github.com
    promoted="$(retrieve_keyring_secret "$GITHUB_GIT_SERVICE" github.com)"
    [[ "$promoted" == "$payload" ]] || return 1

    clear_keyring_secret "$STAGED_GIT_SERVICE" github.com || true
    STAGED_GIT_SERVICE=''
    printf 'keyring\n'
}

stage_and_promote_git_file() {
    local username="$1"
    local password="$2"
    local payload="${username}:${password}"
    local stage_file=''
    local staged=''

    stage_file="$(same_dir_tmp "$CONFIG_DIR")"
    CURRENT_TMP_FILE=''
    encrypt_to_file "$payload" "$stage_file"
    staged="$(decrypt_from_file "$stage_file" 2>/dev/null || true)"
    [[ "$staged" == "$payload" ]] || {
        rm -f "$stage_file" >/dev/null 2>&1 || true
        return 1
    }

    mv -f "$stage_file" "$GIT_CREDENTIALS_ENC_FILE"
    staged="$(decrypt_from_file "$GIT_CREDENTIALS_ENC_FILE" 2>/dev/null || true)"
    [[ "$staged" == "$payload" ]]
}

retrieve_github_token() {
    local storage_method=''
    storage_method="$(config_get TOKEN_STORAGE 2>/dev/null || true)"

    case "$storage_method" in
        keyring) retrieve_keyring_secret "$GITHUB_TOKEN_SERVICE" ;;
        file) decrypt_from_file "$TOKEN_ENC_FILE" ;;
        *) return 1 ;;
    esac
}

export_token_into_shell() {
    local token=''
    token="$(retrieve_github_token 2>/dev/null || true)"
    if [[ -n "$token" ]]; then
        export GH_TOKEN="$token"
        export GITHUB_TOKEN="$token"
    fi
}

backup_git_key_to_file() {
    local key="$1"
    local out="$2"
    : >"$out"
    git config --global --get-all "$key" >"$out" 2>/dev/null || true
}

restore_git_key_from_file() {
    local key="$1"
    local in="$2"
    git config --global --unset-all "$key" >/dev/null 2>&1 || true
    while IFS= read -r value; do
        [[ -n "$value" ]] || continue
        git config --global --add "$key" "$value"
    done <"$in"
}

shell_single_quote() {
    printf '%s' "$1" | sed "s/'/'\\''/g"
}

git_helper_shell_snippet() {
    local quoted=''
    quoted="$(shell_single_quote "$GIT_HELPER")"
    printf '!f(){ helper=$1; shift; "$helper" "$@"; }; f '\''%s'\'' "$@"' "$quoted"
}

configure_github_https_helper() {
    git config --global --unset-all credential.https://github.com.helper >/dev/null 2>&1 || true
    git config --global --add credential.https://github.com.helper "$(git_helper_shell_snippet)"
}

current_gh_git_protocol() {
    command gh config get git_protocol --host github.com 2>/dev/null || true
}

set_gh_git_protocol() {
    local mode="$1"
    command gh config set git_protocol "$mode" --host github.com >/dev/null 2>&1 || return 1
    [[ "$(current_gh_git_protocol)" == "$mode" ]]
}

read_helper_resolved_creds() {
    local requested_user="$1"
    printf 'protocol=https\nhost=github.com\nusername=%s\n\n' "$requested_user" | git credential fill 2>/dev/null || true
}

healthcheck_git_https() {
    local username="$1"
    local password="$2"
    local output=''

    output="$(read_helper_resolved_creds "$username")"
    grep -qx "username=$username" <<<"$output" || return 1
    grep -qx "password=$password" <<<"$output" || return 1
    validate_github_basic_auth "$username" "$password"
}

clear_github_https_git_config() {
    git config --global --unset-all credential.https://github.com.helper >/dev/null 2>&1 || true
}

clear_github_https_state() {
    clear_github_https_git_config
    purge_git_backend keyring
    purge_git_backend file
    config_unset GIT_CREDENTIAL_STORAGE
}

apply_requested_git_user_info() {
    local git_user_name=''
    local git_user_email=''

    if [[ -n "$CLI_GIT_USER_NAME" ]]; then
        git config --global user.name "$CLI_GIT_USER_NAME"
        git_user_name="$CLI_GIT_USER_NAME"
    else
        git_user_name="$(git config --global user.name 2>/dev/null || true)"
    fi

    if [[ -n "$CLI_GIT_USER_EMAIL" ]]; then
        git config --global user.email "$CLI_GIT_USER_EMAIL"
        git_user_email="$CLI_GIT_USER_EMAIL"
    else
        git_user_email="$(git config --global user.email 2>/dev/null || true)"
    fi

    if [[ "$NONINTERACTIVE" -eq 1 ]]; then
        return 0
    fi

    if [[ -n "$git_user_name" && -n "$git_user_email" ]]; then
        return 0
    fi

    if ! ui_confirm 'Set Git user.name and user.email now?' true; then
        return 0
    fi

    if [[ -z "$git_user_name" ]]; then
        while true; do
            git_user_name="$(ui_input 'Git full name:' 'Jane Doe' || true)"
            git_user_name="$(printf '%s' "$git_user_name" | trim_whitespace)"
            case "$git_user_name" in
                exit|quit) break ;;
                '') printf 'Name cannot be empty.\n' >&2 ;;
                *) git config --global user.name "$git_user_name"; break ;;
            esac
        done
    fi

    if [[ -z "$git_user_email" ]]; then
        while true; do
            git_user_email="$(ui_input 'Git email:' 'jane@example.com' || true)"
            git_user_email="$(printf '%s' "$git_user_email" | trim_whitespace)"
            case "$git_user_email" in
                exit|quit) break ;;
                '') printf 'Email cannot be empty.\n' >&2 ;;
                *)
                    if [[ "$git_user_email" =~ ^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$ ]]; then
                        git config --global user.email "$git_user_email"
                        break
                    fi
                    printf 'Email format looks invalid.\n' >&2
                    ;;
            esac
        done
    fi
}

current_github_git_mode() {
    local configured=''
    configured="$(config_get GIT_MODE 2>/dev/null || true)"
    if [[ -n "$configured" ]]; then
        printf '%s\n' "$configured"
        return 0
    fi

    configured="$(current_gh_git_protocol)"
    if [[ -n "$configured" ]]; then
        printf '%s\n' "$configured"
    else
        printf 'not configured\n'
    fi
}

ssh_config_quote() {
    local s="$1"
    s=${s//\\/\\\\}
    s=${s//\"/\\\"}
    printf '"%s"' "$s"
}

clear_managed_github_ssh_config() {
    local tmp=''

    [[ -f "$SSH_CONFIG_FILE" ]] || return 0
    tmp="$(same_dir_tmp "$(dirname "$SSH_CONFIG_FILE")")"
    awk -v start="$MANAGED_SSH_BLOCK_START" -v end="$MANAGED_SSH_BLOCK_END" '
        $0 == start { skip=1; next }
        $0 == end { skip=0; next }
        skip != 1 { print }
    ' "$SSH_CONFIG_FILE" >"$tmp"
    chmod 600 "$tmp"
    mv -f "$tmp" "$SSH_CONFIG_FILE"
    CURRENT_TMP_FILE=''
}

install_managed_github_ssh_config() {
    local key_path="$1"
    local quoted_key=''

    mkdir -p "$(dirname "$SSH_CONFIG_FILE")"
    chmod 700 "$(dirname "$SSH_CONFIG_FILE")" 2>/dev/null || true
    clear_managed_github_ssh_config
    quoted_key="$(ssh_config_quote "$key_path")"

    {
        [[ -f "$SSH_CONFIG_FILE" ]] && cat "$SSH_CONFIG_FILE"
        printf '%s\n' "$MANAGED_SSH_BLOCK_START"
        printf 'Host github.com\n'
        printf '  HostName ssh.github.com\n'
        printf '  Port 443\n'
        printf '  User git\n'
        printf '  IdentitiesOnly yes\n'
        printf '  IdentityFile %s\n' "$quoted_key"
        printf '%s\n' "$MANAGED_SSH_BLOCK_END"
    } | atomic_write_file "$SSH_CONFIG_FILE" 600
}

ssh_smoke_test_route() {
    local key_path="$1"
    local host="$2"
    local port="$3"
    local output=''
    local rc=0

    need_cmd ssh
    output="$({
        ssh -T \
            -o BatchMode=yes \
            -o ConnectTimeout=10 \
            -o IdentitiesOnly=yes \
            -o IdentityFile="$key_path" \
            -o PreferredAuthentications=publickey \
            -o StrictHostKeyChecking=accept-new \
            -p "$port" \
            "git@$host"
    } 2>&1)" || rc=$?

    if grep -qi 'successfully authenticated' <<<"$output"; then
        return 0
    fi
    return 1
}

ssh_smoke_test() {
    local key_path="$1"

    SSH_SMOKE_ROUTE=''
    if ssh_smoke_test_route "$key_path" github.com 22; then
        SSH_SMOKE_ROUTE='github.com:22'
        return 0
    fi
    if ssh_smoke_test_route "$key_path" ssh.github.com 443; then
        SSH_SMOKE_ROUTE='ssh.github.com:443'
        return 0
    fi
    return 1
}

generate_ssh_keypair() {
    local key_path="$1"
    local passphrase="$2"
    local comment="$3"

    need_cmd ssh-keygen
    mkdir -p "$(dirname "$key_path")"
    chmod 700 "$(dirname "$key_path")"
    ssh-keygen -t ed25519 -f "$key_path" -N "$passphrase" -C "$comment" >/dev/null
    chmod 600 "$key_path"
    chmod 644 "$key_path.pub"
}

maybe_add_ssh_key_to_agent() {
    local key_path="$1"

    if ! have_cmd ssh-add; then
        return 0
    fi
    if [[ -z "${SSH_AUTH_SOCK:-}" ]]; then
        warn 'SSH agent is not active in this shell; add the key later with ssh-add if needed.'
        return 0
    fi
    ssh-add "$key_path" >/dev/null 2>&1 || warn 'Could not add the SSH key to the running agent.'
}

configure_git_https_transaction() {
    local username="$1"
    local password="$2"
    local helper_backup=''
    local protocol_backup=''
    local old_storage=''
    local old_mode=''
    local storage_method=''
    local keyring_attempted=0

    helper_backup="$(same_dir_tmp "$CONFIG_DIR")"
    backup_git_key_to_file credential.https://github.com.helper "$helper_backup"
    CURRENT_TMP_FILE=''

    protocol_backup="$(current_gh_git_protocol)"
    old_storage="$(config_get GIT_CREDENTIAL_STORAGE 2>/dev/null || true)"
    old_mode="$(config_get GIT_MODE 2>/dev/null || true)"

    if keyring_available; then
        keyring_attempted=1
        backup_git_keyring_if_present
        if storage_method="$(stage_and_promote_git_keyring "$username" "$password" 2>/dev/null || true)"; then
            :
        else
            restore_git_keyring_backup
            storage_method=''
        fi
    fi

    if [[ -z "$storage_method" ]]; then
        if file_fallback_allowed; then
            backup_git_file_if_present
            if stage_and_promote_git_file "$username" "$password"; then
                storage_method='file'
            else
                restore_git_file_backup
                rm -f "$helper_backup" >/dev/null 2>&1 || true
                return 1
            fi
        else
            [[ "$keyring_attempted" -eq 1 ]] && warn 'Keyring storage for Git HTTPS was unavailable or unusable, and file fallback is disabled.'
            rm -f "$helper_backup" >/dev/null 2>&1 || true
            return 1
        fi
    fi

    config_set GIT_CREDENTIAL_STORAGE "$storage_method"
    configure_github_https_helper
    set_gh_git_protocol https || {
        if [[ "$storage_method" == 'keyring' ]]; then
            restore_git_keyring_backup
        else
            restore_git_file_backup
        fi
        if [[ -n "$old_storage" ]]; then
            config_set GIT_CREDENTIAL_STORAGE "$old_storage"
        else
            config_unset GIT_CREDENTIAL_STORAGE
        fi
        restore_git_key_from_file credential.https://github.com.helper "$helper_backup"
        rm -f "$helper_backup" >/dev/null 2>&1 || true
        return 1
    }

    if healthcheck_git_https "$username" "$password"; then
        if [[ "$storage_method" == 'keyring' ]]; then
            purge_git_backend file
            commit_git_keyring_backup
            commit_git_file_backup
        else
            purge_git_backend keyring
            commit_git_file_backup
            commit_git_keyring_backup
        fi
        config_set GIT_CREDENTIAL_STORAGE "$storage_method"
        config_set GIT_MODE https
        config_unset SSH_KEY_PATH
        config_unset SSH_ROUTE
        clear_managed_github_ssh_config
        rm -f "$helper_backup" >/dev/null 2>&1 || true
        printf '%s\n' "$storage_method"
        return 0
    fi

    if [[ "$storage_method" == 'keyring' ]]; then
        restore_git_keyring_backup
    else
        restore_git_file_backup
    fi
    restore_git_key_from_file credential.https://github.com.helper "$helper_backup"
    rm -f "$helper_backup" >/dev/null 2>&1 || true

    if [[ -n "$protocol_backup" ]]; then
        command gh config set git_protocol "$protocol_backup" --host github.com >/dev/null 2>&1 || true
    fi
    if [[ -n "$old_storage" ]]; then
        config_set GIT_CREDENTIAL_STORAGE "$old_storage"
    else
        config_unset GIT_CREDENTIAL_STORAGE
    fi
    if [[ -n "$old_mode" ]]; then
        config_set GIT_MODE "$old_mode"
    else
        config_unset GIT_MODE
    fi
    return 1
}

configure_git_ssh_transaction() {
    local token="$1"
    local ssh_key_path="$2"
    local key_title="$3"
    local helper_backup=''
    local protocol_backup=''
    local old_mode=''
    local old_storage=''
    local old_ssh_key_path=''
    local old_ssh_route=''
    local key_json=''
    local existing_key_id=''
    local ssh_config_backup=''
    local ssh_config_had_old=0

    rollback_git_ssh_transaction() {
        if [[ -n "$SSH_UPLOADED_KEY_ID" ]]; then
            delete_ssh_key_by_id "$token" "$SSH_UPLOADED_KEY_ID" >/dev/null 2>&1 || warn 'Uploaded SSH key could not be deleted after rollback.'
            SSH_UPLOADED_KEY_ID=''
        fi

        if [[ "$ssh_config_had_old" -eq 1 && -n "$ssh_config_backup" && -f "$ssh_config_backup" ]]; then
            mv -f "$ssh_config_backup" "$SSH_CONFIG_FILE"
        else
            [[ -n "$ssh_config_backup" && -e "$ssh_config_backup" ]] && rm -f "$ssh_config_backup" || true
            rm -f "$SSH_CONFIG_FILE" >/dev/null 2>&1 || true
        fi

        restore_git_key_from_file credential.https://github.com.helper "$helper_backup"
        rm -f "$helper_backup" >/dev/null 2>&1 || true

        if [[ -n "$protocol_backup" ]]; then
            command gh config set git_protocol "$protocol_backup" --host github.com >/dev/null 2>&1 || true
        fi
        if [[ -n "$old_mode" ]]; then
            config_set GIT_MODE "$old_mode"
        else
            config_unset GIT_MODE
        fi
        if [[ -n "$old_storage" ]]; then
            config_set GIT_CREDENTIAL_STORAGE "$old_storage"
        else
            config_unset GIT_CREDENTIAL_STORAGE
        fi
        if [[ -n "$old_ssh_key_path" ]]; then
            config_set SSH_KEY_PATH "$old_ssh_key_path"
        else
            config_unset SSH_KEY_PATH
        fi
        if [[ -n "$old_ssh_route" ]]; then
            config_set SSH_ROUTE "$old_ssh_route"
        else
            config_unset SSH_ROUTE
        fi
        return 1
    }

    helper_backup="$(same_dir_tmp "$CONFIG_DIR")"
    backup_git_key_to_file credential.https://github.com.helper "$helper_backup"
    CURRENT_TMP_FILE=''

    protocol_backup="$(current_gh_git_protocol)"
    old_mode="$(config_get GIT_MODE 2>/dev/null || true)"
    old_storage="$(config_get GIT_CREDENTIAL_STORAGE 2>/dev/null || true)"
    old_ssh_key_path="$(config_get SSH_KEY_PATH 2>/dev/null || true)"
    old_ssh_route="$(config_get SSH_ROUTE 2>/dev/null || true)"
    key_json="$(list_ssh_keys_json "$token" 2>/dev/null || true)"
    existing_key_id="$(ssh_key_id_from_json_by_key "$key_json" "$(cat "$ssh_key_path.pub")" 2>/dev/null || true)"

    if [[ -f "$SSH_CONFIG_FILE" ]]; then
        ssh_config_backup="$(same_dir_tmp "$(dirname "$SSH_CONFIG_FILE")")"
        cp -f "$SSH_CONFIG_FILE" "$ssh_config_backup"
        chmod 600 "$ssh_config_backup"
        ssh_config_had_old=1
        CURRENT_TMP_FILE=''
    fi

    if [[ -z "$existing_key_id" ]]; then
        SSH_UPLOADED_KEY_ID="$(upload_ssh_key "$token" "$ssh_key_path.pub" "$key_title" 2>/dev/null || true)"
        [[ -n "$SSH_UPLOADED_KEY_ID" ]] || {
            [[ -n "$ssh_config_backup" && -e "$ssh_config_backup" ]] && rm -f "$ssh_config_backup" || true
            restore_git_key_from_file credential.https://github.com.helper "$helper_backup"
            rm -f "$helper_backup" >/dev/null 2>&1 || true
            return 1
        }
    fi

    set_gh_git_protocol ssh || {
        rollback_git_ssh_transaction
        return 1
    }

    maybe_add_ssh_key_to_agent "$ssh_key_path"
    if ssh_smoke_test "$ssh_key_path"; then
        if [[ "$SSH_SMOKE_ROUTE" == 'ssh.github.com:443' ]]; then
            install_managed_github_ssh_config "$ssh_key_path" || {
                rollback_git_ssh_transaction
                return 1
            }
        else
            clear_managed_github_ssh_config || {
                rollback_git_ssh_transaction
                return 1
            }
        fi

        clear_github_https_state
        commit_git_file_backup
        commit_git_keyring_backup
        config_set GIT_MODE ssh
        config_set SSH_KEY_PATH "$ssh_key_path"
        config_set SSH_ROUTE "$SSH_SMOKE_ROUTE"
        [[ -n "$ssh_config_backup" && -e "$ssh_config_backup" ]] && rm -f "$ssh_config_backup" || true
        rm -f "$helper_backup" >/dev/null 2>&1 || true
        SSH_UPLOADED_KEY_ID=''
        printf 'ssh\n'
        return 0
    fi

    rollback_git_ssh_transaction
    return 1
}

check_existing_token() {
    local stored=''
    local token=''

    if [[ -n "$CLI_TOKEN" ]]; then
        return 1
    fi

    stored="$(config_get GITHUB_TOKEN_STORED 2>/dev/null || true)"
    [[ "$stored" == 'true' ]] || return 1

    token="$(retrieve_github_token 2>/dev/null || true)"
    [[ -n "$token" ]] || return 1

    if validate_github_token "$token" && healthcheck_gh "$token"; then
        write_runtime_assets
        export_token_into_shell
        return 0
    fi

    warn 'Stored GitHub token is invalid or gh cannot use it now.'
    return 1
}

prompt_for_token() {
    local github_token=''

    if [[ -n "$CLI_TOKEN" ]]; then
        printf '%s\n' "$CLI_TOKEN"
        return 0
    fi

    if [[ "$NONINTERACTIVE" -eq 1 ]]; then
        return 1
    fi

    while true; do
        github_token="$(ui_secret 'GitHub token:' || true)"
        case "$github_token" in
            exit|quit) return 1 ;;
            '')
                printf 'Token cannot be empty.\n' >&2
                continue
                ;;
        esac

        printf 'Validating token...\n'
        if validate_github_token "$github_token"; then
            if healthcheck_gh "$github_token"; then
                printf 'Token works with the GitHub API and gh.\n'
                printf '%s\n' "$github_token"
                return 0
            fi
            printf 'Token is valid, but gh could not use it in this env.\n' >&2
        else
            printf 'Token is invalid. Please try again.\n' >&2
        fi
    done
}

store_token_with_fallback() {
    local github_token="$1"
    local storage_method=''
    local token_storage=''
    local old_storage=''
    local keyring_attempted=0

    old_storage="$(config_get TOKEN_STORAGE 2>/dev/null || true)"

    if keyring_available; then
        keyring_attempted=1
        backup_token_keyring_if_present
        if storage_method="$(stage_and_promote_token_keyring "$github_token" 2>/dev/null || true)"; then
            :
        else
            restore_token_keyring_backup
            storage_method=''
        fi
    fi

    if [[ -z "$storage_method" ]]; then
        if file_fallback_allowed; then
            backup_token_file_if_present
            if stage_and_promote_token_file "$github_token"; then
                storage_method='file'
            else
                restore_token_file_backup
                return 1
            fi
        else
            [[ "$keyring_attempted" -eq 1 ]] && warn 'Keyring storage was unavailable or unusable, and file fallback is disabled.'
            return 1
        fi
    fi

    config_set TOKEN_STORAGE "$storage_method"
    config_set GITHUB_TOKEN_STORED true
    write_runtime_assets
    export_token_into_shell

    if healthcheck_gh "$github_token"; then
        if [[ "$storage_method" == 'keyring' ]]; then
            purge_token_backend file
            commit_token_keyring_backup
            commit_token_file_backup
            token_storage='keyring'
        else
            purge_token_backend keyring
            commit_token_file_backup
            commit_token_keyring_backup
            token_storage='encrypted file'
        fi
        printf '%s\n' "$token_storage"
        return 0
    fi

    if [[ "$storage_method" == 'keyring' ]]; then
        restore_token_keyring_backup
    else
        restore_token_file_backup
    fi
    if [[ -n "$old_storage" ]]; then
        config_set TOKEN_STORAGE "$old_storage"
    else
        config_unset TOKEN_STORAGE
    fi
    config_unset GITHUB_TOKEN_STORED
    return 1
}

prompt_for_git_username() {
    local default_username="$1"
    local git_username=''

    if [[ -n "$CLI_GIT_USERNAME" ]]; then
        printf '%s\n' "$CLI_GIT_USERNAME"
        return 0
    fi

    if [[ "$NONINTERACTIVE" -eq 1 ]]; then
        [[ -n "$default_username" ]] || return 1
        printf '%s\n' "$default_username"
        return 0
    fi

    while true; do
        git_username="$(ui_input 'GitHub username:' "$default_username" || true)"
        git_username="$(printf '%s' "$git_username" | trim_whitespace)"
        [[ -n "$git_username" ]] || git_username="$default_username"
        case "$git_username" in
            exit|quit) return 1 ;;
            '') printf 'Username cannot be empty.\n' >&2 ;;
            *) printf '%s\n' "$git_username"; return 0 ;;
        esac
    done
}

prompt_for_git_password() {
    local git_password=''

    if [[ -n "$CLI_GIT_PASSWORD" ]]; then
        printf '%s\n' "$CLI_GIT_PASSWORD"
        return 0
    fi

    if [[ "$NONINTERACTIVE" -eq 1 ]]; then
        return 1
    fi

    while true; do
        git_password="$(ui_secret 'GitHub token for Git HTTPS:' || true)"
        case "$git_password" in
            exit|quit) return 1 ;;
            '') printf 'Token cannot be empty.\n' >&2 ;;
            *) printf '%s\n' "$git_password"; return 0 ;;
        esac
    done
}

prompt_for_git_mode() {
    local choice=''

    if [[ -n "$CLI_GIT_MODE" ]]; then
        printf '%s\n' "$CLI_GIT_MODE"
        return 0
    fi

    choice="$(ui_select 'Choose Git mode for github.com' 'HTTPS' 'SSH' 'Skip Git setup' || true)"
    case "$choice" in
        HTTPS) printf 'https\n' ;;
        SSH) printf 'ssh\n' ;;
        'Skip Git setup'|'') printf 'skip\n' ;;
        *) printf 'skip\n' ;;
    esac
}

prompt_for_ssh_key_path() {
    local path=''

    if [[ -n "$CLI_SSH_KEY_PATH" ]]; then
        printf '%s\n' "$CLI_SSH_KEY_PATH"
        return 0
    fi

    if [[ "$NONINTERACTIVE" -eq 1 ]]; then
        printf '%s\n' "$DEFAULT_SSH_KEY_PATH"
        return 0
    fi

    path="$(ui_input 'SSH private key path:' "$DEFAULT_SSH_KEY_PATH" || true)"
    path="$(printf '%s' "$path" | trim_whitespace)"
    [[ -n "$path" ]] || path="$DEFAULT_SSH_KEY_PATH"
    case "$path" in
        exit|quit) return 1 ;;
    esac
    printf '%s\n' "$path"
}

prepare_ssh_keypair() {
    local key_path="$1"
    local passphrase=''
    local comment=''

    if [[ -f "$key_path" && -f "$key_path.pub" ]]; then
        printf '%s\n' "$key_path"
        return 0
    fi

    if [[ -f "$key_path" && ! -f "$key_path.pub" ]]; then
        need_cmd ssh-keygen
        ssh-keygen -y -f "$key_path" >"$key_path.pub"
        chmod 644 "$key_path.pub"
        printf '%s\n' "$key_path"
        return 0
    fi

    if [[ -e "$key_path" || -e "$key_path.pub" ]]; then
        printf 'Key path is partially occupied. Clean it up or choose a different path.\n' >&2
        return 1
    fi

    if [[ "$NONINTERACTIVE" -eq 1 ]]; then
        passphrase=''
    else
        if ! ui_confirm 'Generate a new SSH key pair at that path?' true; then
            return 1
        fi
        passphrase="$(ui_secret 'SSH key passphrase (blank allowed):' || true)"
        case "$passphrase" in
            exit|quit) return 1 ;;
        esac
    fi

    comment="${USER}@$(host_shortname)-flox-github"
    generate_ssh_keypair "$key_path" "$passphrase" "$comment"
    printf '%s\n' "$key_path"
}

rewrite_github_remote_url() {
    local mode="$1"
    local url="$2"
    local path=''

    case "$mode" in
        ssh)
            if [[ "$url" =~ ^https://github\.com/(.+)$ ]]; then
                path="${BASH_REMATCH[1]}"
                printf 'git@github.com:%s\n' "$path"
                return 0
            fi
            if [[ "$url" =~ ^ssh://git@github\.com/(.+)$ ]]; then
                path="${BASH_REMATCH[1]}"
                printf 'git@github.com:%s\n' "$path"
                return 0
            fi
            ;;
        https)
            if [[ "$url" =~ ^git@github\.com:(.+)$ ]]; then
                path="${BASH_REMATCH[1]}"
                printf 'https://github.com/%s\n' "$path"
                return 0
            fi
            if [[ "$url" =~ ^ssh://git@github\.com/(.+)$ ]]; then
                path="${BASH_REMATCH[1]}"
                printf 'https://github.com/%s\n' "$path"
                return 0
            fi
            ;;
    esac
    printf '%s\n' "$url"
}

rewrite_repo_remotes() {
    local repo="$1"
    local mode="$2"
    local remote=''
    local fetch_url=''
    local push_url=''
    local new_fetch=''
    local new_push=''

    git -C "$repo" rev-parse --is-inside-work-tree >/dev/null 2>&1 || {
        warn "Skipping non-repo path: $repo"
        return 0
    }

    while IFS= read -r remote; do
        [[ -n "$remote" ]] || continue
        fetch_url="$(git -C "$repo" remote get-url "$remote" 2>/dev/null || true)"
        push_url="$(git -C "$repo" remote get-url --push "$remote" 2>/dev/null || true)"
        new_fetch="$(rewrite_github_remote_url "$mode" "$fetch_url")"
        new_push="$(rewrite_github_remote_url "$mode" "$push_url")"
        if [[ "$new_fetch" != "$fetch_url" ]]; then
            git -C "$repo" remote set-url "$remote" "$new_fetch"
        fi
        if [[ -n "$push_url" && "$new_push" != "$push_url" ]]; then
            git -C "$repo" remote set-url --push "$remote" "$new_push"
        fi
    done < <(git -C "$repo" remote)
}

maybe_rewrite_remotes() {
    local mode="$1"
    local repo=''

    [[ "$REWRITE_REMOTES" -eq 1 ]] || return 0
    if [[ ${#REPO_PATHS[@]} -eq 0 ]]; then
        REPO_PATHS+=("$PWD")
    fi
    for repo in "${REPO_PATHS[@]}"; do
        rewrite_repo_remotes "$repo" "$mode"
    done
}

setup_github_integration() {
    local github_token=''
    local token_storage='not configured'
    local git_mode='not configured'
    local git_storage='not configured'
    local selected_mode=''
    local git_username=''
    local git_password=''
    local ssh_key_path=''
    local ssh_title=''
    local suggested_username=''

    init_runtime
    require_basic_tools
    write_runtime_assets

    if [[ "$NONINTERACTIVE" -eq 0 ]]; then
        ui_clear
        show_welcome_message
        if ! ui_confirm 'Continue with setup?' true; then
            printf 'Setup cancelled.\n'
            return 1
        fi
    fi

    if check_existing_token; then
        github_token="$(retrieve_github_token 2>/dev/null || true)"
        token_storage="$(config_get TOKEN_STORAGE 2>/dev/null || printf 'not configured')"
        case "$token_storage" in
            keyring) token_storage='keyring' ;;
            file) token_storage='encrypted file' ;;
        esac
        [[ "$NONINTERACTIVE" -eq 1 ]] || printf 'A valid stored GitHub token was found.\n\n'
    else
        [[ "$NONINTERACTIVE" -eq 1 ]] || show_token_instructions
        github_token="$(prompt_for_token || true)"
        [[ -n "$github_token" ]] || die 'GitHub token is required'
        token_storage="$(store_token_with_fallback "$github_token" || true)"
        [[ -n "$token_storage" ]] || die 'Token storage setup failed'
    fi

    apply_requested_git_user_info

    selected_mode="$(prompt_for_git_mode)"
    case "$selected_mode" in
        https)
            git_mode='https'
            [[ "$NONINTERACTIVE" -eq 1 ]] || show_token_instructions
            suggested_username="$(github_login_from_token "$github_token" || true)"
            git_username="$(prompt_for_git_username "$suggested_username" || true)"
            [[ -n "$git_username" ]] || die 'GitHub username is required for HTTPS mode'

            if [[ -n "$CLI_GIT_PASSWORD" ]]; then
                git_password="$CLI_GIT_PASSWORD"
            elif [[ -n "$CLI_TOKEN" || "$NONINTERACTIVE" -eq 1 ]]; then
                git_password="$github_token"
            elif ui_confirm 'Reuse the GitHub CLI token for Git HTTPS?' true; then
                git_password="$github_token"
            else
                git_password="$(prompt_for_git_password || true)"
            fi
            [[ -n "$git_password" ]] || die 'Git token is required for HTTPS mode'

            if git_storage="$(configure_git_https_transaction "$git_username" "$git_password")"; then
                case "$git_storage" in
                    keyring) git_storage='keyring' ;;
                    file) git_storage='encrypted file' ;;
                esac
            else
                die 'Git HTTPS setup failed its GitHub auth check. Existing Git config was kept.'
            fi
            maybe_rewrite_remotes https
            ;;
        ssh)
            git_mode='ssh'
            ssh_key_path="$(prompt_for_ssh_key_path || true)"
            [[ -n "$ssh_key_path" ]] || die 'SSH key path is required for SSH mode'
            ssh_key_path="$(prepare_ssh_keypair "$ssh_key_path" || true)"
            [[ -n "$ssh_key_path" ]] || die 'SSH key setup failed'
            ssh_title="$CLI_SSH_KEY_TITLE"
            [[ -n "$ssh_title" ]] || ssh_title="flox-$(host_shortname)-$(basename "$ssh_key_path")"

            if configure_git_ssh_transaction "$github_token" "$ssh_key_path" "$ssh_title" >/dev/null; then
                git_storage='ssh key'
            else
                die 'SSH setup failed. The script rolled back local state and tried to delete any SSH key uploaded in this run.'
            fi
            maybe_rewrite_remotes ssh
            ;;
        skip|'')
            git_mode="$(current_github_git_mode)"
            git_storage="$(config_get GIT_CREDENTIAL_STORAGE 2>/dev/null || printf 'not configured')"
            case "$git_storage" in
                keyring) git_storage='keyring' ;;
                file) git_storage='encrypted file' ;;
                '') git_storage='not configured' ;;
            esac
            ;;
        *)
            die "Unsupported git mode: $selected_mode"
            ;;
    esac

    show_completion_message "$token_storage" "$git_mode" "$git_storage"
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    parse_args "$@"
    setup_github_integration "$@"
fi
