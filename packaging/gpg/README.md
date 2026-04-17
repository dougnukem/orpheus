# GPG release signing

Public key used to sign Orpheus release artifacts:
[`orpheus-release-pubkey.asc`](./orpheus-release-pubkey.asc).

## Key identity

| Field | Value |
|---|---|
| UID | `Orpheus Release Signing <dougnukem+orpheus-releases@users.noreply.github.com>` |
| Algorithm | RSA 4096 |
| Master fingerprint | `1A5F 37B8 6C66 7F73 D54E  D753 82A4 3907 317F D789` |
| Signing subkey | `084A724EDCD8882C` |
| Created | 2026-04-17 |
| Expires | 2028-04-16 (rotate 1 month before) |

Verify the fingerprint you imported matches the one above before trusting
signatures. The master key is kept offline; the signing subkey lives in
GitHub Actions secrets for CI + on the release maintainer's workstation.

## User verification flow

```bash
# 1. Import the release public key (one-time)
curl -L https://github.com/dougnukem/orpheus/raw/main/packaging/gpg/orpheus-release-pubkey.asc \
  | gpg --import

# 2. Confirm the fingerprint
gpg --fingerprint dougnukem+orpheus-releases@users.noreply.github.com
# expect: 1A5F 37B8 6C66 7F73 D54E  D753 82A4 3907 317F D789

# 3. Download a release + its SHA256SUMS manifest + signature
gh release download v0.1.0 --pattern '*' --dir orpheus-v0.1.0
cd orpheus-v0.1.0

# 4. Verify the manifest was signed by the release key
gpg --verify SHA256SUMS.asc SHA256SUMS

# 5. Verify every downloaded artifact matches the manifest
sha256sum -c SHA256SUMS
```

## Rotation

Subkey rotation (recommended yearly): see [docs/RELEASING.md](../../docs/RELEASING.md#linux-self-signing-gpg).
Master-key rotation is rare — only when expired or compromised.
