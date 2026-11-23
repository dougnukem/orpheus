//! The hunt pipeline: discover → triage → extract → enrich → report.
//!
//! `scan` answers "what is in this directory". `hunt` answers "what is on this
//! machine, what have I already tried, and what is it worth" — which needs
//! state that outlives a single command.
//!
//! Everything a hunt produces lands in a run directory outside the repo,
//! created 0700 with 0600 files, because `keys.jsonl` holds live private keys.
//!
//! Stages `discover`, `triage`, and `extract` never open a socket. `enrich` is
//! the only networked stage and is separately invocable, preserving the
//! "extract air-gapped, check balances elsewhere" workflow.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::balance::BalanceProvider;
use crate::discovery::{
    Candidate, DetectedFormat, DiscoverOptions, Tier, discover, group_by_digest,
};
use crate::extractors::{
    Extractor, bip39_mnemonic::Bip39TextExtractor, bitcoin_core::BitcoinCoreExtractor,
    bitcoin_core_encrypted::EncryptedBitcoinCoreExtractor, encrypted::EncryptedWalletExtractor,
    multibit::MultibitExtractor, wallet_dump::WalletDumpExtractor,
};
use crate::ledger::{Attempt, AttemptOutcome, Ledger, fmt_rfc3339};
use crate::models::{BalanceInfo, ExtractedKey, TxRecord};

/// Filenames inside a run directory.
pub const INVENTORY_FILE: &str = "inventory.jsonl";
pub const ATTEMPTS_FILE: &str = "attempts.jsonl";
pub const KEYS_FILE: &str = "keys.jsonl";
pub const BALANCES_FILE: &str = "balances.jsonl";
pub const TRANSACTIONS_FILE: &str = "transactions.jsonl";
/// Addresses whose balance lookup never succeeded. Distinct from a zero
/// balance, and surfaced in the report so they can be retried.
pub const FAILURES_FILE: &str = "lookup_failures.jsonl";
pub const REPORT_FILE: &str = "report.md";

/// A hunt's on-disk workspace.
///
/// Lives at `~/.orpheus/hunt/<run-id>/` by default — deliberately outside any
/// git repository, because this directory contains private keys.
#[derive(Debug, Clone)]
pub struct RunDir {
    pub root: PathBuf,
    pub id: String,
}

impl RunDir {
    /// Create (or reopen) a run directory.
    pub fn create(base: Option<&Path>, id: Option<&str>) -> std::io::Result<Self> {
        let base = match base {
            Some(b) => b.to_path_buf(),
            None => default_base()?,
        };
        let id = match id {
            Some(i) => i.to_string(),
            None => {
                let epoch = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                fmt_rfc3339(epoch).replace([':', '-'], "").replace('Z', "")
            }
        };
        let root = base.join(&id);
        std::fs::create_dir_all(&root)?;
        harden_dir(&root)?;
        Ok(Self { root, id })
    }

    /// Reopen the most recent run under `base`, if any.
    pub fn latest(base: Option<&Path>) -> std::io::Result<Option<Self>> {
        let base = match base {
            Some(b) => b.to_path_buf(),
            None => default_base()?,
        };
        if !base.exists() {
            return Ok(None);
        }
        let mut ids: Vec<String> = std::fs::read_dir(&base)?
            .filter_map(Result::ok)
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .filter_map(|e| e.file_name().to_str().map(str::to_string))
            .collect();
        ids.sort();
        Ok(ids.pop().map(|id| Self {
            root: base.join(&id),
            id,
        }))
    }

    #[must_use]
    pub fn file(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    pub fn ledger(&self) -> std::io::Result<Ledger> {
        Ledger::open(self.file(ATTEMPTS_FILE))
    }
}

fn default_base() -> std::io::Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "no HOME or USERPROFILE set")
        })?;
    Ok(PathBuf::from(home).join(".orpheus").join("hunt"))
}

#[cfg(unix)]
fn harden_dir(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn harden_dir(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn harden_file(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn harden_file(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

/// Write a slice as JSON Lines, hardening the file afterwards.
fn write_jsonl<T: Serialize>(path: &Path, items: &[T]) -> std::io::Result<()> {
    let mut f = std::fs::File::create(path)?;
    for item in items {
        let line = serde_json::to_string(item)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writeln!(f, "{line}")?;
    }
    drop(f);
    harden_file(path)
}

fn read_jsonl<T: for<'de> Deserialize<'de>>(path: &Path) -> std::io::Result<Vec<T>> {
    use std::io::BufRead;
    if !path.exists() {
        return Ok(vec![]);
    }
    let f = std::fs::File::open(path)?;
    Ok(std::io::BufReader::new(f)
        .lines()
        .map_while(Result::ok)
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<T>(&l).ok())
        .collect())
}

// ---------------------------------------------------------------------------
// Stage 1 + 2 — discover and triage
// ---------------------------------------------------------------------------

/// Walk the roots and persist the inventory. Offline.
pub fn stage_discover(run: &RunDir, opts: &DiscoverOptions) -> std::io::Result<Vec<Candidate>> {
    let candidates = discover(opts);
    write_jsonl(&run.file(INVENTORY_FILE), &candidates)?;
    Ok(candidates)
}

pub fn load_inventory(run: &RunDir) -> std::io::Result<Vec<Candidate>> {
    read_jsonl(&run.file(INVENTORY_FILE))
}

/// Pick the extractor for an already-identified format.
///
/// This is the whole point of the triage stage. `scan` asks each extractor
/// "can you handle this filename?"; `hunt` already knows what the bytes are and
/// hands the file to the right extractor regardless of what it is called. It is
/// how `bitcoin_1776391129.legacy.bak` reaches the Bitcoin Core extractor.
#[must_use]
pub fn extractor_for_format(format: DetectedFormat) -> Option<Box<dyn Extractor>> {
    match format {
        DetectedFormat::BitcoinCoreBdb | DetectedFormat::BitcoinCoreSqlite => {
            Some(Box::new(BitcoinCoreExtractor))
        }
        DetectedFormat::BitcoinCoreEncrypted => Some(Box::new(EncryptedBitcoinCoreExtractor)),
        DetectedFormat::MultibitProtobuf => Some(Box::new(MultibitExtractor)),
        DetectedFormat::MultibitEncrypted | DetectedFormat::BlockchainComAesJson => {
            Some(Box::new(EncryptedWalletExtractor))
        }
        DetectedFormat::WalletDumpText
        | DetectedFormat::ListDescriptorsJson
        | DetectedFormat::WifText
        | DetectedFormat::XprvText => Some(Box::new(WalletDumpExtractor)),
        DetectedFormat::Bip39Text => Some(Box::new(Bip39TextExtractor)),
        DetectedFormat::Archive | DetectedFormat::Unknown => None,
    }
}

// ---------------------------------------------------------------------------
// Stage 3 — extract (offline)
// ---------------------------------------------------------------------------

/// Result of extracting one distinct artifact (one content digest).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactResult {
    pub digest: String,
    /// Every path that holds these identical bytes.
    pub paths: Vec<String>,
    pub format: DetectedFormat,
    pub tier: Tier,
    pub outcome: AttemptOutcome,
    pub keys: Vec<ExtractedKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Run extractors over the inventory, once per distinct digest.
///
/// Offline — no provider is consulted. Every attempt is appended to the ledger
/// whether it succeeded or not; that record is what makes a later rerun able to
/// skip solved artifacts and retry locked ones with a longer password list.
pub fn stage_extract(
    run: &RunDir,
    candidates: &[Candidate],
    passwords: &[String],
    skip_solved: bool,
) -> std::io::Result<Vec<ArtifactResult>> {
    let ledger = run.ledger()?;
    let already: HashSet<String> = if skip_solved {
        ledger.succeeded_digests()?
    } else {
        HashSet::new()
    };

    let mut results = Vec::new();
    for (digest, group) in group_by_digest(candidates) {
        let first = &group[0];
        let paths: Vec<String> = group.iter().map(|c| c.path.display().to_string()).collect();

        if already.contains(&digest) {
            continue;
        }
        if !first.tier.is_extractable() {
            ledger.append(
                &Attempt::now(&digest, &paths[0], first.format.as_str(), "-")
                    .outcome(AttemptOutcome::Skipped, 0),
            )?;
            results.push(ArtifactResult {
                digest,
                paths,
                format: first.format,
                tier: first.tier,
                outcome: AttemptOutcome::Skipped,
                keys: vec![],
                error: None,
            });
            continue;
        }

        let Some(extractor) = extractor_for_format(first.format) else {
            continue;
        };
        let extractor_name = first.format.source_type().as_str();
        let scan = extractor.extract(&first.path, passwords);

        // Order matters. An encrypted wallet reports *why* it stayed shut in
        // `error`, but "locked" is the useful classification — it is retryable
        // with a longer password list, where a genuine error is not.
        let outcome = if !scan.keys.is_empty() {
            AttemptOutcome::Success
        } else if first.format.needs_password() {
            AttemptOutcome::NeedsPassword
        } else if scan.error.is_some() {
            AttemptOutcome::Error
        } else {
            AttemptOutcome::NoKeys
        };

        ledger.append(
            &Attempt::now(&digest, &paths[0], first.format.as_str(), extractor_name)
                .passwords(passwords.len())
                .outcome(outcome, scan.keys.len())
                .error(scan.error.clone()),
        )?;

        results.push(ArtifactResult {
            digest,
            paths,
            format: first.format,
            tier: first.tier,
            outcome,
            keys: scan.keys,
            error: scan.error,
        });
    }

    write_jsonl(&run.file(KEYS_FILE), &results)?;
    Ok(results)
}

pub fn load_results(run: &RunDir) -> std::io::Result<Vec<ArtifactResult>> {
    read_jsonl(&run.file(KEYS_FILE))
}

// ---------------------------------------------------------------------------
// Stage 4 — enrich (network)
// ---------------------------------------------------------------------------

/// Every distinct address across all recovered keys, in stable order.
#[must_use]
pub fn unique_addresses(results: &[ArtifactResult]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for r in results {
        for k in &r.keys {
            for addr in [
                Some(&k.address_compressed),
                k.address_uncompressed.as_ref(),
                k.address_p2sh_segwit.as_ref(),
                k.address_bech32.as_ref(),
            ]
            .into_iter()
            .flatten()
            {
                if !addr.is_empty() && seen.insert(addr.clone()) {
                    out.push(addr.clone());
                }
            }
        }
    }
    out
}

/// Look up balances for every address, and transaction history for every
/// address that has seen activity.
///
/// History is fetched only where `tx_count > 0`, which keeps the request count
/// proportional to real activity rather than to the size of the keyspace a
/// BIP39 seed derives.
pub fn stage_enrich(
    run: &RunDir,
    results: &[ArtifactResult],
    provider: &dyn BalanceProvider,
    fetch_transactions: bool,
) -> std::io::Result<EnrichOutcome> {
    let addresses = unique_addresses(results);
    let map = provider.fetch(&addresses);

    // A provider omits an address it could not look up. Those are unknown, not
    // empty, and the distinction is carried through to the report.
    let failed: Vec<String> = addresses
        .iter()
        .filter(|a| !map.contains_key(*a))
        .cloned()
        .collect();

    let mut balances: Vec<BalanceInfo> = addresses
        .iter()
        .filter_map(|a| map.get(a).cloned())
        .collect();
    balances.sort_by_key(|b| std::cmp::Reverse((b.balance_sat, b.total_received_sat)));
    write_jsonl(&run.file(BALANCES_FILE), &balances)?;
    write_jsonl(&run.file(FAILURES_FILE), &failed)?;

    let mut txs = Vec::new();
    if fetch_transactions && provider.supports_transactions() {
        for b in &balances {
            if b.tx_count == 0 {
                continue;
            }
            txs.extend(provider.transactions(&b.address));
        }
    }
    write_jsonl(&run.file(TRANSACTIONS_FILE), &txs)?;

    Ok(EnrichOutcome {
        balances,
        transactions: txs,
        failed_lookups: failed,
    })
}

/// What [`stage_enrich`] produced.
#[derive(Debug, Clone, Default)]
pub struct EnrichOutcome {
    pub balances: Vec<BalanceInfo>,
    pub transactions: Vec<TxRecord>,
    /// Addresses the provider could not answer for. Retry before concluding
    /// anything about them.
    pub failed_lookups: Vec<String>,
}

pub fn load_failures(run: &RunDir) -> std::io::Result<Vec<String>> {
    read_jsonl(&run.file(FAILURES_FILE))
}

pub fn load_balances(run: &RunDir) -> std::io::Result<Vec<BalanceInfo>> {
    read_jsonl(&run.file(BALANCES_FILE))
}

pub fn load_transactions(run: &RunDir) -> std::io::Result<Vec<TxRecord>> {
    read_jsonl(&run.file(TRANSACTIONS_FILE))
}

// ---------------------------------------------------------------------------
// Stage 5 — report
// ---------------------------------------------------------------------------

#[must_use]
pub fn sat_to_btc(sat: u64) -> String {
    format!("{:.8}", sat as f64 / 1.0e8)
}

#[must_use]
fn signed_sat_to_btc(sat: i64) -> String {
    format!(
        "{}{:.8}",
        if sat < 0 { "-" } else { "+" },
        sat.abs() as f64 / 1.0e8
    )
}

/// Shorten a secret for display. Full keys stay in `keys.jsonl`.
#[must_use]
pub fn redact(secret: &str) -> String {
    if secret.len() <= 12 {
        return "…".to_string();
    }
    format!("{}…{}", &secret[..6], &secret[secret.len() - 4..])
}

/// Rendered summary numbers, so the CLI can print them without re-deriving.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HuntSummary {
    pub candidates: usize,
    pub distinct_artifacts: usize,
    pub extracted_wallets: usize,
    pub total_keys: usize,
    pub unique_addresses: usize,
    pub funded_addresses: usize,
    pub spent_addresses: usize,
    pub unfunded_addresses: usize,
    pub total_received_sat: u64,
    pub total_sent_sat: u64,
    pub total_balance_sat: u64,
    pub needs_password: usize,
    pub archives_flagged: usize,
}

#[must_use]
pub fn summarize(
    candidates: &[Candidate],
    results: &[ArtifactResult],
    balances: &[BalanceInfo],
) -> HuntSummary {
    HuntSummary {
        candidates: candidates.len(),
        distinct_artifacts: results.len(),
        extracted_wallets: results
            .iter()
            .filter(|r| r.outcome == AttemptOutcome::Success)
            .count(),
        total_keys: results.iter().map(|r| r.keys.len()).sum(),
        // Counted from the recovered keys, not from `balances`, so an offline
        // run (`--provider none`) still reports how many addresses it found.
        unique_addresses: unique_addresses(results).len(),
        funded_addresses: balances.iter().filter(|b| b.balance_sat > 0).count(),
        spent_addresses: balances
            .iter()
            .filter(|b| b.balance_sat == 0 && b.total_received_sat > 0)
            .count(),
        unfunded_addresses: balances
            .iter()
            .filter(|b| b.total_received_sat == 0)
            .count(),
        total_received_sat: balances.iter().map(|b| b.total_received_sat).sum(),
        total_sent_sat: balances.iter().map(|b| b.total_sent_sat).sum(),
        total_balance_sat: balances.iter().map(|b| b.balance_sat).sum(),
        needs_password: results
            .iter()
            .filter(|r| r.outcome == AttemptOutcome::NeedsPassword)
            .count(),
        archives_flagged: candidates.iter().filter(|c| c.tier == Tier::C).count(),
    }
}

/// Everything the report renderer needs. Grouped into a struct because the
/// five stages each contribute one slice and a positional argument list that
/// long is easy to misorder.
pub struct ReportInput<'a> {
    pub run: &'a RunDir,
    pub candidates: &'a [Candidate],
    pub results: &'a [ArtifactResult],
    pub balances: &'a [BalanceInfo],
    pub transactions: &'a [TxRecord],
    pub attempts: &'a [Attempt],
    /// Addresses the provider could not answer for — reported separately so
    /// they are never read as "empty".
    pub failed_lookups: &'a [String],
    pub provider_name: &'a str,
    /// Print full private keys instead of `L1aW…8xQz`.
    pub unredact: bool,
}

/// Render the human-facing Markdown report.
///
/// Redacted by default: WIFs appear as `L1aW…8xQz`. `unredact` prints them in
/// full, which is what the operator needs when they are ready to sweep.
#[must_use]
pub fn render_report(input: &ReportInput<'_>) -> String {
    let ReportInput {
        run,
        candidates,
        results,
        balances,
        transactions,
        attempts,
        failed_lookups,
        provider_name,
        unredact,
    } = *input;

    let s = summarize(candidates, results, balances);
    let mut o = String::new();
    let show = |wif: &str| {
        if unredact {
            wif.to_string()
        } else {
            redact(wif)
        }
    };

    o.push_str(&format!("# Orpheus hunt — run `{}`\n\n", run.id));
    if !unredact {
        o.push_str(
            "> Private keys are redacted. Full WIFs are in `keys.jsonl` in this run\n\
             > directory, or re-render with `--unredact`.\n\n",
        );
    }

    // -- headline ----------------------------------------------------------
    o.push_str("## Headline\n\n");
    o.push_str(&format!(
        "| | |\n|---|---:|\n\
         | Candidates found | {} |\n\
         | Distinct artifacts (deduped) | {} |\n\
         | Wallets successfully extracted | {} |\n\
         | Private keys recovered | {} |\n\
         | Unique addresses | {} |\n\
         | **Currently funded addresses** | **{}** |\n\
         | Addresses with past activity, now empty | {} |\n\
         | Never-used addresses | {} |\n\
         | **Current balance** | **{} BTC** |\n\
         | Total ever received | {} BTC |\n\
         | Total ever sent | {} BTC |\n\
         | Wallets still locked (need a password) | {} |\n\
         | Archives flagged for manual review | {} |\n\
         | Balance provider | {} |\n\n",
        s.candidates,
        s.distinct_artifacts,
        s.extracted_wallets,
        s.total_keys,
        s.unique_addresses,
        s.funded_addresses,
        s.spent_addresses,
        s.unfunded_addresses,
        sat_to_btc(s.total_balance_sat),
        sat_to_btc(s.total_received_sat),
        sat_to_btc(s.total_sent_sat),
        s.needs_password,
        s.archives_flagged,
        provider_name,
    ));

    // -- money -------------------------------------------------------------
    let funded: Vec<&BalanceInfo> = balances.iter().filter(|b| b.balance_sat > 0).collect();
    o.push_str("## Funded addresses\n\n");
    if funded.is_empty() {
        o.push_str("None. No recovered address currently holds a balance.\n\n");
    } else {
        o.push_str("| Address | Balance (BTC) | Received | Txs | Key | Source |\n");
        o.push_str("|---|---:|---:|---:|---|---|\n");
        for b in &funded {
            let (wif, src) = key_for_address(results, &b.address);
            o.push_str(&format!(
                "| `{}` | **{}** | {} | {} | `{}` | {} |\n",
                b.address,
                sat_to_btc(b.balance_sat),
                sat_to_btc(b.total_received_sat),
                b.tx_count,
                wif.map(|w| show(&w)).unwrap_or_else(|| "-".into()),
                src.unwrap_or_else(|| "-".into()),
            ));
        }
        o.push('\n');
    }

    let spent: Vec<&BalanceInfo> = balances
        .iter()
        .filter(|b| b.balance_sat == 0 && b.total_received_sat > 0)
        .collect();
    o.push_str("## Addresses with history but no remaining balance\n\n");
    if spent.is_empty() {
        o.push_str("None.\n\n");
    } else {
        o.push_str(&format!(
            "{} address(es) received funds at some point and were emptied.\n\n",
            spent.len()
        ));
        o.push_str("| Address | Ever received (BTC) | Sent | Txs |\n|---|---:|---:|---:|\n");
        for b in spent.iter().take(50) {
            o.push_str(&format!(
                "| `{}` | {} | {} | {} |\n",
                b.address,
                sat_to_btc(b.total_received_sat),
                sat_to_btc(b.total_sent_sat),
                b.tx_count
            ));
        }
        if spent.len() > 50 {
            o.push_str(&format!(
                "\n_…and {} more. Full list in `balances.jsonl`._\n",
                spent.len() - 50
            ));
        }
        o.push('\n');
    }

    // -- lookup failures ---------------------------------------------------
    if !failed_lookups.is_empty() {
        o.push_str("## ⚠ Unresolved address lookups\n\n");
        o.push_str(&format!(
            "{} address(es) could not be checked — the provider errored or rate-limited.\n\
             **These are unknown, not empty.** Rerun `orpheus hunt enrich` before\n\
             concluding anything about them.\n\n",
            failed_lookups.len()
        ));
        for a in failed_lookups.iter().take(25) {
            o.push_str(&format!("- `{a}`\n"));
        }
        if failed_lookups.len() > 25 {
            o.push_str(&format!(
                "\n_…and {} more in `lookup_failures.jsonl`._\n",
                failed_lookups.len() - 25
            ));
        }
        o.push('\n');
    }

    // -- transactions ------------------------------------------------------
    o.push_str("## Transactions\n\n");
    if transactions.is_empty() {
        o.push_str("No transaction history retrieved.\n\n");
    } else {
        let mut by_addr: BTreeMap<&str, Vec<&TxRecord>> = BTreeMap::new();
        for t in transactions {
            by_addr.entry(t.address.as_str()).or_default().push(t);
        }
        o.push_str(&format!(
            "{} transaction(s) across {} address(es).\n\n",
            transactions.len(),
            by_addr.len()
        ));
        for (addr, mut txs) in by_addr {
            txs.sort_by_key(|t| t.block_time.unwrap_or(0));
            o.push_str(&format!("### `{addr}`\n\n"));
            o.push_str("| Date | Txid | Net (BTC) | Height |\n|---|---|---:|---:|\n");
            for t in txs {
                o.push_str(&format!(
                    "| {} | `{}` | {} | {} |\n",
                    t.block_time
                        .map(fmt_rfc3339)
                        .unwrap_or_else(|| "unconfirmed".into()),
                    redact_txid(&t.txid),
                    signed_sat_to_btc(t.net_value_sat),
                    t.block_height
                        .map(|h| h.to_string())
                        .unwrap_or_else(|| "-".into()),
                ));
            }
            o.push('\n');
        }
    }

    // -- inventory ---------------------------------------------------------
    o.push_str("## Inventory\n\n");
    let mut by_tier: BTreeMap<&str, usize> = BTreeMap::new();
    let mut by_format: BTreeMap<&str, usize> = BTreeMap::new();
    for c in candidates {
        *by_tier.entry(c.tier.as_str()).or_default() += 1;
        *by_format.entry(c.format.as_str()).or_default() += 1;
    }
    o.push_str("| Tier | Meaning | Count |\n|---|---|---:|\n");
    for (t, n) in &by_tier {
        let meaning = match *t {
            "A" => "identified wallet container",
            "B" => "text with key or seed material",
            "C" => "archive / encrypted container (not opened)",
            _ => "contextual, needs human review",
        };
        o.push_str(&format!("| {t} | {meaning} | {n} |\n"));
    }
    o.push_str("\n| Detected format | Count |\n|---|---:|\n");
    for (f, n) in &by_format {
        o.push_str(&format!("| `{f}` | {n} |\n"));
    }
    o.push('\n');

    // -- per-wallet outcomes ----------------------------------------------
    o.push_str("## Extraction outcomes\n\n");
    o.push_str("| Artifact | Format | Outcome | Keys | Copies |\n|---|---|---|---:|---:|\n");
    let mut sorted: Vec<&ArtifactResult> = results.iter().collect();
    sorted.sort_by_key(|r| std::cmp::Reverse(r.keys.len()));
    for r in sorted
        .iter()
        .filter(|r| r.outcome != AttemptOutcome::Skipped || r.format != DetectedFormat::Unknown)
    {
        o.push_str(&format!(
            "| `{}` | `{}` | {} | {} | {} |\n",
            shorten_path(&r.paths[0]),
            r.format.as_str(),
            r.outcome.as_str(),
            r.keys.len(),
            r.paths.len(),
        ));
    }
    o.push('\n');

    // -- follow-ups --------------------------------------------------------
    o.push_str("## Follow-ups\n\n");
    let locked: Vec<&ArtifactResult> = results
        .iter()
        .filter(|r| r.outcome == AttemptOutcome::NeedsPassword)
        .collect();
    if locked.is_empty() {
        o.push_str("- No encrypted wallets are waiting on a password.\n");
    } else {
        o.push_str("**Encrypted wallets — no supplied password worked:**\n\n");
        for r in &locked {
            o.push_str(&format!(
                "- `{}` (`{}`)\n",
                shorten_path(&r.paths[0]),
                r.format.as_str()
            ));
        }
        o.push_str(
            "\nExtend the password list and rerun `orpheus hunt extract`; solved\n\
             artifacts are skipped automatically. For heavier search, see\n\
             `docs/password-recovery.md`.\n",
        );
    }

    let archives: Vec<&Candidate> = candidates.iter().filter(|c| c.tier == Tier::C).collect();
    if !archives.is_empty() {
        o.push_str(&format!(
            "\n**Archives and encrypted containers ({}) — not opened by the hunt:**\n\n",
            archives.len()
        ));
        for c in archives.iter().take(40) {
            o.push_str(&format!(
                "- `{}`\n",
                shorten_path(&c.path.display().to_string())
            ));
        }
        if archives.len() > 40 {
            o.push_str(&format!(
                "\n_…and {} more in `inventory.jsonl`._\n",
                archives.len() - 40
            ));
        }
        o.push_str("\nUnpack any that look plausible and rerun the hunt against the\nextracted directory.\n");
    }

    // -- ledger ------------------------------------------------------------
    o.push_str("\n## Recovery-attempt ledger\n\n");
    if attempts.is_empty() {
        o.push_str("No attempts recorded.\n");
    } else {
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for a in attempts {
            *counts.entry(a.outcome.as_str()).or_default() += 1;
        }
        o.push_str(&format!(
            "{} attempt(s) on record in `attempts.jsonl`.\n\n",
            attempts.len()
        ));
        o.push_str("| Outcome | Count |\n|---|---:|\n");
        for (k, v) in &counts {
            o.push_str(&format!("| {k} | {v} |\n"));
        }
        o.push_str("\nThe ledger is append-only and keyed on content digest, so a rerun\nskips artifacts that already gave up their keys and retries the rest.\n");
    }

    o.push_str(&format!(
        "\n---\n\nRun directory: `{}`\n\n\
         - `inventory.jsonl` — every candidate, with tier, format, and digest\n\
         - `attempts.jsonl` — the append-only attempt ledger\n\
         - `keys.jsonl` — **sensitive**, holds recovered private keys\n\
         - `balances.jsonl`, `transactions.jsonl` — enrichment output\n\n\
         If you sweep any funds, move them immediately — a key that sat in\n\
         Dropbox for a decade should be treated as compromised.\n",
        run.root.display()
    ));

    o
}

fn redact_txid(txid: &str) -> String {
    if txid.len() <= 16 {
        return txid.to_string();
    }
    format!("{}…{}", &txid[..8], &txid[txid.len() - 6..])
}

fn shorten_path(p: &str) -> String {
    match std::env::var("HOME") {
        Ok(home) if !home.is_empty() && p.starts_with(&home) => p.replacen(&home, "~", 1),
        _ => p.to_string(),
    }
}

/// Find a recovered key matching `address`, returning its WIF and source path.
fn key_for_address(results: &[ArtifactResult], address: &str) -> (Option<String>, Option<String>) {
    for r in results {
        for k in &r.keys {
            let hit = k.address_compressed == address
                || k.address_uncompressed.as_deref() == Some(address)
                || k.address_p2sh_segwit.as_deref() == Some(address)
                || k.address_bech32.as_deref() == Some(address);
            if hit {
                return (
                    Some(k.wif.clone()),
                    Some(shorten_path(
                        r.paths.first().map(String::as_str).unwrap_or("-"),
                    )),
                );
            }
        }
    }
    (None, None)
}

/// Persist the report and harden it.
pub fn write_report(run: &RunDir, markdown: &str) -> std::io::Result<PathBuf> {
    let path = run.file(REPORT_FILE);
    std::fs::write(&path, markdown)?;
    harden_file(&path)?;
    Ok(path)
}

/// Load candidate passwords, one per line, blanks stripped.
pub fn load_passwords(path: &Path) -> std::io::Result<Vec<String>> {
    Ok(std::fs::read_to_string(path)?
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

/// Balances keyed by address, for callers that want random access.
#[must_use]
pub fn balance_map(balances: &[BalanceInfo]) -> HashMap<String, BalanceInfo> {
    balances
        .iter()
        .map(|b| (b.address.clone(), b.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SourceType;

    fn tmp_base(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("orpheus-hunt-{tag}-{}", std::process::id()));
        std::fs::remove_dir_all(&p).ok();
        p
    }

    fn key(addr: &str, wif: &str) -> ExtractedKey {
        ExtractedKey {
            wif: wif.into(),
            address_compressed: addr.into(),
            address_uncompressed: None,
            address_p2sh_segwit: None,
            address_bech32: None,
            source_file: "/tmp/w.dat".into(),
            source_type: SourceType::BitcoinCore,
            derivation_path: None,
            balance_sat: None,
            total_received_sat: None,
            total_sent_sat: None,
            tx_count: None,
            notes: None,
        }
    }

    #[test]
    fn run_dir_is_created_private() {
        let base = tmp_base("private");
        let run = RunDir::create(Some(&base), Some("testrun")).unwrap();
        assert!(run.root.is_dir());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&run.root).unwrap().permissions().mode();
            assert_eq!(
                mode & 0o777,
                0o700,
                "run dir must not be group/world readable"
            );
        }
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn latest_returns_the_newest_run() {
        let base = tmp_base("latest");
        RunDir::create(Some(&base), Some("20260101T000000")).unwrap();
        RunDir::create(Some(&base), Some("20260802T195151")).unwrap();
        let latest = RunDir::latest(Some(&base)).unwrap().unwrap();
        assert_eq!(latest.id, "20260802T195151");
        std::fs::remove_dir_all(&base).ok();
    }

    /// The dispatch table is the fix for filename-driven detection. A Bitcoin
    /// Core backup named `.legacy.bak` must reach the Bitcoin Core extractor.
    #[test]
    fn format_dispatch_routes_legacy_bak_to_bitcoin_core() {
        let ex = extractor_for_format(DetectedFormat::BitcoinCoreBdb).expect("extractor");
        assert_eq!(ex.source_type(), SourceType::BitcoinCore);
        assert!(
            !ex.can_handle(Path::new("/tmp/bitcoin_1776391129.legacy.bak")),
            "precondition: can_handle still rejects this name, which is why \
             hunt dispatches on format instead"
        );
    }

    #[test]
    fn format_dispatch_covers_every_extractable_format() {
        for f in [
            DetectedFormat::BitcoinCoreBdb,
            DetectedFormat::BitcoinCoreSqlite,
            DetectedFormat::MultibitProtobuf,
            DetectedFormat::MultibitEncrypted,
            DetectedFormat::BlockchainComAesJson,
            DetectedFormat::WalletDumpText,
            DetectedFormat::ListDescriptorsJson,
            DetectedFormat::WifText,
            DetectedFormat::XprvText,
            DetectedFormat::Bip39Text,
        ] {
            assert!(
                extractor_for_format(f).is_some(),
                "{} must have an extractor",
                f.as_str()
            );
        }
        assert!(extractor_for_format(DetectedFormat::Archive).is_none());
        assert!(extractor_for_format(DetectedFormat::Unknown).is_none());
    }

    #[test]
    fn unique_addresses_dedupes_across_wallets() {
        let results = vec![
            ArtifactResult {
                digest: "a".into(),
                paths: vec!["/tmp/a".into()],
                format: DetectedFormat::BitcoinCoreBdb,
                tier: Tier::A,
                outcome: AttemptOutcome::Success,
                keys: vec![key("1AAA", "L1"), key("1BBB", "L2")],
                error: None,
            },
            ArtifactResult {
                digest: "b".into(),
                paths: vec!["/tmp/b".into()],
                format: DetectedFormat::BitcoinCoreBdb,
                tier: Tier::A,
                outcome: AttemptOutcome::Success,
                keys: vec![key("1AAA", "L1")],
                error: None,
            },
        ];
        assert_eq!(unique_addresses(&results), vec!["1AAA", "1BBB"]);
    }

    #[test]
    fn redaction_hides_the_middle_of_a_key() {
        let wif = "L1aWxyzABCDEFGHIJKLMNOPQRSTUVWX8xQz";
        let r = redact(wif);
        assert!(r.starts_with("L1aWxy"));
        assert!(r.ends_with("8xQz"));
        assert!(!r.contains("MNOPQ"), "the body of the key must not survive");
    }

    #[test]
    fn report_redacts_by_default_and_reveals_on_request() {
        let base = tmp_base("report");
        let run = RunDir::create(Some(&base), Some("r1")).unwrap();
        let wif = "L1aWxyzABCDEFGHIJKLMNOPQRSTUVWX8xQz";
        let results = vec![ArtifactResult {
            digest: "d".into(),
            paths: vec!["/tmp/wallet.dat".into()],
            format: DetectedFormat::BitcoinCoreBdb,
            tier: Tier::A,
            outcome: AttemptOutcome::Success,
            keys: vec![key("1Funded", wif)],
            error: None,
        }];
        let balances = vec![BalanceInfo {
            address: "1Funded".into(),
            balance_sat: 3_865_052,
            total_received_sat: 5_000_000,
            total_sent_sat: 1_134_948,
            tx_count: 4,
        }];

        let input = |unredact| ReportInput {
            run: &run,
            candidates: &[],
            results: &results,
            balances: &balances,
            transactions: &[],
            attempts: &[],
            failed_lookups: &[],
            provider_name: "blockstream",
            unredact,
        };

        let redacted = render_report(&input(false));
        assert!(
            !redacted.contains(wif),
            "default report must not leak a WIF"
        );
        assert!(redacted.contains("0.03865052"));
        assert!(redacted.contains("1Funded"));

        let full = render_report(&input(true));
        assert!(full.contains(wif), "--unredact must reveal the key");

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn summary_partitions_addresses_correctly() {
        let balances = vec![
            BalanceInfo {
                address: "funded".into(),
                balance_sat: 100,
                total_received_sat: 100,
                total_sent_sat: 0,
                tx_count: 1,
            },
            BalanceInfo {
                address: "spent".into(),
                balance_sat: 0,
                total_received_sat: 500,
                total_sent_sat: 500,
                tx_count: 2,
            },
            BalanceInfo {
                address: "never".into(),
                balance_sat: 0,
                total_received_sat: 0,
                total_sent_sat: 0,
                tx_count: 0,
            },
        ];
        let s = summarize(&[], &[], &balances);
        assert_eq!(s.funded_addresses, 1);
        assert_eq!(s.spent_addresses, 1);
        assert_eq!(s.unfunded_addresses, 1);
        assert_eq!(s.total_balance_sat, 100);
        assert_eq!(s.total_received_sat, 600);
    }

    /// A provider that cannot answer must not have its silence rendered as a
    /// zero balance — that is how a funded address gets written off as empty.
    #[test]
    fn failed_lookups_are_reported_separately_from_empty_ones() {
        let base = tmp_base("failures");
        let run = RunDir::create(Some(&base), Some("r1")).unwrap();
        let balances = vec![BalanceInfo {
            address: "1Empty".into(),
            balance_sat: 0,
            total_received_sat: 0,
            total_sent_sat: 0,
            tx_count: 0,
        }];
        let failed = vec!["1Unknown".to_string()];

        let md = render_report(&ReportInput {
            run: &run,
            candidates: &[],
            results: &[],
            balances: &balances,
            transactions: &[],
            attempts: &[],
            failed_lookups: &failed,
            provider_name: "blockstream",
            unredact: false,
        });

        assert!(md.contains("Unresolved address lookups"));
        assert!(md.contains("1Unknown"));
        assert!(md.contains("unknown, not empty"));

        // And the summary must not count the unknown address as never-used.
        let s = summarize(&[], &[], &balances);
        assert_eq!(s.unfunded_addresses, 1, "only the genuinely empty one");

        std::fs::remove_dir_all(&base).ok();
    }

    /// `stage_enrich` must classify an address the provider omitted as failed
    /// rather than inventing a zero for it.
    #[test]
    fn enrich_marks_omitted_addresses_as_failed() {
        struct HalfBlindProvider;
        impl BalanceProvider for HalfBlindProvider {
            fn name(&self) -> &'static str {
                "half-blind"
            }
            fn fetch(&self, addresses: &[String]) -> HashMap<String, BalanceInfo> {
                // Answers for the first address only, as a rate-limited
                // provider would.
                addresses
                    .iter()
                    .take(1)
                    .map(|a| (a.clone(), BalanceInfo::zero(a.clone())))
                    .collect()
            }
        }

        let base = tmp_base("enrich-failed");
        let run = RunDir::create(Some(&base), Some("r1")).unwrap();
        let results = vec![ArtifactResult {
            digest: "d".into(),
            paths: vec!["/tmp/w".into()],
            format: DetectedFormat::BitcoinCoreBdb,
            tier: Tier::A,
            outcome: AttemptOutcome::Success,
            keys: vec![key("1AAA", "L1"), key("1BBB", "L2")],
            error: None,
        }];

        let out = stage_enrich(&run, &results, &HalfBlindProvider, false).unwrap();
        assert_eq!(out.balances.len(), 1);
        assert_eq!(out.failed_lookups, vec!["1BBB".to_string()]);
        assert_eq!(load_failures(&run).unwrap(), vec!["1BBB".to_string()]);

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn signed_btc_formatting_marks_direction() {
        assert_eq!(signed_sat_to_btc(5_000_000), "+0.05000000");
        assert_eq!(signed_sat_to_btc(-5_000_000), "-0.05000000");
    }

    /// A rerun must not redo work the ledger already records as solved.
    #[test]
    fn extract_skips_digests_that_already_succeeded() {
        let base = tmp_base("skip");
        let run = RunDir::create(Some(&base), Some("r1")).unwrap();
        let ledger = run.ledger().unwrap();
        ledger
            .append(
                &Attempt::now("deadbeef", "/tmp/w.dat", "bitcoin_core_bdb", "bitcoin_core")
                    .outcome(AttemptOutcome::Success, 7),
            )
            .unwrap();

        let candidate = Candidate {
            path: PathBuf::from("/tmp/does-not-matter"),
            size: 10,
            modified_epoch: None,
            sha256: "deadbeef".into(),
            tier: Tier::A,
            format: DetectedFormat::BitcoinCoreBdb,
            signals: vec![],
        };

        let out = stage_extract(&run, std::slice::from_ref(&candidate), &[], true).unwrap();
        assert!(out.is_empty(), "solved digest must be skipped on rerun");

        let out2 = stage_extract(&run, &[candidate], &[], false).unwrap();
        assert_eq!(out2.len(), 1, "skip_solved=false forces a retry");

        std::fs::remove_dir_all(&base).ok();
    }
}
