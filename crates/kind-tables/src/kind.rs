//! The canonical kind point derivation — RFC 9380 hash-to-curve over `logic_ref ‖ label_ref`, exactly as the
//! compliance circuit derives it. The cross-check test against `anoma-rm-risc0` pins this duplication.

use crate::error::{Error, Result};
use k256::Secp256k1;
use k256::elliptic_curve::hash2curve::{ExpandMsgXmd, GroupDigest};
use k256::elliptic_curve::sec1::ToEncodedPoint;
use risc0_zkvm::Digest;
use risc0_zkvm::sha::rust_crypto::Sha256;

/// The RFC 9380 domain separation tag the compliance circuit uses.
const DST: &[u8] = b"QUUX-V01-CS02-with-secp256k1_XMD:SHA-256_SSWU_RO_";

/// Derives the canonical kind point for a key: the uncompressed SEC1 encoding (65 bytes).
pub fn point(logic_ref: &Digest, label_ref: &Digest) -> Result<Vec<u8>> {
    let mut bytes = [0u8; 64];
    bytes[..32].copy_from_slice(logic_ref.as_ref());
    bytes[32..].copy_from_slice(label_ref.as_ref());
    let point =
        Secp256k1::hash_from_bytes::<ExpandMsgXmd<Sha256>>(&[&bytes], &[DST]).map_err(|_| {
            Error::KindDerivationFailed {
                logic_ref: logic_ref.to_string(),
                label_ref: label_ref.to_string(),
            }
        })?;
    Ok(point.to_encoded_point(false).as_bytes().to_vec())
}
