//! Orpheus axum server — hosts the embedded React UI and JSON API.

use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use anyhow::Result;
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use clap::Parser;
use orpheus_core::{
    WalletScanResult,
    balance::{BalanceProvider, MockProvider, ProviderKind, provider_from_kind},
    extractors::bip39_mnemonic::{DEFAULT_SPECS, derive_bip39},
    extractors::blockchain_com::decode_mnemonic,
    scanner::scan_path,
};
use rust_embed::Embed;
use serde::{Deserialize, Serialize};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

#[derive(Parser)]
#[command(name = "orpheus-server", version)]
struct Args {
    /// Bind address
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    /// Port
    #[arg(long, default_value_t = 3000)]
    port: u16,
    /// Path to mock_balances.json
    #[arg(long)]
    mock_file: Option<PathBuf>,
    /// Path to the demo wallet fixtures directory
    #[arg(long)]
    demo_dir: Option<PathBuf>,
}

#[derive(Clone)]
struct AppState {
    demo_dir: PathBuf,
    mock_file: PathBuf,
}

#[derive(Embed)]
#[folder = "../../apps/web/dist"]
struct Assets;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();
    let workspace_root = workspace_root();
    let state = AppState {
        demo_dir: args
            .demo_dir
            .unwrap_or_else(|| workspace_root.join("fixtures/demo-wallets")),
        mock_file: args
            .mock_file
            .unwrap_or_else(|| workspace_root.join("fixtures/mock_balances.json")),
    };

    let app = Router::new()
        .route("/api/healthz", get(healthz))
        .route("/api/scan", post(api_scan))
        .route("/api/mnemonic", post(api_mnemonic))
        .route("/api/demo", post(api_demo))
        .fallback(get(serve_static))
        .layer(DefaultBodyLimit::max(64 * 1024 * 1024))
        .layer(TraceLayer::new_for_http())
        .with_state(Arc::new(state));

    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;
    if addr.ip().to_string() != "127.0.0.1" {
        warn!(
            "binding to {} exposes Orpheus beyond localhost — only do this inside a trusted network",
            addr
        );
    }
    info!("orpheus-server listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn healthz() -> &'static str {
    "ok"
}

#[derive(Debug, Serialize)]
struct ScanReply {
    results: Vec<WalletScanResult>,
}

async fn api_scan(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<ScanReply>, ApiError> {
    let tmp = tempfile::TempDir::new().map_err(|e| ApiError::internal(format!("tempdir: {e}")))?;
    let mut passwords: Vec<String> = Vec::new();
    let mut provider_name = String::from("none");
    let mut file_count = 0;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(format!("multipart: {e}")))?
    {
        match field.name().unwrap_or("") {
            "wallet" => {
                let filename = field.file_name().unwrap_or("upload.bin").to_string();
                let safe = sanitize_filename(&filename);
                let dest = tmp.path().join(safe);
                let mut f = tokio::fs::File::create(&dest)
                    .await
                    .map_err(|e| ApiError::internal(format!("create: {e}")))?;
                while let Some(chunk) = field
                    .chunk()
                    .await
                    .map_err(|e| ApiError::internal(format!("chunk: {e}")))?
                {
                    use tokio::io::AsyncWriteExt;
                    f.write_all(&chunk)
                        .await
                        .map_err(|e| ApiError::internal(format!("write: {e}")))?;
                }
                file_count += 1;
            }
            "passwords" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| ApiError::bad_request(format!("passwords: {e}")))?;
                passwords = text
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect();
            }
            "provider" => {
                provider_name = field
                    .text()
                    .await
                    .map_err(|e| ApiError::bad_request(format!("provider: {e}")))?;
            }
            _ => {}
        }
    }

    if file_count == 0 {
        return Err(ApiError::bad_request("no files uploaded".into()));
    }

    let provider: Option<Box<dyn BalanceProvider>> = match ProviderKind::parse(&provider_name) {
        Ok(ProviderKind::Mock) => Some(Box::new(MockProvider {
            path: Some(state.mock_file.clone()),
        })),
        Ok(kind) => provider_from_kind(kind, None),
        Err(e) => return Err(ApiError::bad_request(e)),
    };

    let results = tokio::task::spawn_blocking({
        let root = tmp.path().to_path_buf();
        move || scan_path(&root, &passwords, provider.as_deref())
    })
    .await
    .map_err(|e| ApiError::internal(format!("join: {e}")))?;

    Ok(Json(ScanReply { results }))
}

#[derive(Debug, Deserialize)]
struct MnemonicRequest {
    phrase: String,
    #[serde(default = "default_kind")]
    kind: String,
    #[serde(default)]
    passphrase: String,
    #[serde(default = "default_gap_limit")]
    gap_limit: u32,
    #[serde(default)]
    wordlist: Option<String>,
}

fn default_kind() -> String {
    "bip39".into()
}
fn default_gap_limit() -> u32 {
    20
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum MnemonicReply {
    Bip39 {
        keys: Vec<orpheus_core::ExtractedKey>,
    },
    Blockchain {
        decoded: DecodedReply,
    },
}

#[derive(Debug, Serialize)]
struct DecodedReply {
    password: String,
    word_count: usize,
    version: String,
}

async fn api_mnemonic(Json(req): Json<MnemonicRequest>) -> Result<Json<MnemonicReply>, ApiError> {
    if req.phrase.trim().is_empty() {
        return Err(ApiError::bad_request("phrase is required".into()));
    }
    match req.kind.as_str() {
        "bip39" => {
            let keys = derive_bip39(
                req.phrase.trim(),
                &req.passphrase,
                req.gap_limit,
                DEFAULT_SPECS,
                "(mnemonic)",
            )
            .map_err(ApiError::bad_request)?;
            Ok(Json(MnemonicReply::Bip39 { keys }))
        }
        "blockchain" => {
            let path = req.wordlist.ok_or_else(|| {
                ApiError::bad_request("blockchain.com mnemonics require a wordlist path".into())
            })?;
            let text = std::fs::read_to_string(&path)
                .map_err(|e| ApiError::bad_request(format!("wordlist: {e}")))?;
            let words: Vec<String> = text
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();
            let decoded = decode_mnemonic(req.phrase.trim(), &words)
                .map_err(|e| ApiError::bad_request(e.to_string()))?;
            Ok(Json(MnemonicReply::Blockchain {
                decoded: DecodedReply {
                    password: decoded.password,
                    word_count: decoded.word_count,
                    version: decoded.version_guess,
                },
            }))
        }
        other => Err(ApiError::bad_request(format!("unknown kind: {other}"))),
    }
}

async fn api_demo(State(state): State<Arc<AppState>>) -> Result<Json<ScanReply>, ApiError> {
    let provider = MockProvider {
        path: Some(state.mock_file.clone()),
    };
    let dir = state.demo_dir.clone();
    let results = tokio::task::spawn_blocking(move || {
        scan_path(
            &dir,
            &["orpheus-demo".to_string()],
            Some(&provider as &dyn BalanceProvider),
        )
    })
    .await
    .map_err(|e| ApiError::internal(format!("join: {e}")))?;
    Ok(Json(ScanReply { results }))
}

// ---- static assets ----------------------------------------------------------

async fn serve_static(uri: axum::http::Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    if let Some(asset) = Assets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, mime.as_ref())],
            asset.data.into_owned(),
        )
            .into_response()
    } else if let Some(index) = Assets::get("index.html") {
        // SPA fallback
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            index.data.into_owned(),
        )
            .into_response()
    } else {
        (StatusCode::NOT_FOUND, "not found").into_response()
    }
}

fn sanitize_filename(name: &str) -> String {
    name.rsplit('/')
        .next()
        .unwrap_or(name)
        .rsplit('\\')
        .next()
        .unwrap_or(name)
        .chars()
        .filter(|c| !matches!(c, '\0' | '\n' | '\r'))
        .collect::<String>()
}

fn workspace_root() -> PathBuf {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for ancestor in here.ancestors() {
        let cargo = ancestor.join("Cargo.toml");
        if cargo.exists()
            && std::fs::read_to_string(&cargo)
                .map(|t| t.contains("[workspace]"))
                .unwrap_or(false)
        {
            return ancestor.to_path_buf();
        }
    }
    here
}

// ---- errors ----------------------------------------------------------------

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(msg: String) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: msg,
        }
    }
    fn internal(msg: String) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: msg,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(serde_json::json!({ "error": self.message })),
        )
            .into_response()
    }
}
