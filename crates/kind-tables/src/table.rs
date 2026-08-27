//! Generated chain tables and their commitments, one set per environment. A chain's table lives in
//! `data/generated/<environment>/<chain id>.json`; the tables and commitments are embedded and looked up per chain.

use crate::commitment;
use crate::entry::Entry;
use crate::error::{Error, Result};
use alloy_chains::NamedChain;
use risc0_zkvm::Digest;
use serde::Deserialize;
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

/// One chain's recorded commitment. The `_comment` naming the chain is review context and is not deserialized.
#[derive(Deserialize)]
struct ChainCommitment {
    commitment: String,
}

fn parse_commitments(json: &str) -> BTreeMap<NamedChain, Digest> {
    use hex::FromHex;
    let raw: BTreeMap<u64, ChainCommitment> =
        serde_json::from_str(json).expect("commitments.json: invalid JSON");
    raw.into_iter()
        .map(|(id, recorded)| {
            // A chain that fails to resolve must fail loudly: dropping it would erase its freshness assertion.
            let chain =
                NamedChain::try_from(id).unwrap_or_else(|_| panic!("unknown chain ID: {id}"));
            let digest = Digest::from_hex(&recorded.commitment)
                .unwrap_or_else(|_| panic!("invalid commitment for {chain}"));
            (chain, digest)
        })
        .collect()
}

macro_rules! environment_module {
    ($name:ident, $commitments_path:literal $(, ($id:literal, $table_path:literal))*) => {
        pub mod $name {
            use super::*;
            use std::sync::LazyLock;

            static COMMITMENTS: LazyLock<BTreeMap<NamedChain, Digest>> =
                LazyLock::new(|| parse_commitments(include_str!($commitments_path)));

            static TABLES: LazyLock<BTreeMap<NamedChain, Table>> = LazyLock::new(|| {
                let tables: Vec<(NamedChain, Table)> = vec![$((
                    NamedChain::try_from($id as u64)
                        .unwrap_or_else(|_| panic!("unknown chain ID: {}", $id)),
                    Table::from_json(include_str!($table_path))
                        .unwrap_or_else(|error| panic!("invalid table for chain ID {}: {error}", $id)),
                )),*];
                tables.into_iter().collect()
            });

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

            /// The tables of all recorded chains.
            pub fn tables() -> &'static BTreeMap<NamedChain, Table> {
                &TABLES
            }

            /// The table recorded for the chain.
            pub fn table(chain: NamedChain) -> Result<&'static Table> {
                TABLES.get(&chain).ok_or(Error::UnrecordedChain(chain))
            }
        }
    };
}

environment_module!(
    staging,
    "../data/generated/staging/commitments.json",
    (11155111, "../data/generated/staging/11155111.json"),
    (84532, "../data/generated/staging/84532.json")
);
environment_module!(production, "../data/generated/production/commitments.json");

#[cfg(test)]
mod tests {
    use super::*;

    /// The macro invocation lists the table files by hand, so pin it to `commitments.json`.
    #[test]
    fn embedded_tables_match_the_recorded_commitments() {
        for (module, commitments, tables) in [
            ("staging", staging::commitments(), staging::tables()),
            (
                "production",
                production::commitments(),
                production::tables(),
            ),
        ] {
            let recorded: Vec<&NamedChain> = commitments.keys().collect();
            let embedded: Vec<&NamedChain> = tables.keys().collect();
            assert_eq!(recorded, embedded, "{module}: chains out of sync");
            for (chain, commitment) in commitments {
                assert_eq!(
                    tables[chain].commitment(),
                    *commitment,
                    "{module}: stale table for {chain}"
                );
            }
        }
    }
}
