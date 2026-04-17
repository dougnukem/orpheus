use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use comfy_table::{Cell, CellAlignment, Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL};
use orpheus_core::{
    WalletScanResult,
    balance::{BalanceProvider, MockProvider},
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
        /// Balance provider
        #[arg(long, default_value = "none")]
        provider: String,
        /// Mock balance JSON file (used when provider=mock)
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

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("orpheus=info")),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Scan { path, passwords, provider, mock_file, output } => {
            let passes = load_passwords(passwords.as_deref())?;
            let prov = make_provider(&provider, mock_file)?;
            let results = scan_path(&path, &passes, prov.as_deref());
            render(&results, output);
        }
        Command::Extract { wallet, passwords } => {
            let passes = load_passwords(passwords.as_deref())?;
            let results = scan_path(&wallet, &passes, None);
            render(&results, OutputFormat::Table);
        }
        Command::Mnemonic { phrase, kind, passphrase, gap_limit, wordlist } => {
            match kind.as_str() {
                "bip39" => {
                    let keys = derive_bip39(
                        &phrase,
                        &passphrase,
                        gap_limit,
                        DEFAULT_SPECS,
                        "(mnemonic)",
                    )
                    .map_err(|e| anyhow::anyhow!(e))?;
                    let r = WalletScanResult {
                        source_file: "(mnemonic)".into(),
                        source_type: orpheus_core::SourceType::Bip39,
                        keys,
                        error: None,
                    };
                    render(&[r], OutputFormat::Table);
                }
                "blockchain" => {
                    let path = wordlist
                        .ok_or_else(|| anyhow::anyhow!("blockchain.com requires --wordlist"))?;
                    let words: Vec<String> = std::fs::read_to_string(&path)?
                        .lines()
                        .map(|l| l.trim().to_string())
                        .filter(|l| !l.is_empty())
                        .collect();
                    let decoded = decode_mnemonic(&phrase, &words)
                        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                    println!(
                        "decoded {} ({} words) -> password: {}",
                        decoded.version_guess, decoded.word_count, decoded.password
                    );
                }
                other => anyhow::bail!("unknown mnemonic kind: {other}"),
            }
        }
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
            render(&results, OutputFormat::Table);
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

fn make_provider(
    name: &str,
    mock_file: Option<PathBuf>,
) -> Result<Option<Box<dyn BalanceProvider>>> {
    Ok(match name {
        "none" => None,
        "mock" => Some(Box::new(MockProvider { path: mock_file })),
        other => anyhow::bail!("provider {other} not yet supported"),
    })
}

fn render(results: &[WalletScanResult], fmt: OutputFormat) {
    match fmt {
        OutputFormat::Json => {
            let v = serde_json::to_string_pretty(results).unwrap();
            println!("{v}");
        }
        OutputFormat::Csv => {
            println!("source_file,source_type,path,address,wif,balance_sat");
            for r in results {
                for k in &r.keys {
                    println!(
                        "{},{},{},{},{},{}",
                        r.source_file,
                        r.source_type.as_str(),
                        k.derivation_path.as_deref().unwrap_or(""),
                        k.address_compressed,
                        k.wif,
                        k.balance_sat.unwrap_or(0)
                    );
                }
            }
        }
        OutputFormat::Table => render_table(results),
    }
}

fn render_table(results: &[WalletScanResult]) {
    let total_keys: usize = results.iter().map(|r| r.key_count()).sum();
    let total_sat: u64 = results.iter().map(|r| r.total_balance_sat()).sum();
    println!(
        "\n Orpheus scan: {} wallet(s), {} key(s), total balance {:.8} BTC\n",
        results.len(),
        total_keys,
        total_sat as f64 / 1.0e8,
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
            .set_header(vec!["path", "address", "BTC", "txs", "WIF"]);

        let mut sorted: Vec<_> = r.keys.iter().collect();
        sorted.sort_by_key(|k| std::cmp::Reverse(k.balance_sat.unwrap_or(0)));
        let show_all = sorted.iter().all(|k| k.balance_sat.unwrap_or(0) == 0);
        for k in sorted.iter().take(if show_all { 5 } else { usize::MAX }) {
            if !show_all && k.balance_sat.unwrap_or(0) == 0 {
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
                Cell::new(format!("{:.8}", k.balance_sat.unwrap_or(0) as f64 / 1.0e8))
                    .set_alignment(CellAlignment::Right),
                Cell::new(k.tx_count.unwrap_or(0)).set_alignment(CellAlignment::Right),
                Cell::new(wif),
            ]);
        }
        println!("{table}\n");
    }
}
