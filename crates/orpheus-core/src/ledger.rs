//! Append-only recovery-attempt ledger.
//!
//! A recovery is rarely one sitting. You find a `wallet.dat` in March, try the
//! four passwords you remember, get nothing, and come back in August with two
//! more. Without a record, the August run repeats the March work and you still
//! cannot answer the only question that matters: *what have I already tried?*
//!
//! The ledger is JSON Lines — one [`Attempt`] per line, appended, never
//! rewritten. That makes it durable against a crash mid-hunt and trivially
//! greppable outside this tool.

use std::collections::HashSet;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// How an extraction attempt ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptOutcome {
    /// Keys came out.
    Success,
    /// The extractor ran cleanly and found nothing.
    NoKeys,
    /// The extractor errored.
    Error,
    /// Encrypted, and no supplied password worked.
    NeedsPassword,
    /// Not attempted — an archive or an unidentified file.
    Skipped,
}

impl AttemptOutcome {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::NoKeys => "no_keys",
            Self::Error => "error",
            Self::NeedsPassword => "needs_password",
            Self::Skipped => "skipped",
        }
    }
}

/// One recorded attempt against one wallet artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attempt {
    /// RFC3339 UTC, for humans reading the file.
    pub ts: String,
    /// Unix epoch seconds, for sorting.
    pub ts_epoch: u64,
    /// SHA-256 of the artifact — the identity that survives being moved.
    pub digest: String,
    pub path: String,
    pub format: String,
    pub extractor: String,
    pub passwords_tried: usize,
    pub outcome: AttemptOutcome,
    pub keys_found: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Attempt {
    /// Start an attempt record stamped with the current wall clock.
    ///
    /// Outcome defaults to [`AttemptOutcome::Skipped`]; chain
    /// [`Attempt::outcome`], [`Attempt::passwords`], and [`Attempt::error`] to
    /// fill in what actually happened. Built this way rather than as one long
    /// constructor so call sites read as sentences.
    #[must_use]
    pub fn now(
        digest: impl Into<String>,
        path: impl Into<String>,
        format: impl Into<String>,
        extractor: impl Into<String>,
    ) -> Self {
        let epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            ts: fmt_rfc3339(epoch),
            ts_epoch: epoch,
            digest: digest.into(),
            path: path.into(),
            format: format.into(),
            extractor: extractor.into(),
            passwords_tried: 0,
            outcome: AttemptOutcome::Skipped,
            keys_found: 0,
            error: None,
        }
    }

    /// Record how the attempt ended and how many keys it produced.
    #[must_use]
    pub fn outcome(mut self, outcome: AttemptOutcome, keys_found: usize) -> Self {
        self.outcome = outcome;
        self.keys_found = keys_found;
        self
    }

    /// Record how many candidate passwords were tried.
    #[must_use]
    pub const fn passwords(mut self, tried: usize) -> Self {
        self.passwords_tried = tried;
        self
    }

    /// Attach an extractor error message.
    #[must_use]
    pub fn error(mut self, error: Option<String>) -> Self {
        self.error = error;
        self
    }
}

/// A JSONL attempt log on disk.
pub struct Ledger {
    path: PathBuf,
}

impl Ledger {
    /// Open (creating the parent directory if needed) a ledger at `path`.
    pub fn open(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(Self { path })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Append one attempt. Opens, writes, and closes so a crash between
    /// attempts cannot lose earlier lines.
    pub fn append(&self, attempt: &Attempt) -> std::io::Result<()> {
        let line = serde_json::to_string(attempt)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(f, "{line}")?;
        Ok(())
    }

    /// Every attempt on record, in write order. Malformed lines are skipped
    /// rather than failing the read — a truncated final line from a killed run
    /// should not make the whole history unreadable.
    pub fn read_all(&self) -> std::io::Result<Vec<Attempt>> {
        if !self.path.exists() {
            return Ok(vec![]);
        }
        let f = std::fs::File::open(&self.path)?;
        Ok(BufReader::new(f)
            .lines()
            .map_while(Result::ok)
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str::<Attempt>(&l).ok())
            .collect())
    }

    /// Digests that have already yielded keys. A rerun can skip these.
    pub fn succeeded_digests(&self) -> std::io::Result<HashSet<String>> {
        Ok(self
            .read_all()?
            .into_iter()
            .filter(|a| a.outcome == AttemptOutcome::Success)
            .map(|a| a.digest)
            .collect())
    }
}

/// Format Unix epoch seconds as RFC3339 UTC.
///
/// Hand-rolled rather than pulling in a date crate for one call site. Uses
/// Howard Hinnant's `civil_from_days`, which is exact for all dates this tool
/// will ever see.
#[must_use]
pub fn fmt_rfc3339(epoch_secs: u64) -> String {
    let days = (epoch_secs / 86_400) as i64;
    let secs_of_day = epoch_secs % 86_400;
    let (y, m, d) = civil_from_days(days);
    let (h, mi, s) = (
        secs_of_day / 3600,
        (secs_of_day % 3600) / 60,
        secs_of_day % 60,
    );
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// Days since 1970-01-01 to (year, month, day). Hinnant's algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let p =
            std::env::temp_dir().join(format!("orpheus-ledger-{tag}-{}.jsonl", std::process::id()));
        std::fs::remove_file(&p).ok();
        p
    }

    fn attempt(digest: &str, outcome: AttemptOutcome, keys: usize) -> Attempt {
        Attempt::now(
            digest,
            "/tmp/wallet.dat",
            "bitcoin_core_bdb",
            "bitcoin_core",
        )
        .passwords(5)
        .outcome(outcome, keys)
    }

    #[test]
    fn builder_defaults_to_skipped_with_no_keys() {
        let a = Attempt::now("d", "/tmp/x", "archive", "-");
        assert_eq!(a.outcome, AttemptOutcome::Skipped);
        assert_eq!(a.keys_found, 0);
        assert_eq!(a.passwords_tried, 0);
        assert!(a.error.is_none());

        let b = a
            .passwords(3)
            .outcome(AttemptOutcome::Error, 0)
            .error(Some("boom".into()));
        assert_eq!(b.passwords_tried, 3);
        assert_eq!(b.outcome, AttemptOutcome::Error);
        assert_eq!(b.error.as_deref(), Some("boom"));
    }

    #[test]
    fn appends_and_reads_back_in_order() {
        let path = tmp("order");
        let ledger = Ledger::open(&path).unwrap();
        ledger
            .append(&attempt("aaa", AttemptOutcome::NeedsPassword, 0))
            .unwrap();
        ledger
            .append(&attempt("bbb", AttemptOutcome::Success, 102))
            .unwrap();

        let all = ledger.read_all().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].digest, "aaa");
        assert_eq!(all[0].outcome, AttemptOutcome::NeedsPassword);
        assert_eq!(all[1].digest, "bbb");
        assert_eq!(all[1].keys_found, 102);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn succeeded_digests_only_lists_successes() {
        let path = tmp("succeeded");
        let ledger = Ledger::open(&path).unwrap();
        ledger
            .append(&attempt("good", AttemptOutcome::Success, 3))
            .unwrap();
        ledger
            .append(&attempt("bad", AttemptOutcome::Error, 0))
            .unwrap();
        ledger
            .append(&attempt("locked", AttemptOutcome::NeedsPassword, 0))
            .unwrap();

        let done = ledger.succeeded_digests().unwrap();
        assert_eq!(done.len(), 1);
        assert!(done.contains("good"));
        assert!(
            !done.contains("locked"),
            "a locked wallet must stay retryable with a new password list"
        );
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn truncated_final_line_does_not_poison_history() {
        let path = tmp("truncated");
        let ledger = Ledger::open(&path).unwrap();
        ledger
            .append(&attempt("aaa", AttemptOutcome::Success, 1))
            .unwrap();
        // Simulate a run killed mid-write.
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        write!(f, "{{\"ts\":\"2026-08-02T00:00").unwrap();
        drop(f);

        let all = ledger.read_all().unwrap();
        assert_eq!(all.len(), 1, "the intact record still reads back");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn missing_ledger_reads_as_empty() {
        let path = tmp("missing");
        let ledger = Ledger::open(&path).unwrap();
        assert!(ledger.read_all().unwrap().is_empty());
    }

    // -- timestamp formatting ------------------------------------------------

    #[test]
    fn rfc3339_epoch_zero() {
        assert_eq!(fmt_rfc3339(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn rfc3339_pinned_known_instants() {
        // 2026-08-02T19:51:51Z — the wall clock of the run this shipped in.
        assert_eq!(fmt_rfc3339(1_785_700_311), "2026-08-02T19:51:51Z");
        // A leap day, to exercise the civil-from-days branch.
        assert_eq!(fmt_rfc3339(1_709_164_800), "2024-02-29T00:00:00Z");
        // Bitcoin genesis block timestamp.
        assert_eq!(fmt_rfc3339(1_231_006_505), "2009-01-03T18:15:05Z");
    }
}
