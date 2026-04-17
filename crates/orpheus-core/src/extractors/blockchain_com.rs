//! blockchain.com legacy mnemonic decoder.
//!
//! Decodes the V2 (1626-word) / V3 (65591-word) mnemonic into the encoded
//! password payload. Caller is responsible for providing the matching word
//! list; the words are too large to bundle for v0.1.

use std::path::Path;

use thiserror::Error;

use crate::{
    extractors::Extractor,
    models::{ExtractedKey, SourceType, WalletScanResult},
};

#[derive(Debug, Error)]
pub enum BlockchainComError {
    #[error("word count must be a multiple of 3, got {0}")]
    BadWordCount(usize),
    #[error("word not in wordlist: {0}")]
    UnknownWord(String),
    #[error("decoded payload is not UTF-8")]
    BadUtf8,
}

pub struct Decoded {
    pub password: String,
    pub word_count: usize,
    pub version_guess: String,
}

pub fn decode_mnemonic(phrase: &str, wordlist: &[String]) -> Result<Decoded, BlockchainComError> {
    let words: Vec<&str> = phrase.split_whitespace().collect();
    if !words.len().is_multiple_of(3) {
        return Err(BlockchainComError::BadWordCount(words.len()));
    }
    let idx = |w: &str| -> Result<usize, BlockchainComError> {
        wordlist
            .iter()
            .position(|x| x.eq_ignore_ascii_case(w))
            .ok_or_else(|| BlockchainComError::UnknownWord(w.to_string()))
    };
    let n = wordlist.len() as u64;
    let mut out = Vec::with_capacity(words.len() / 3 * 4);
    for chunk in words.chunks(3) {
        let a = idx(chunk[0])? as u64;
        let b = idx(chunk[1])? as u64;
        let c = idx(chunk[2])? as u64;
        let x = a + n * ((b + n - a) % n) + n * n * ((c + n - b) % n);
        out.extend_from_slice(&(x as u32).to_be_bytes());
    }
    while out.last() == Some(&0) {
        out.pop();
    }
    let password = String::from_utf8(out).map_err(|_| BlockchainComError::BadUtf8)?;
    Ok(Decoded {
        password,
        word_count: words.len(),
        version_guess: if wordlist.len() == 1626 {
            "V2 (1626 words)".into()
        } else {
            format!("V3+ ({}-word list)", wordlist.len())
        },
    })
}

pub struct BlockchainComExtractor;

impl Extractor for BlockchainComExtractor {
    fn source_type(&self) -> SourceType {
        SourceType::BlockchainCom
    }

    fn can_handle(&self, _: &Path) -> bool {
        false // opt-in via the CLI; never auto-detected
    }

    fn extract(&self, path: &Path, _: &[String]) -> WalletScanResult {
        WalletScanResult {
            source_file: path.display().to_string(),
            source_type: self.source_type(),
            keys: vec![ExtractedKey {
                wif: String::new(),
                address_compressed: String::new(),
                address_uncompressed: None,
                address_p2sh_segwit: None,
                address_bech32: None,
                source_file: path.display().to_string(),
                source_type: SourceType::BlockchainCom,
                derivation_path: None,
                balance_sat: None,
                total_received_sat: None,
                tx_count: None,
                notes: Some("pass this file to the CLI mnemonic subcommand".into()),
            }],
            error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synth_wordlist(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("w{i:05}")).collect()
    }

    fn encode(payload: &[u8], wl: &[String]) -> String {
        let n = wl.len() as u64;
        let mut words = Vec::new();
        for chunk in payload.chunks(4) {
            let mut buf = [0u8; 4];
            buf[..chunk.len()].copy_from_slice(chunk);
            let x = u32::from_be_bytes(buf) as u64;
            let a = x % n;
            let b = (a + (x / n) % n) % n;
            let c = (b + (x / (n * n)) % n) % n;
            words.push(wl[a as usize].clone());
            words.push(wl[b as usize].clone());
            words.push(wl[c as usize].clone());
        }
        words.join(" ")
    }

    #[test]
    fn roundtrip() {
        let wl = synth_wordlist(1626);
        let payload = b"Monkey99";
        let phrase = encode(payload, &wl);
        let decoded = decode_mnemonic(&phrase, &wl).unwrap();
        assert_eq!(decoded.password, "Monkey99");
        assert_eq!(decoded.word_count, 6);
    }

    #[test]
    fn bad_word_count() {
        let wl = synth_wordlist(1626);
        assert!(matches!(
            decode_mnemonic("w00000 w00001", &wl),
            Err(BlockchainComError::BadWordCount(2))
        ));
    }
}
