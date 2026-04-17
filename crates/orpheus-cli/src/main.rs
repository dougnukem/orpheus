use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use comfy_table::{Cell, CellAlignment, Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL};
use orpheus_core::{
    ScanSummary, WalletScanResult,
    balance::{BalanceProvider, MockProvider, ProviderKind, provider_from_kind},
    extractors::bip39_mnemonic::{DEFAULT_SPECS, derive_bip39},
    extractors::blockchain_com::decode_mnemonic,
    scanner::scan_path,
};

#[derive(Parser)]
#[command(
    name = "orpheus",
    version,
    about = "Recover lost cryptocurrency from forgotten wallets"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scan a file or directory for wallets
    Scan {
        path: PathBuf,
        /// File containing one candidate password per line
        #[arg(long)]
        passwords: Option<PathBuf>,
        /// Where to look up balances for each recovered address.
        ///
        ///   blockstream  — https://blockstream.info/api (default; public esplora, no API key)
        ///   blockchain   — https://blockchain.info/balance (public, batched up to 20 addrs)
        ///   mock         — offline lookup against --mock-file JSON; used by `orpheus demo`
        ///   none         — skip balance lookup entirely (all balances reported as 0)
        #[arg(
            long,
            value_enum,
            default_value_t = CliProvider::Blockstream,
            env = "ORPHEUS_PROVIDER",
            verbatim_doc_comment,
        )]
        provider: CliProvider,
        /// Mock balance JSON file (used when --provider mock)
        #[arg(long)]
        mock_file: Option<PathBuf>,
        /// Output format
        #[arg(long, default_value = "table")]
        output: OutputFormat,
    },
    /// Extract keys from a single wallet file
    Extract {
        wallet: PathBuf,
        #[arg(long)]
        passwords: Option<PathBuf>,
    },
    /// Derive keys from a mnemonic phrase
    Mnemonic {
        phrase: String,
        #[arg(long, default_value = "bip39")]
        kind: String,
        #[arg(long, default_value = "")]
        passphrase: String,
        #[arg(long, default_value_t = 20)]
        gap_limit: u32,
        #[arg(long)]
        wordlist: Option<PathBuf>,
    },
    /// Run the offline demo against bundled fixtures
    Demo,
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum OutputFormat {
    Table,
    Json,
    Csv,
}

/// Values accepted by `--provider`. Kept in lockstep with
/// [`orpheus_core::balance::ProviderKind`].
#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliProvider {
    /// https://blockstream.info/api (public esplora, no API key)
    Blockstream,
    /// https://blockchain.info/balance (public, batched up to 20 addrs)
    Blockchain,
    /// Offline lookup against --mock-file JSON
    Mock,
    /// Skip balance lookup entirely
    None,
}

impl From<CliProvider> for ProviderKind {
    fn from(p: CliProvider) -> Self {
        match p {
            CliProvider::Blockstream => ProviderKind::Blockstream,
            CliProvider::Blockchain => ProviderKind::BlockchainInfo,
            CliProvider::Mock => ProviderKind::Mock,
            CliProvider::None => ProviderKind::None,
        }
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("orpheus=info")),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Scan {
            path,
            passwords,
            provider,
            mock_file,
            output,
        } => {
            let passes = load_passwords(passwords.as_deref())?;
            let prov = provider_from_kind(provider.into(), mock_file);
            let provider_name = prov.as_deref().map(|p| p.name().to_string());
            if let Some(name) = &provider_name {
                tracing::info!(provider = %name, "balance provider");
            } else {
                tracing::info!("balance lookup disabled (--provider none)");
            }
            let results = scan_path(&path, &passes, prov.as_deref());
            render(&results, output, provider_name.as_deref());
        }
        Command::Extract { wallet, passwords } => {
            let passes = load_passwords(passwords.as_deref())?;
            let results = scan_path(&wallet, &passes, None);
            render(&results, OutputFormat::Table, None);
        }
        Command::Mnemonic {
            phrase,
            kind,
            passphrase,
            gap_limit,
            wordlist,
        } => match kind.as_str() {
            "bip39" => {
                let keys =
                    derive_bip39(&phrase, &passphrase, gap_limit, DEFAULT_SPECS, "(mnemonic)")
                        .map_err(|e| anyhow::anyhow!(e))?;
                let r = WalletScanResult {
                    source_file: "(mnemonic)".into(),
                    source_type: orpheus_core::SourceType::Bip39,
                    keys,
                    error: None,
                };
                render(&[r], OutputFormat::Table, None);
            }
            "blockchain" => {
                let path = wordlist
                    .ok_or_else(|| anyhow::anyhow!("blockchain.com requires --wordlist"))?;
                let words: Vec<String> = std::fs::read_to_string(&path)?
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect();
                let decoded =
                    decode_mnemonic(&phrase, &words).map_err(|e| anyhow::anyhow!(e.to_string()))?;
                println!(
                    "decoded {} ({} words) -> password: {}",
                    decoded.version_guess, decoded.word_count, decoded.password
                );
            }
            other => anyhow::bail!("unknown mnemonic kind: {other}"),
        },
        Command::Demo => {
            let demo_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join("fixtures")
                .join("demo-wallets");
            let mock = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join("fixtures")
                .join("mock_balances.json");
            if !demo_dir.exists() {
                anyhow::bail!(
                    "demo fixtures missing — generate them with `cargo run -p orpheus-demo-fixtures`"
                );
            }
            let provider = MockProvider { path: Some(mock) };
            let results = scan_path(
                &demo_dir,
                &["orpheus-demo".into()],
                Some(&provider as &dyn BalanceProvider),
            );
            render(&results, OutputFormat::Table, Some("mock"));
        }
    }
    Ok(())
}

fn load_passwords(path: Option<&std::path::Path>) -> Result<Vec<String>> {
    let Some(path) = path else { return Ok(vec![]) };
    let text = std::fs::read_to_string(path)?;
    Ok(text
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

fn render(results: &[WalletScanResult], fmt: OutputFormat, provider: Option<&str>) {
    match fmt {
        OutputFormat::Json => {
            let v = serde_json::json!({
                "results": results,
                "summary": ScanSummary::from_results(results, provider),
            });
            println!("{}", serde_json::to_string_pretty(&v).unwrap());
        }
        OutputFormat::Csv => {
            println!(
                "source_file,source_type,path,address,wif,received_sat,sent_sat,balance_sat,tx_count"
            );
            for r in results {
                for k in &r.keys {
                    println!(
                        "{},{},{},{},{},{},{},{},{}",
                        r.source_file,
                        r.source_type.as_str(),
                        k.derivation_path.as_deref().unwrap_or(""),
                        k.address_compressed,
                        k.wif,
                        k.total_received_sat.unwrap_or(0),
                        k.total_sent_sat.unwrap_or(0),
                        k.balance_sat.unwrap_or(0),
                        k.tx_count.unwrap_or(0),
                    );
                }
            }
        }
        OutputFormat::Table => render_table(results, provider),
    }
}

fn sat_to_btc(sat: u64) -> String {
    format!("{:.8}", sat as f64 / 1.0e8)
}

fn render_table(results: &[WalletScanResult], provider: Option<&str>) {
    let summary = ScanSummary::from_results(results, provider);

    println!(
        "\n Orpheus scan: {} wallet(s), {} key(s)\n",
        summary.wallets, summary.total_keys,
    );

    for r in results {
        println!("◆ {}  [{}]", r.source_file, r.source_type.as_str());
        if let Some(err) = &r.error {
            println!("  error: {err}");
            continue;
        }
        if r.keys.is_empty() {
            println!("  (no keys)");
            continue;
        }

        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_header(vec![
                "path", "address", "received", "sent", "balance", "txs", "WIF",
            ]);

        let mut sorted: Vec<_> = r.keys.iter().collect();
        // Primary sort: current balance. Secondary: anything ever received.
        sorted.sort_by_key(|k| {
            std::cmp::Reverse((
                k.balance_sat.unwrap_or(0),
                k.total_received_sat.unwrap_or(0),
            ))
        });
        let show_all = sorted
            .iter()
            .all(|k| k.balance_sat.unwrap_or(0) == 0 && k.total_received_sat.unwrap_or(0) == 0);
        for k in sorted.iter().take(if show_all { 5 } else { usize::MAX }) {
            let seen = k.balance_sat.unwrap_or(0) != 0 || k.total_received_sat.unwrap_or(0) != 0;
            if !show_all && !seen {
                continue;
            }
            let wif = if k.wif.len() > 16 {
                format!("{}…{}", &k.wif[..8], &k.wif[k.wif.len() - 4..])
            } else {
                k.wif.clone()
            };
            table.add_row(vec![
                Cell::new(k.derivation_path.clone().unwrap_or_else(|| "-".into())),
                Cell::new(&k.address_compressed),
                Cell::new(sat_to_btc(k.total_received_sat.unwrap_or(0)))
                    .set_alignment(CellAlignment::Right),
                Cell::new(sat_to_btc(k.total_sent_sat.unwrap_or(0)))
                    .set_alignment(CellAlignment::Right),
                Cell::new(sat_to_btc(k.balance_sat.unwrap_or(0)))
                    .set_alignment(CellAlignment::Right),
                Cell::new(k.tx_count.unwrap_or(0)).set_alignment(CellAlignment::Right),
                Cell::new(wif),
            ]);
        }
        println!("{table}\n");
    }

    print_summary(&summary);
}

fn print_summary(s: &ScanSummary) {
    const RULE: &str = "─────────────────────────────────────────────────────────";
    println!("{RULE}");
    println!("  Orpheus scan summary");
    println!("{RULE}");
    println!("  wallets matched        {}", s.wallets);
    println!("  total keys extracted   {}", s.total_keys);
    println!("  unique addresses       {}", s.unique_addresses);
    println!("    funded (balance > 0)   {}", s.funded_addresses,);
    println!("    spent (history, empty) {}", s.spent_addresses,);
    println!("    unfunded               {}", s.unfunded_addresses,);
    println!(
        "  total received         {} BTC",
        sat_to_btc(s.total_received_sat)
    );
    println!(
        "  total sent             {} BTC",
        sat_to_btc(s.total_sent_sat)
    );
    println!(
        "  current balance        {} BTC",
        sat_to_btc(s.total_balance_sat)
    );
    if !s.by_source_type.is_empty() {
        println!("  by source type:");
        for st in &s.by_source_type {
            println!(
                "    {:14}  {:>3} wallet(s)  {:>5} keys  {} BTC",
                st.source_type.as_str(),
                st.wallets,
                st.keys,
                sat_to_btc(st.balance_sat),
            );
        }
    }
    println!(
        "  balance provider       {}",
        s.provider.as_deref().unwrap_or("none (lookup skipped)"),
    );
    println!("{RULE}");
}
