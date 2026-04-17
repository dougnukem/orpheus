//! Crypto primitives: BIP32 derivation, address derivation, WIF, scrypt+AES.

use std::str::FromStr;

use bitcoin::{
    Address, CompressedPublicKey, Network, PrivateKey,
    address::NetworkUnchecked,
    bip32::{DerivationPath, Xpriv},
    hashes::Hash,
    key::Secp256k1,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("invalid BIP32 derivation path: {0}")]
    Path(String),
    #[error("invalid private key")]
    InvalidPrivkey,
    #[error("invalid WIF: {0}")]
    Wif(String),
    #[error("bip32: {0}")]
    Bip32(String),
}

pub struct Addresses {
    pub p2pkh_compressed: String,
    pub p2pkh_uncompressed: String,
    pub p2sh_p2wpkh: String,
    pub bech32: String,
}

/// Build every canonical address form for a given secp256k1 private key on mainnet.
pub fn addresses_for_privkey(privkey: &[u8; 32]) -> Result<Addresses, CryptoError> {
    let secp = Secp256k1::new();
    let pk_comp = PrivateKey::from_slice(privkey, Network::Bitcoin)
        .map_err(|_| CryptoError::InvalidPrivkey)?;
    let mut pk_uncomp = pk_comp;
    pk_uncomp.compressed = false;

    let pub_comp = pk_comp.public_key(&secp);
    let pub_uncomp = pk_uncomp.public_key(&secp);

    let p2pkh_c = Address::p2pkh(pub_comp, Network::Bitcoin).to_string();
    let p2pkh_u = Address::p2pkh(pub_uncomp, Network::Bitcoin).to_string();

    let comp = CompressedPublicKey::from_private_key(&secp, &pk_comp)
        .map_err(|e| CryptoError::Bip32(e.to_string()))?;
    let p2sh = Address::p2shwpkh(&comp, Network::Bitcoin).to_string();
    let bech32 = Address::p2wpkh(&comp, Network::Bitcoin).to_string();

    Ok(Addresses {
        p2pkh_compressed: p2pkh_c,
        p2pkh_uncompressed: p2pkh_u,
        p2sh_p2wpkh: p2sh,
        bech32,
    })
}

/// Encode a 32-byte private key as mainnet WIF (compressed by default).
pub fn privkey_to_wif(privkey: &[u8; 32], compressed: bool) -> Result<String, CryptoError> {
    let mut pk = PrivateKey::from_slice(privkey, Network::Bitcoin)
        .map_err(|_| CryptoError::InvalidPrivkey)?;
    pk.compressed = compressed;
    Ok(pk.to_wif())
}

/// Decode a WIF back into raw bytes; returns (privkey, compressed).
pub fn wif_to_privkey(wif: &str) -> Result<([u8; 32], bool), CryptoError> {
    let pk = PrivateKey::from_wif(wif).map_err(|e| CryptoError::Wif(e.to_string()))?;
    let bytes: [u8; 32] = pk.inner.secret_bytes();
    Ok((bytes, pk.compressed))
}

/// Derive an `Xpriv` at `path` from a BIP39 seed (64 bytes).
pub fn derive_from_seed(seed: &[u8], path: &str) -> Result<Xpriv, CryptoError> {
    let network = bitcoin::NetworkKind::Main;
    let master = Xpriv::new_master(network, seed).map_err(|e| CryptoError::Bip32(e.to_string()))?;
    let dp = DerivationPath::from_str(path).map_err(|e| CryptoError::Path(e.to_string()))?;
    let secp = Secp256k1::new();
    master
        .derive_priv(&secp, &dp)
        .map_err(|e| CryptoError::Bip32(e.to_string()))
}

/// Validate a parsed mainnet address string, returning its canonical form.
pub fn parse_address(addr: &str) -> Result<String, CryptoError> {
    let parsed: Address<NetworkUnchecked> =
        Address::from_str(addr).map_err(|e| CryptoError::Wif(e.to_string()))?;
    Ok(parsed.assume_checked().to_string())
}

/// AES-256-CBC decrypt with PKCS7-padded output. Returns plaintext on success.
pub fn aes_cbc_decrypt(key: &[u8; 32], iv: &[u8; 16], ciphertext: &[u8]) -> Option<Vec<u8>> {
    use aes::cipher::{BlockDecryptMut, KeyIvInit, block_padding::Pkcs7};
    type Cbc = cbc::Decryptor<aes::Aes256>;
    Cbc::new(key.into(), iv.into())
        .decrypt_padded_vec_mut::<Pkcs7>(ciphertext)
        .ok()
}

/// Scrypt KDF used by MultiBit v3 wallets: N=16384, r=8, p=1, 32-byte output.
pub fn scrypt_aes_key(password: &[u8], salt: &[u8]) -> Result<[u8; 32], CryptoError> {
    let params =
        scrypt::Params::new(14, 8, 1, 32).map_err(|e| CryptoError::Bip32(e.to_string()))?;
    let mut out = [0u8; 32];
    scrypt::scrypt(password, salt, &params, &mut out)
        .map_err(|e| CryptoError::Bip32(e.to_string()))?;
    Ok(out)
}

/// SHA256 followed by RIPEMD160 — used by Bitcoin for address payloads.
pub fn hash160(data: &[u8]) -> [u8; 20] {
    bitcoin::hashes::hash160::Hash::hash(data).to_byte_array()
}

#[cfg(test)]
mod tests {
    use super::*;

    const KNOWN_PRIV_HEX: &str = "0c28fca386c7a227600b2fe50b7cae11ec86d3bf1fbe471be89827e19d72aa1d";
    const KNOWN_WIF_COMPRESSED: &str = "KwdMAjGmerYanjeui5SHS7JkmpZvVipYvB2LJGU1ZxJwYvP98617";
    const KNOWN_WIF_UNCOMPRESSED: &str = "5HueCGU8rMjxEXxiPuD5BDku4MkFqeZyd4dZ1jvhTVqvbTLvyTJ";

    fn priv_bytes() -> [u8; 32] {
        let mut out = [0u8; 32];
        hex::decode_to_slice(KNOWN_PRIV_HEX, &mut out).unwrap();
        out
    }

    #[test]
    fn wif_roundtrip_compressed() {
        let wif = privkey_to_wif(&priv_bytes(), true).unwrap();
        assert_eq!(wif, KNOWN_WIF_COMPRESSED);
        let (bytes, compressed) = wif_to_privkey(&wif).unwrap();
        assert_eq!(bytes, priv_bytes());
        assert!(compressed);
    }

    #[test]
    fn wif_roundtrip_uncompressed() {
        let wif = privkey_to_wif(&priv_bytes(), false).unwrap();
        assert_eq!(wif, KNOWN_WIF_UNCOMPRESSED);
    }

    #[test]
    fn all_addresses_populated() {
        let a = addresses_for_privkey(&priv_bytes()).unwrap();
        assert!(a.p2pkh_compressed.starts_with('1'));
        assert!(a.p2pkh_uncompressed.starts_with('1'));
        assert!(a.p2sh_p2wpkh.starts_with('3'));
        assert!(a.bech32.starts_with("bc1q"));
    }

    #[test]
    fn bip39_abandon_first_address_matches_known_vector() {
        use bip39::{Language, Mnemonic};

        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let mnemonic = Mnemonic::parse_in(Language::English, phrase).unwrap();
        let seed = mnemonic.to_seed("");
        let child = derive_from_seed(&seed, "m/44'/0'/0'/0/0").unwrap();
        let bytes = child.private_key.secret_bytes();
        let a = addresses_for_privkey(&bytes).unwrap();
        assert_eq!(a.p2pkh_compressed, "1LqBGSKuX5yYUonjxT5qGfpUsXKYYWeabA");
    }

    #[test]
    fn breadwallet_path_derives_distinct_keys() {
        use bip39::{Language, Mnemonic};

        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let mnemonic = Mnemonic::parse_in(Language::English, phrase).unwrap();
        let seed = mnemonic.to_seed("");
        let receive = derive_from_seed(&seed, "m/0'/0/0").unwrap();
        let change = derive_from_seed(&seed, "m/0'/1/0").unwrap();
        assert_ne!(
            receive.private_key.secret_bytes(),
            change.private_key.secret_bytes()
        );
    }
}
