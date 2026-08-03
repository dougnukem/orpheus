//! Machine-wide candidate discovery.
//!
//! [`crate::scanner::scan_path`] answers "what wallets are in this directory",
//! and it decides by asking each extractor's `can_handle`, which gates on the
//! file *extension* before it looks at any bytes. That is the right trade for
//! a directory you chose on purpose.
//!
//! It is the wrong trade for a whole machine. A real `$HOME` holds Bitcoin
//! Core wallet backups called `bitcoin_1776391129.legacy.bak`,
//! `wallet.dat.backup`, and `wallet.dat-journal`; none of those reach the
//! Bitcoin Core extractor today. Discovery fixes that by sniffing magic bytes
//! once, up front, and recording *what a file actually is* so the caller can
//! dispatch on format instead of on filename.
//!
//! Nothing here opens a socket, and nothing here writes to a scanned path.

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use bip39::{Language, Mnemonic};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::models::SourceType;

/// Bytes read from the head of every file for magic sniffing.
const HEAD_BYTES: usize = 4096;

/// Files larger than this are never opened for content inspection. Bitcoin
/// Core wallets run to a few hundred MB in pathological cases, but a wallet
/// that size is not what we are hunting for in a Dropbox sweep.
pub const DEFAULT_MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;

/// Text files larger than this are not scanned for WIF / seed material.
pub const DEFAULT_MAX_TEXT_BYTES: u64 = 8 * 1024 * 1024;

/// Word counts BIP39 permits.
const BIP39_WORD_COUNTS: &[usize] = &[12, 15, 18, 21, 24];

/// Directory names pruned during the walk. These are either build noise, caches
/// that regenerate, or vendored trees that cannot contain a personal wallet.
///
/// `.Trash` is deliberately absent — a deleted wallet is still a wallet.
pub const PRUNE_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    ".venv",
    "venv",
    "__pycache__",
    ".npm",
    ".pnpm-store",
    "DerivedData",
    "Caches",
    "Cache",
    ".cache",
    "OrbStack",
    ".gradle",
    ".m2",
    ".terraform",
    "Code Cache",
    "GPUCache",
    "Service Worker",
    "IndexedDB",
    ".Spotlight-V100",
    ".fseventsd",
    ".DocumentRevisions-V100",
    "CoreSimulator",
    "iOS DeviceSupport",
    ".rustup",
    ".vscode-server",
];

/// Extensions worth reading as text when hunting for key material.
const TEXT_EXTS: &[&str] = &[
    "txt", "md", "json", "csv", "log", "rtf", "text", "dump", "bak", "html", "xml", "yaml", "yml",
    "py", "sh", "eml", "mbox", "note", "enex", "asc", "conf", "ini", "wallet", "info", "key",
];

/// Extensions treated as archives or opaque encrypted containers. Flagged for
/// the operator, never opened.
const ARCHIVE_EXTS: &[&str] = &[
    "zip",
    "gz",
    "tgz",
    "bz2",
    "xz",
    "7z",
    "rar",
    "dmg",
    "sparsebundle",
    "sparseimage",
    "gpg",
    "pgp",
    "enc",
    "aes",
    "kdbx",
];

/// What a file turned out to be, decided from magic bytes and content rather
/// than from its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectedFormat {
    /// Berkeley DB — Bitcoin Core legacy wallet (`wallet.dat`, `*.legacy.bak`).
    BitcoinCoreBdb,
    /// SQLite — Bitcoin Core descriptor wallet.
    BitcoinCoreSqlite,
    /// Bitcoin Core wallet (either backing store) whose keys are encrypted
    /// under a passphrase — `mkey`/`ckey` records are present.
    ///
    /// Distinguished from the plain variants because an encrypted wallet that
    /// yields no keys is *locked*, not *empty*, and reporting those the same
    /// way is how someone concludes they have nothing when they have a wallet
    /// they simply cannot open yet.
    BitcoinCoreEncrypted,
    /// MultiBit Classic protobuf, keys in the clear.
    MultibitProtobuf,
    /// MultiBit Classic protobuf with scrypt+AES encrypted keys.
    MultibitEncrypted,
    /// blockchain.com `wallet.aes.json` backup.
    BlockchainComAesJson,
    /// Bitcoin Core `dumpwallet` text, or the `BITCOIN_CORE_WALLET_DUMP` export
    /// produced by the 2026-04 recovery session.
    WalletDumpText,
    /// `listdescriptors` JSON.
    ListDescriptorsJson,
    /// Plain text holding a valid BIP39 mnemonic.
    Bip39Text,
    /// Plain text holding one or more WIF private keys.
    WifText,
    /// Plain text holding a BIP32 extended private key.
    XprvText,
    /// Archive or opaque encrypted container — flagged, not opened.
    Archive,
    /// Matched a name pattern or lives in a crypto-named directory, but the
    /// bytes did not identify it.
    Unknown,
}

impl DetectedFormat {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BitcoinCoreBdb => "bitcoin_core_bdb",
            Self::BitcoinCoreSqlite => "bitcoin_core_sqlite",
            Self::BitcoinCoreEncrypted => "bitcoin_core_encrypted",
            Self::MultibitProtobuf => "multibit_protobuf",
            Self::MultibitEncrypted => "multibit_encrypted",
            Self::BlockchainComAesJson => "blockchain_com_aes_json",
            Self::WalletDumpText => "wallet_dump_text",
            Self::ListDescriptorsJson => "list_descriptors_json",
            Self::Bip39Text => "bip39_text",
            Self::WifText => "wif_text",
            Self::XprvText => "xprv_text",
            Self::Archive => "archive",
            Self::Unknown => "unknown",
        }
    }

    /// Which extractor family should handle this format.
    #[must_use]
    pub const fn source_type(self) -> SourceType {
        match self {
            Self::BitcoinCoreBdb | Self::BitcoinCoreSqlite => SourceType::BitcoinCore,
            Self::MultibitProtobuf => SourceType::Multibit,
            Self::BitcoinCoreEncrypted | Self::MultibitEncrypted | Self::BlockchainComAesJson => {
                SourceType::Encrypted
            }
            Self::WalletDumpText | Self::ListDescriptorsJson | Self::WifText | Self::XprvText => {
                SourceType::WalletDump
            }
            Self::Bip39Text => SourceType::Bip39,
            Self::Archive | Self::Unknown => SourceType::Unknown,
        }
    }

    /// Whether extraction needs a password list to stand any chance.
    #[must_use]
    pub const fn needs_password(self) -> bool {
        matches!(
            self,
            Self::BitcoinCoreEncrypted | Self::MultibitEncrypted | Self::BlockchainComAesJson
        )
    }
}

/// How much attention a candidate deserves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// Known wallet container with an identified format. Extract it.
    A,
    /// Text carrying key or seed material. Extract it.
    B,
    /// Archive or encrypted container. Report it; the operator opens it.
    C,
    /// Contextual — sits in a crypto-named directory but is not itself
    /// identifiable. Report it for human review.
    D,
}

impl Tier {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
        }
    }

    /// Tiers A and B are fed to extractors.
    #[must_use]
    pub const fn is_extractable(self) -> bool {
        matches!(self, Self::A | Self::B)
    }
}

/// One file worth looking at, with the evidence that promoted it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub path: PathBuf,
    pub size: u64,
    /// Unix epoch seconds of last modification, when the OS reports one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_epoch: Option<u64>,
    /// Hex SHA-256 of the file contents. Identical wallets dedupe on this.
    pub sha256: String,
    pub tier: Tier,
    pub format: DetectedFormat,
    /// Human-readable reasons this file was promoted.
    pub signals: Vec<String>,
}

/// Knobs for [`discover`].
#[derive(Debug, Clone)]
pub struct DiscoverOptions {
    pub roots: Vec<PathBuf>,
    pub max_file_bytes: u64,
    pub max_text_bytes: u64,
    pub follow_links: bool,
    /// Extra directory names to prune on top of [`PRUNE_DIRS`].
    pub extra_prunes: Vec<String>,
}

impl Default for DiscoverOptions {
    fn default() -> Self {
        Self {
            roots: vec![],
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
            max_text_bytes: DEFAULT_MAX_TEXT_BYTES,
            follow_links: false,
            extra_prunes: vec![],
        }
    }
}

// ---------------------------------------------------------------------------
// Magic-byte detection
// ---------------------------------------------------------------------------

/// Berkeley DB B-tree magic. Bitcoin Core legacy wallets are BDB btrees, and
/// the magic sits at offset 12 of the metadata page in the host's byte order —
/// so both orders are valid and we accept either.
const BDB_BTREE_MAGIC: u32 = 0x0005_3162;
const BDB_MAGIC_OFFSET: usize = 12;

const SQLITE_MAGIC: &[u8] = b"SQLite format 3\0";

/// Bitcoin Core serialises wallet record keys as length-prefixed strings, so an
/// encrypted master key is stored under the 5 bytes `\x04mkey` and each
/// encrypted private key under `\x04ckey`. Presence of either means the wallet
/// is passphrase-protected.
const MKEY_MARKER: &[u8] = b"\x04mkey";
const CKEY_MARKER: &[u8] = b"\x04ckey";

/// Cap on how much of a wallet we read looking for encryption markers. Records
/// are spread through the file, so a head-only peek is not enough, but neither
/// is reading a 300 MB blockchain-adjacent file.
const ENCRYPTION_PROBE_BYTES: u64 = 64 * 1024 * 1024;
const MULTIBIT_TAG: &[u8] = b"org.bitcoin";
/// MultiBit Classic writes this literal for wallets whose keys are in the clear.
const MULTIBIT_UNENCRYPTED_TAG: &[u8] = b"org.bitcoin.production";

/// Identify a file from its name and the first [`HEAD_BYTES`] of its contents.
///
/// Pure: takes bytes, returns a verdict, touches no filesystem. That is what
/// makes the format table testable against byte literals.
#[must_use]
pub fn detect_format(file_name: &str, head: &[u8]) -> DetectedFormat {
    let lower = file_name.to_ascii_lowercase();

    if head.len() >= BDB_MAGIC_OFFSET + 4 {
        let raw: [u8; 4] = head[BDB_MAGIC_OFFSET..BDB_MAGIC_OFFSET + 4]
            .try_into()
            .expect("slice is exactly 4 bytes");
        if u32::from_le_bytes(raw) == BDB_BTREE_MAGIC || u32::from_be_bytes(raw) == BDB_BTREE_MAGIC
        {
            return DetectedFormat::BitcoinCoreBdb;
        }
    }

    if head.starts_with(SQLITE_MAGIC) {
        // Bitcoin Core descriptor wallets are SQLite with a `main` table. Other
        // SQLite files are everywhere on a Mac, so require the marker.
        if contains(head, b"main") {
            return DetectedFormat::BitcoinCoreSqlite;
        }
        return DetectedFormat::Unknown;
    }

    if contains(head, MULTIBIT_TAG) {
        // A v3 MultiBit wallet carries scrypt parameters alongside the keys.
        if contains(head, b"scrypt") || contains(head, b"encrypted") {
            return DetectedFormat::MultibitEncrypted;
        }
        if contains(head, MULTIBIT_UNENCRYPTED_TAG) {
            return DetectedFormat::MultibitProtobuf;
        }
        return DetectedFormat::MultibitProtobuf;
    }

    // Text-shaped formats.
    let text = String::from_utf8_lossy(head);
    let trimmed = text.trim_start();

    if trimmed.starts_with("# Wallet dump created by Bitcoin")
        || trimmed.starts_with("BITCOIN_CORE_WALLET_DUMP")
    {
        return DetectedFormat::WalletDumpText;
    }

    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        if contains(head, b"pbkdf2_iterations") && contains(head, b"payload") {
            return DetectedFormat::BlockchainComAesJson;
        }
        if contains(head, b"\"desc\"") {
            return DetectedFormat::ListDescriptorsJson;
        }
    }

    if lower.ends_with(".aes.json") {
        return DetectedFormat::BlockchainComAesJson;
    }

    if is_archive_name(&lower) {
        return DetectedFormat::Archive;
    }

    DetectedFormat::Unknown
}

/// Whether a Bitcoin Core wallet's keys are encrypted under a passphrase.
///
/// Scans the file for `mkey`/`ckey` records. Cheap relative to extraction, and
/// the difference it makes to the report is the difference between "this wallet
/// is empty" and "this wallet is locked".
#[must_use]
pub fn is_encrypted_bitcoin_core(path: &Path, size: u64) -> bool {
    if size > ENCRYPTION_PROBE_BYTES {
        return false;
    }
    let Ok(data) = std::fs::read(path) else {
        return false;
    };
    contains(&data, MKEY_MARKER) || contains(&data, CKEY_MARKER)
}

fn is_archive_name(lower_name: &str) -> bool {
    ARCHIVE_EXTS
        .iter()
        .any(|e| lower_name.ends_with(&format!(".{e}")))
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || haystack.len() < needle.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

// ---------------------------------------------------------------------------
// Text content signatures
// ---------------------------------------------------------------------------

/// Strong textual signals that a file holds key material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextSignal {
    Wif,
    Xprv,
    Xpub,
    Bip39,
}

impl TextSignal {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Wif => "wif-in-text",
            Self::Xprv => "xprv-in-text",
            Self::Xpub => "xpub-in-text",
            Self::Bip39 => "bip39-mnemonic-in-text",
        }
    }
}

fn wif_re() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(r"\b([5KL9c][1-9A-HJ-NP-Za-km-z]{50,51})\b").expect("static regex")
    })
}

fn xprv_re() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(r"\b((?:xprv|yprv|zprv|tprv)[1-9A-HJ-NP-Za-km-z]{100,115})\b")
            .expect("static regex")
    })
}

fn xpub_re() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(r"\b((?:xpub|ypub|zpub)[1-9A-HJ-NP-Za-km-z]{100,115})\b")
            .expect("static regex")
    })
}

/// The English BIP39 wordlist as a set, for cheap membership pre-filtering.
fn english_words() -> &'static HashSet<&'static str> {
    static WORDS: std::sync::OnceLock<HashSet<&'static str>> = std::sync::OnceLock::new();
    WORDS.get_or_init(|| Language::English.word_list().iter().copied().collect())
}

/// Which strong signals fire on this text.
///
/// A WIF match is checked against the real base58check decoder, not just the
/// regex, because `[5KL9c]` followed by 50 base58 characters happens by
/// accident in base64 blobs and git hashes.
#[must_use]
pub fn text_signals(text: &str) -> Vec<TextSignal> {
    let mut out = Vec::new();

    if wif_re()
        .captures_iter(text)
        .any(|c| crate::crypto::wif_to_privkey(&c[1]).is_ok())
    {
        out.push(TextSignal::Wif);
    }
    if xprv_re().is_match(text) {
        out.push(TextSignal::Xprv);
    }
    if xpub_re().is_match(text) {
        out.push(TextSignal::Xpub);
    }
    if find_bip39_phrase(text).is_some() {
        out.push(TextSignal::Bip39);
    }
    out
}

/// Locate a valid BIP39 English mnemonic anywhere in `text`.
///
/// Tokenises to lowercase alphabetic runs, then tries the five legal lengths at
/// each start offset whose first word is in the wordlist. The membership
/// pre-filter is what keeps this cheap on ordinary prose.
#[must_use]
pub fn find_bip39_phrase(text: &str) -> Option<String> {
    let words: Vec<&str> = text
        .split(|c: char| !c.is_ascii_alphabetic())
        .filter(|w| !w.is_empty())
        .collect();
    if words.len() < 12 {
        return None;
    }
    let dict = english_words();

    // Lowercase once per token rather than per window.
    let lowered: Vec<String> = words.iter().map(|w| w.to_ascii_lowercase()).collect();

    for start in 0..lowered.len() {
        if !dict.contains(lowered[start].as_str()) {
            continue;
        }
        for &n in BIP39_WORD_COUNTS {
            if start + n > lowered.len() {
                continue;
            }
            let window = &lowered[start..start + n];
            if !window.iter().all(|w| dict.contains(w.as_str())) {
                continue;
            }
            let phrase = window.join(" ");
            if Mnemonic::parse_in(Language::English, &phrase).is_ok() {
                return Some(phrase);
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Filename and directory heuristics
// ---------------------------------------------------------------------------

/// Filename fragments that mark a file as worth opening even when its bytes
/// turn out to be unremarkable.
const NAME_HINTS: &[&str] = &[
    "wallet",
    "mnemonic",
    "seedphrase",
    "seed_phrase",
    "recovery",
    "privkey",
    "private_key",
    "privatekey",
    "passwords",
    "dumpwallet",
    "walletinfo",
    "electrum",
    "multibit",
    "armory",
    "keystore",
];

/// Directory-name fragments that make everything beneath them contextually
/// interesting.
const DIR_HINTS: &[&str] = &[
    "bitcoin",
    "wallet",
    "crypto",
    "btc",
    "blockchain",
    "electrum",
    "multibit",
    "armory",
    "coinbase",
    "exodus",
    "metamask",
    "trezor",
    "breadwallet",
    "mycelium",
];

#[must_use]
fn name_is_hinted(lower_name: &str) -> bool {
    NAME_HINTS.iter().any(|h| lower_name.contains(h))
        || lower_name.starts_with("wallet.dat")
        || lower_name.ends_with(".wallet")
        || lower_name.ends_with(".aes.json")
        || lower_name.ends_with(".legacy.bak")
}

#[must_use]
fn path_in_crypto_dir(path: &Path) -> bool {
    path.parent()
        .map(|parent| {
            parent.components().any(|c| {
                let s = c.as_os_str().to_string_lossy().to_ascii_lowercase();
                DIR_HINTS.iter().any(|h| s.contains(h))
            })
        })
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// The walk
// ---------------------------------------------------------------------------

/// Walk `opts.roots` and return every candidate worth a second look.
///
/// Read-only: opens files for reading and never writes beneath a root.
/// Unreadable paths are skipped silently — a machine sweep hits plenty of
/// permission-denied directories and that is not an error worth aborting on.
#[must_use]
pub fn discover(opts: &DiscoverOptions) -> Vec<Candidate> {
    let mut out = Vec::new();
    let mut seen_paths = HashSet::new();

    for root in &opts.roots {
        for entry in WalkDir::new(root)
            .follow_links(opts.follow_links)
            .into_iter()
            .filter_entry(|e| !is_pruned(e.path(), opts))
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if !seen_paths.insert(path.to_path_buf()) {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            if meta.len() == 0 || meta.len() > opts.max_file_bytes {
                continue;
            }
            if let Some(c) = inspect(path, meta.len(), opts) {
                out.push(c);
            }
        }
    }
    out
}

fn is_pruned(path: &Path, opts: &DiscoverOptions) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    // Only prune directories; a file called `Cache` is fine.
    if path.is_file() {
        return false;
    }
    PRUNE_DIRS.contains(&name) || opts.extra_prunes.iter().any(|p| p == name)
}

/// Classify one file. Returns `None` when nothing about it is interesting.
fn inspect(path: &Path, size: u64, opts: &DiscoverOptions) -> Option<Candidate> {
    let file_name = path.file_name()?.to_string_lossy().to_string();
    let lower = file_name.to_ascii_lowercase();

    let hinted_name = name_is_hinted(&lower);
    let hinted_dir = path_in_crypto_dir(path);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    let texty = ext.is_empty() || TEXT_EXTS.contains(&ext.as_str());

    // Reading the head is the only unavoidable cost. Skip it for files that are
    // neither hinted, in a crypto directory, nor plausibly text.
    if !hinted_name && !hinted_dir && !texty && !is_archive_name(&lower) {
        return None;
    }

    let head = read_head(path, HEAD_BYTES).ok()?;
    let mut format = detect_format(&file_name, &head);
    let mut signals: Vec<String> = Vec::new();

    // A Bitcoin Core wallet that turns out to be passphrase-protected gets
    // promoted, so "no keys" is never confused with "locked".
    if matches!(
        format,
        DetectedFormat::BitcoinCoreBdb | DetectedFormat::BitcoinCoreSqlite
    ) && is_encrypted_bitcoin_core(path, size)
    {
        format = DetectedFormat::BitcoinCoreEncrypted;
        signals.push("bitcoin-core-mkey/ckey-present".to_string());
    }

    if format != DetectedFormat::Unknown {
        signals.push(format!("magic:{}", format.as_str()));
    }

    // Text scan, for files small enough to read whole.
    if format == DetectedFormat::Unknown
        && texty
        && size <= opts.max_text_bytes
        && let Ok(text) = std::fs::read_to_string(path)
    {
        let sigs = text_signals(&text);
        for s in &sigs {
            signals.push(s.as_str().to_string());
        }
        format = if sigs.contains(&TextSignal::Bip39) {
            DetectedFormat::Bip39Text
        } else if sigs.contains(&TextSignal::Wif) {
            DetectedFormat::WifText
        } else if sigs.contains(&TextSignal::Xprv) {
            DetectedFormat::XprvText
        } else {
            DetectedFormat::Unknown
        };
    }

    if hinted_name {
        signals.push(format!("name:{lower}"));
    }
    if hinted_dir {
        signals.push("in-crypto-directory".to_string());
    }

    let tier = classify_tier(format, &signals);
    // A Tier D file with no signal at all is noise.
    if tier == Tier::D && !hinted_name && !hinted_dir {
        return None;
    }

    Some(Candidate {
        path: path.to_path_buf(),
        size,
        modified_epoch: modified_epoch(path),
        sha256: sha256_file(path).unwrap_or_default(),
        tier,
        format,
        signals,
    })
}

fn classify_tier(format: DetectedFormat, signals: &[String]) -> Tier {
    match format {
        DetectedFormat::BitcoinCoreBdb
        | DetectedFormat::BitcoinCoreSqlite
        | DetectedFormat::BitcoinCoreEncrypted
        | DetectedFormat::MultibitProtobuf
        | DetectedFormat::MultibitEncrypted
        | DetectedFormat::BlockchainComAesJson
        | DetectedFormat::WalletDumpText
        | DetectedFormat::ListDescriptorsJson => Tier::A,
        DetectedFormat::Bip39Text | DetectedFormat::WifText | DetectedFormat::XprvText => Tier::B,
        DetectedFormat::Archive => Tier::C,
        // An xpub is not spendable, but it proves a wallet existed and lets the
        // operator watch the addresses — worth more than bare context.
        DetectedFormat::Unknown if signals.iter().any(|s| s.contains("xpub")) => Tier::B,
        DetectedFormat::Unknown => Tier::D,
    }
}

fn read_head(path: &Path, n: usize) -> std::io::Result<Vec<u8>> {
    let mut f = std::fs::File::open(path)?;
    let mut buf = vec![0u8; n];
    let read = f.read(&mut buf)?;
    buf.truncate(read);
    Ok(buf)
}

fn modified_epoch(path: &Path) -> Option<u64> {
    std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

/// Hex SHA-256 of a file, streamed so a large wallet does not land in memory.
pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Group candidates by content digest, preserving first-seen order.
///
/// Wallets get copied — `~/Dropbox/bitcoin/wallet.dat` and
/// `~/Dropbox/BACKUP bitcoin/wallet.dat` are often byte-identical. Extraction
/// should run once per distinct digest, with every path reported.
#[must_use]
pub fn group_by_digest(candidates: &[Candidate]) -> Vec<(String, Vec<Candidate>)> {
    let mut order: Vec<String> = Vec::new();
    let mut map: std::collections::HashMap<String, Vec<Candidate>> =
        std::collections::HashMap::new();
    for c in candidates {
        let key = if c.sha256.is_empty() {
            c.path.display().to_string()
        } else {
            c.sha256.clone()
        };
        if !map.contains_key(&key) {
            order.push(key.clone());
        }
        map.entry(key).or_default().push(c.clone());
    }
    order
        .into_iter()
        .filter_map(|k| map.remove(&k).map(|v| (k, v)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A BDB btree metadata page header: 12 bytes of LSN/page-number, then the
    /// magic. This is the shape of every `wallet.dat` and `*.legacy.bak`
    /// Bitcoin Core wrote before descriptor wallets.
    fn bdb_head(big_endian: bool) -> Vec<u8> {
        let mut v = vec![0u8; BDB_MAGIC_OFFSET];
        let magic = if big_endian {
            BDB_BTREE_MAGIC.to_be_bytes()
        } else {
            BDB_BTREE_MAGIC.to_le_bytes()
        };
        v.extend_from_slice(&magic);
        v.extend_from_slice(&[0u8; 32]);
        v
    }

    #[test]
    fn detects_bdb_little_endian() {
        assert_eq!(
            detect_format("bitcoin_1776391129.legacy.bak", &bdb_head(false)),
            DetectedFormat::BitcoinCoreBdb
        );
    }

    #[test]
    fn detects_bdb_big_endian() {
        assert_eq!(
            detect_format("wallet.dat", &bdb_head(true)),
            DetectedFormat::BitcoinCoreBdb
        );
    }

    /// The regression this module exists for: a Bitcoin Core wallet backup
    /// whose name the extractor registry rejects still gets identified.
    #[test]
    fn legacy_bak_is_recognised_despite_extension() {
        let f = detect_format("bitcoin_1776392150.legacy.bak", &bdb_head(false));
        assert_eq!(f, DetectedFormat::BitcoinCoreBdb);
        assert_eq!(f.source_type(), SourceType::BitcoinCore);
        assert!(!f.needs_password());
    }

    #[test]
    fn detects_sqlite_descriptor_wallet() {
        let mut head = SQLITE_MAGIC.to_vec();
        head.extend_from_slice(b"........main........");
        assert_eq!(
            detect_format("wallet.dat", &head),
            DetectedFormat::BitcoinCoreSqlite
        );
    }

    #[test]
    fn plain_sqlite_without_main_is_not_a_wallet() {
        let mut head = SQLITE_MAGIC.to_vec();
        head.extend_from_slice(b"........history........");
        assert_eq!(
            detect_format("Safari.db", &head),
            DetectedFormat::Unknown,
            "ordinary SQLite files must not be claimed as wallets"
        );
    }

    #[test]
    fn detects_multibit_protobuf() {
        let head = b"\x0a\x16org.bitcoin.production\x12\x20".to_vec();
        assert_eq!(
            detect_format("ddaniels.wallet", &head),
            DetectedFormat::MultibitProtobuf
        );
    }

    #[test]
    fn detects_multibit_encrypted() {
        let head = b"\x0a\x16org.bitcoin.production scrypt salt".to_vec();
        let f = detect_format("ddaniels-protected.wallet", &head);
        assert_eq!(f, DetectedFormat::MultibitEncrypted);
        assert!(f.needs_password());
    }

    #[test]
    fn detects_bitcoin_core_dump_text() {
        assert_eq!(
            detect_format("dump.txt", b"# Wallet dump created by Bitcoin v0.21.0"),
            DetectedFormat::WalletDumpText
        );
    }

    /// The custom export format left behind by the 2026-04 recovery session.
    #[test]
    fn detects_prior_session_dump_header() {
        assert_eq!(
            detect_format("dump.txt", b"BITCOIN_CORE_WALLET_DUMP,1\nformat,bdb\n"),
            DetectedFormat::WalletDumpText
        );
    }

    #[test]
    fn detects_blockchain_com_backup() {
        let head = br#"{"payload":"abc","pbkdf2_iterations":5000}"#;
        assert_eq!(
            detect_format("wallet.aes.json", head),
            DetectedFormat::BlockchainComAesJson
        );
    }

    #[test]
    fn detects_listdescriptors_json() {
        let head = br#"{"wallets":[{"desc":"wpkh(xprv.../0/*)"}]}"#;
        assert_eq!(
            detect_format("descriptors.json", head),
            DetectedFormat::ListDescriptorsJson
        );
    }

    #[test]
    fn detects_archive_by_name() {
        assert_eq!(
            detect_format("bitcoin-backup.zip", b"PK\x03\x04"),
            DetectedFormat::Archive
        );
    }

    // -- text signals -------------------------------------------------------

    /// Pinned to a known-good WIF whose base58check decodes. This is the
    /// uncompressed key for private key 0x01…01 (32 bytes of 0x01).
    const KNOWN_WIF: &str = "5HpHagT65TZzG1PH3CSu63k8DbpvD8s5ip4nEB3kEsreAnchuDf";

    #[test]
    fn wif_signal_true_positive() {
        let text = format!("some notes\nkey: {KNOWN_WIF}\nmore notes");
        assert!(text_signals(&text).contains(&TextSignal::Wif));
    }

    /// A 51-character base58 string starting with `5` that is not a valid WIF
    /// must not fire. Regex alone would match this; the checksum rejects it.
    #[test]
    fn wif_signal_false_positive_rejected() {
        let fake = "5zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz";
        assert_eq!(fake.len(), 51);
        assert!(
            !text_signals(fake).contains(&TextSignal::Wif),
            "base58-shaped noise must not be reported as a private key"
        );
    }

    /// The standard BIP39 all-`abandon` test vector.
    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn bip39_found_inside_surrounding_prose() {
        let text = format!("Backup note from 2014.\n\n{TEST_MNEMONIC}\n\nkeep this safe");
        assert_eq!(find_bip39_phrase(&text).as_deref(), Some(TEST_MNEMONIC));
        assert!(text_signals(&text).contains(&TextSignal::Bip39));
    }

    #[test]
    fn bip39_false_positive_rejected() {
        // Twelve ordinary English words, none of which form a valid mnemonic.
        let text = "the quick brown fox jumps over a lazy dog while the cat sleeps soundly here";
        assert!(find_bip39_phrase(text).is_none());
        assert!(!text_signals(text).contains(&TextSignal::Bip39));
    }

    #[test]
    fn bip39_checksum_failure_rejected() {
        // Every word is in the wordlist, but the checksum is wrong.
        let bad = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon";
        assert!(
            find_bip39_phrase(bad).is_none(),
            "a wordlist-valid but checksum-invalid phrase must not be reported"
        );
    }

    #[test]
    fn xprv_and_xpub_signals() {
        let xprv = format!("xprv{}", "9".repeat(104));
        let xpub = format!("xpub{}", "9".repeat(104));
        assert!(text_signals(&xprv).contains(&TextSignal::Xprv));
        assert!(text_signals(&xpub).contains(&TextSignal::Xpub));
    }

    // -- the walk -----------------------------------------------------------

    fn tmpdir(tag: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("orpheus-discovery-{tag}-{}", std::process::id()));
        std::fs::remove_dir_all(&d).ok();
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn prunes_node_modules_but_finds_the_same_file_outside_it() {
        let root = tmpdir("prune");
        let buried = root.join("node_modules").join("pkg");
        std::fs::create_dir_all(&buried).unwrap();
        std::fs::write(buried.join("wallet.dat"), bdb_head(false)).unwrap();

        let opts = DiscoverOptions {
            roots: vec![root.clone()],
            ..Default::default()
        };
        assert!(
            discover(&opts).is_empty(),
            "node_modules must be pruned entirely"
        );

        std::fs::write(root.join("wallet.dat"), bdb_head(false)).unwrap();
        let found = discover(&opts);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].format, DetectedFormat::BitcoinCoreBdb);
        assert_eq!(found[0].tier, Tier::A);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn identical_copies_collapse_to_one_digest() {
        let root = tmpdir("dedup");
        std::fs::create_dir_all(root.join("BACKUP bitcoin")).unwrap();
        std::fs::create_dir_all(root.join("bitcoin")).unwrap();
        std::fs::write(root.join("bitcoin/wallet.dat"), bdb_head(false)).unwrap();
        std::fs::write(root.join("BACKUP bitcoin/wallet.dat"), bdb_head(false)).unwrap();

        let found = discover(&DiscoverOptions {
            roots: vec![root.clone()],
            ..Default::default()
        });
        assert_eq!(found.len(), 2, "both copies are discovered");

        let groups = group_by_digest(&found);
        assert_eq!(groups.len(), 1, "but they share one digest");
        assert_eq!(groups[0].1.len(), 2, "and both paths are retained");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn empty_files_are_ignored() {
        let root = tmpdir("empty");
        std::fs::write(root.join("wallet.dat"), b"").unwrap();
        assert!(
            discover(&DiscoverOptions {
                roots: vec![root.clone()],
                ..Default::default()
            })
            .is_empty()
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn mnemonic_text_file_is_tier_b() {
        let root = tmpdir("mnemonic");
        std::fs::write(root.join("notes.txt"), TEST_MNEMONIC).unwrap();
        let found = discover(&DiscoverOptions {
            roots: vec![root.clone()],
            ..Default::default()
        });
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].format, DetectedFormat::Bip39Text);
        assert_eq!(found[0].tier, Tier::B);
        assert_eq!(found[0].format.source_type(), SourceType::Bip39);
        std::fs::remove_dir_all(&root).ok();
    }
}
