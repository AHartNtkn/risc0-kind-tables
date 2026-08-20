//! Generated chain tables and their commitments, one set per environment. A chain's table lives in
//! `data/generated/<environment>/<chain>.json`; the commitments are embedded and looked up per chain.

use crate::commitment;
use crate::entry::Entry;
use crate::error::{Error, Result};
use alloy_chains::NamedChain;
use risc0_zkvm::Digest;
use std::collections::BTreeMap;
use std::path::Path;

/// The ordered entries of one chain's kind table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table {
    pub entries: Vec<Entry>,
}

impl Table {
    /// Parses a table from its JSON representation: an array of entries.
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(Self {
            entries: serde_json::from_str(json)?,
        })
    }

    /// Reads a table from a generated `<chain>.json` file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_json(&std::fs::read_to_string(path)?)
    }

    /// The commitment over the entries in their table order.
    pub fn commitment(&self) -> Digest {
        commitment::of(&self.entries)
    }

    /// Whether the entries are sorted by `logic_ref ‖ label_ref`, as the generator emits them.
    pub fn is_sorted(&self) -> bool {
        self.entries
            .windows(2)
            .all(|pair| pair[0].key() < pair[1].key())
    }

    /// The entries whose point is not the one their key hashes to — the review surface.
    pub fn aliases(&self) -> Vec<&Entry> {
        self.entries
            .iter()
            .filter(|entry| !entry.is_canonical())
            .collect()
    }
}

fn parse_commitments(json: &str) -> BTreeMap<NamedChain, Digest> {
    use hex::FromHex;
    let raw: BTreeMap<String, String> =
        serde_json::from_str(json).expect("commitments.json: invalid JSON");
    raw.into_iter()
        .map(|(chain, digest)| {
            // A chain that fails to resolve must fail loudly: dropping it would erase its freshness assertion.
            let chain: NamedChain = chain
                .parse()
                .unwrap_or_else(|_| panic!("unknown chain name: {chain}"));
            let digest = Digest::from_hex(&digest)
                .unwrap_or_else(|_| panic!("invalid commitment for {chain}"));
            (chain, digest)
        })
        .collect()
}

macro_rules! environment_module {
    ($name:ident, $path:literal) => {
        pub mod $name {
            use super::*;
            use std::sync::LazyLock;

            static COMMITMENTS: LazyLock<BTreeMap<NamedChain, Digest>> =
                LazyLock::new(|| parse_commitments(include_str!($path)));

            /// The chains this environment records a table for.
            pub fn chains() -> Vec<NamedChain> {
                COMMITMENTS.keys().copied().collect()
            }

            /// The commitments of all recorded chains.
            pub fn commitments() -> &'static BTreeMap<NamedChain, Digest> {
                &COMMITMENTS
            }

            /// The commitment recorded for the chain.
            pub fn commitment(chain: NamedChain) -> Result<Digest> {
                COMMITMENTS
                    .get(&chain)
                    .copied()
                    .ok_or(Error::UnrecordedChain(chain))
            }
        }
    };
}

environment_module!(staging, "../data/generated/staging/commitments.json");
environment_module!(production, "../data/generated/production/commitments.json");
