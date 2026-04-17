#!/usr/bin/env bash
#
# One-time setup for the Orpheus GPG release signing key.
#
# What this does:
#   1. Generates a dedicated 4096-bit RSA key (master + signing subkey)
#      on the local gpg keyring, separate from any personal keys.
#   2. Exports the public key to packaging/gpg/orpheus-release-pubkey.asc
#      so it can be committed and distributed to users for verification.
#   3. Exports ONLY the signing subkey private material, base64-encoded,
#      and sets three GitHub Actions secrets on dougnukem/orpheus:
#        - GPG_PRIVATE_KEY  (base64 of the subkey)
#        - GPG_PASSPHRASE   (prompted, never echoed)
#        - GPG_KEY_ID       (long ID of the signing subkey)
#   4. Writes the same three values into ./.secrets (gitignored) for
#      local `act` runs.
#
# What this does NOT do:
#   - It does not export the MASTER key. Keep that offline (paper backup
#     or air-gapped USB). Losing the CI subkey is recoverable; losing the
#     master is not.
#   - It does not publish the public key to a keyserver. Do that by hand
#     when you're ready:
#       gpg --keyserver keys.openpgp.org --send-keys <MASTER_FP>
#
# Idempotency: if a key with the same UID email already exists, refuses
# to proceed unless --force is passed (which revokes + deletes the old
# key first — destructive!).
#
# Safety:
#   - set -euo pipefail, umask 077
#   - passphrase read silently, never echoed, never logged
#   - private subkey never written to disk; piped straight into gh via stdin
#   - temp files go under $(mktemp) and are cleaned on exit
#
# Usage:
#   ./scripts/setup-gpg-release-key.sh                 # defaults
#   ./scripts/setup-gpg-release-key.sh --help
#   ./scripts/setup-gpg-release-key.sh --force         # replace existing key
#   ./scripts/setup-gpg-release-key.sh --no-gh         # skip GitHub secrets
#   ./scripts/setup-gpg-release-key.sh --no-local      # skip .secrets update

set -euo pipefail
umask 077

# --- defaults ------------------------------------------------------------
REPO="${ORPHEUS_REPO:-dougnukem/orpheus}"
UID_NAME="${ORPHEUS_KEY_NAME:-Orpheus Release Signing}"
UID_EMAIL="${ORPHEUS_KEY_EMAIL:-dougnukem+orpheus-releases@users.noreply.github.com}"
EXPIRY="${ORPHEUS_KEY_EXPIRY:-2y}"
PUBKEY_PATH="${ORPHEUS_PUBKEY_PATH:-packaging/gpg/orpheus-release-pubkey.asc}"
SECRETS_FILE="${ORPHEUS_SECRETS_FILE:-.secrets}"
DO_GH=1
DO_LOCAL=1
FORCE=0

# --- flag parsing --------------------------------------------------------
usage() {
  sed -n '2,35p' "$0"
  exit 0
}
while [[ $# -gt 0 ]]; do
  case "$1" in
    --help|-h)   usage ;;
    --force)     FORCE=1; shift ;;
    --no-gh)     DO_GH=0; shift ;;
    --no-local)  DO_LOCAL=0; shift ;;
    --name)      UID_NAME="$2"; shift 2 ;;
    --email)     UID_EMAIL="$2"; shift 2 ;;
    --expiry)    EXPIRY="$2"; shift 2 ;;
    --repo)      REPO="$2"; shift 2 ;;
    *) echo "unknown flag: $1" >&2; exit 2 ;;
  esac
done

UID_FULL="$UID_NAME <$UID_EMAIL>"

# --- preflight -----------------------------------------------------------
echo "[setup-gpg] starting preflight..."
command -v gpg >/dev/null || { echo "gpg not found on PATH" >&2; exit 1; }
echo "[setup-gpg]   gpg:  $(command -v gpg)"
if [[ $DO_GH -eq 1 ]]; then
  command -v gh >/dev/null || { echo "gh (github cli) not found on PATH" >&2; exit 1; }
  echo "[setup-gpg]   gh:   $(command -v gh)"
  if ! gh auth status 2>&1 | tail -5; then
    echo "gh is not authenticated; run 'gh auth login'" >&2
    exit 1
  fi
fi

# Reject running from outside the repo root (pubkey + .secrets paths are relative).
echo "[setup-gpg]   cwd:  $(pwd)"
[[ -f Cargo.toml && -f README.md ]] || {
  echo "run this from the orpheus repo root (missing Cargo.toml or README.md)" >&2
  exit 1
}
echo "[setup-gpg] preflight ok"

# --- existing-key guard --------------------------------------------------
echo "[setup-gpg] checking for existing key at $UID_EMAIL..."
existing_fp=$(gpg --list-secret-keys --with-colons "$UID_EMAIL" 2>/dev/null \
  | awk -F: '$1 == "fpr" { print $10; exit }')

if [[ -n "$existing_fp" ]]; then
  if [[ $FORCE -eq 0 ]]; then
    cat >&2 <<EOF
A GPG key with UID email "$UID_EMAIL" already exists:

  fingerprint: $existing_fp

Refusing to regenerate. Either:
  * Use the existing key — extract its subkey id and set secrets manually.
  * Delete it first:  gpg --delete-secret-keys $existing_fp \\
                      && gpg --delete-keys        $existing_fp
  * Or re-run this script with --force to have it deleted for you.

Aborting.
EOF
    exit 1
  fi
  echo "--force: deleting existing key $existing_fp" >&2
  gpg --batch --yes --delete-secret-keys "$existing_fp"
  gpg --batch --yes --delete-keys "$existing_fp"
fi

echo "[setup-gpg] no conflicting key — ready to prompt for passphrase."
echo

# --- passphrase prompt ---------------------------------------------------
prompt_passphrase() {
  local p1 p2
  while true; do
    read -rs -p "New GPG passphrase: " p1; echo >&2
    read -rs -p "Confirm passphrase:  " p2; echo >&2
    if [[ "$p1" != "$p2" ]]; then
      echo "passphrases don't match; try again" >&2
      continue
    fi
    if [[ ${#p1} -lt 12 ]]; then
      echo "passphrase must be >= 12 chars; try again" >&2
      continue
    fi
    PASSPHRASE="$p1"
    break
  done
}
prompt_passphrase

# --- tempdir + cleanup ---------------------------------------------------
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"; unset PASSPHRASE PRIV_B64' EXIT INT TERM

# --- batch key generation -------------------------------------------------
cat > "$TMPDIR/params" <<EOF
%echo Generating Orpheus release key...
Key-Type: RSA
Key-Length: 4096
Key-Usage: sign,cert
Subkey-Type: RSA
Subkey-Length: 4096
Subkey-Usage: sign
Name-Real: $UID_NAME
Name-Email: $UID_EMAIL
Expire-Date: $EXPIRY
Passphrase: $PASSPHRASE
%commit
%echo Done.
EOF

gpg --batch --pinentry-mode loopback --gen-key "$TMPDIR/params"

# --- extract key IDs ------------------------------------------------------
MASTER_FP=$(gpg --list-secret-keys --with-colons "$UID_EMAIL" \
  | awk -F: '$1 == "fpr" { print $10; exit }')
SUBKEY_ID=$(gpg --list-secret-keys --with-colons --keyid-format=LONG "$UID_EMAIL" \
  | awk -F: '$1 == "ssb" && $12 ~ /s/ { print $5; exit }')

[[ -n "$MASTER_FP" && -n "$SUBKEY_ID" ]] || {
  echo "failed to extract fingerprints after generation" >&2; exit 1; }

echo
echo "key generated:"
echo "  master fingerprint : $MASTER_FP"
echo "  signing subkey     : $SUBKEY_ID"
echo

# --- export public key ---------------------------------------------------
mkdir -p "$(dirname "$PUBKEY_PATH")"
gpg --armor --export "$MASTER_FP" > "$PUBKEY_PATH"
echo "wrote public key -> $PUBKEY_PATH"

# --- export signing subkey (private) -> base64 ---------------------------
# The '!' suffix tells gpg to export ONLY this subkey (not the master).
PRIV_B64=$(gpg --pinentry-mode loopback --passphrase "$PASSPHRASE" \
  --armor --export-secret-subkeys "${SUBKEY_ID}!" | base64 | tr -d '\n')

[[ -n "$PRIV_B64" ]] || { echo "subkey export produced empty output" >&2; exit 1; }

# --- set github secrets --------------------------------------------------
if [[ $DO_GH -eq 1 ]]; then
  echo "setting GitHub secrets on $REPO..."
  printf '%s' "$PRIV_B64"   | gh secret set GPG_PRIVATE_KEY --repo "$REPO"
  printf '%s' "$PASSPHRASE" | gh secret set GPG_PASSPHRASE  --repo "$REPO"
  printf '%s' "$SUBKEY_ID"  | gh secret set GPG_KEY_ID      --repo "$REPO"
  echo "  GPG_PRIVATE_KEY  set"
  echo "  GPG_PASSPHRASE   set"
  echo "  GPG_KEY_ID       set"
fi

# --- update local .secrets ----------------------------------------------
if [[ $DO_LOCAL -eq 1 ]]; then
  [[ -f "$SECRETS_FILE" ]] || {
    echo "creating $SECRETS_FILE from .secrets.example" >&2
    cp .secrets.example "$SECRETS_FILE"
  }
  tmp=$(mktemp)
  grep -v '^GPG_\(PRIVATE_KEY\|PASSPHRASE\|KEY_ID\)=' "$SECRETS_FILE" > "$tmp" || true
  {
    cat "$tmp"
    printf 'GPG_PRIVATE_KEY=%s\n' "$PRIV_B64"
    printf 'GPG_PASSPHRASE=%s\n'  "$PASSPHRASE"
    printf 'GPG_KEY_ID=%s\n'       "$SUBKEY_ID"
  } > "$SECRETS_FILE"
  rm -f "$tmp"
  chmod 600 "$SECRETS_FILE"
  echo "updated $SECRETS_FILE (chmod 600)"
fi

# --- summary -------------------------------------------------------------
cat <<EOF

done.

next steps:
  1. commit packaging/gpg/orpheus-release-pubkey.asc and the updated README
     so users can verify releases.
  2. back up the MASTER key offline (paper + air-gapped):
       gpg --armor --export-secret-keys $MASTER_FP > ~/orpheus-master-key.asc
       # move to offline storage, then:
       shred -u ~/orpheus-master-key.asc  # or rm on macOS
  3. optionally publish the public key to a keyserver:
       gpg --keyserver keys.openpgp.org --send-keys $MASTER_FP
  4. verify the github secrets were set:
       gh secret list --repo $REPO | grep GPG_
  5. smoke-test signing locally:
       act push -W .github/workflows/latest.yml    # (needs real build too)

EOF
