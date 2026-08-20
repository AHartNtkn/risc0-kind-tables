//! One row of a kind table. The JSON representation is `anoma-rm-risc0`'s kind table schema extended with the
//! kind point, so the upstream loader reads a generated table unchanged.

use crate::kind;
use risc0_zkvm::Digest;
use serde::{Deserialize, Serialize};

/// A `(logic_ref, label_ref)` key and the kind point it names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Human context for reviewers; not covered by the commitment.
    #[serde(rename = "_comment", default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(with = "hex_digest")]
    pub logic_ref: Digest,
    #[serde(with = "hex_digest")]
    pub label_ref: Digest,
    /// Uncompressed SEC1-encoded point (65 bytes).
    #[serde(with = "hex_bytes")]
    pub kind_point: Vec<u8>,
}

impl Entry {
    /// The ordering key: entries sort by `logic_ref ‖ label_ref`.
    pub fn key(&self) -> [u8; 64] {
        let mut key = [0u8; 64];
        key[..32].copy_from_slice(self.logic_ref.as_ref());
        key[32..].copy_from_slice(self.label_ref.as_ref());
        key
    }

    /// Whether the point is the one the key hashes to. A `false` marks an alias.
    pub fn is_canonical(&self) -> bool {
        kind::point(&self.logic_ref, &self.label_ref).is_ok_and(|point| point == self.kind_point)
    }
}

mod hex_digest {
    use hex::FromHex;
    use risc0_zkvm::Digest;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(digest: &Digest, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&hex::encode(digest.as_bytes()))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Digest, D::Error> {
        let string = String::deserialize(deserializer)?;
        Digest::from_hex(&string).map_err(serde::de::Error::custom)
    }
}

mod hex_bytes {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let string = String::deserialize(deserializer)?;
        hex::decode(&string).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex::FromHex;

    fn entry() -> Entry {
        let logic_ref =
            Digest::from_hex("898f3d23ccad1ec7f07051100973815ce3687870416bc96e823a7aeaa347c367")
                .unwrap();
        let label_ref = Digest::default();
        let kind_point = kind::point(&logic_ref, &label_ref).unwrap();
        Entry {
            comment: None,
            logic_ref,
            label_ref,
            kind_point,
        }
    }

    #[test]
    fn is_canonical_accepts_a_derived_point() {
        assert!(entry().is_canonical());
    }

    #[test]
    fn is_canonical_rejects_an_aliased_point() {
        let mut aliased = entry();
        aliased.label_ref = Digest::from([1u32; 8]);
        assert!(!aliased.is_canonical());
    }

    #[test]
    fn the_json_representation_round_trips() {
        let entry = entry();
        let json = serde_json::to_string(&entry).unwrap();
        assert_eq!(serde_json::from_str::<Entry>(&json).unwrap(), entry);
    }
}
