# GPG release signing

This directory will hold `orpheus-release-pubkey.asc` — the ASCII-armored
public key users import to verify release artifacts.

Until a real release-signing key is generated (see [docs/RELEASING.md](../../docs/RELEASING.md)),
this directory is a placeholder. Once the key exists:

```bash
gpg --armor --export <MASTER_KEY_ID> > packaging/gpg/orpheus-release-pubkey.asc
git add packaging/gpg/orpheus-release-pubkey.asc
```

and document the fingerprint in the top-level README.

## User verification flow

```bash
# Import the release public key (one-time)
curl -L https://github.com/dougnukem/orpheus/raw/main/packaging/gpg/orpheus-release-pubkey.asc \
  | gpg --import

# Download a release + its SHA256SUMS manifest + signature
gh release download v0.1.0 --pattern '*' --dir orpheus-v0.1.0
cd orpheus-v0.1.0

# Verify the manifest was signed by the release key
gpg --verify SHA256SUMS.asc SHA256SUMS

# Verify every downloaded artifact matches the manifest
sha256sum -c SHA256SUMS
```
