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

    /// The aliases: the entries assigned another kind as their kind point. Only the table can express them, and they
    /// are the review surface.
    pub fn aliases(&self) -> Vec<&Entry> {
        self.entries
            .iter()
            .filter(|entry| entry.is_alias())
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
    (11155111, "../data/generated/staging/11155111.json")
);
environment_module!(production, "../data/generated/production/commitments.json");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::Metadata;
    use crate::kind;
    use anomapay_erc20_forwarder_bindings::addresses::{Environment, erc20_forwarder_address};

    /// No kind point is authored. An ERC20 entry is assigned its fungibility domain's kind point — the kind of the
    /// active version under the label of the chain's current forwarder — and every alias says so in `alias_of`.
    /// Every other entry is assigned its own kind. This reads the embedded tables only.
    #[test]
    fn erc20_entries_share_their_fungibility_domains_kind_point() {
        for (module, environment, tables) in [
            ("staging", Environment::Staging, staging::tables()),
            ("production", Environment::Production, production::tables()),
        ] {
            for (chain, table) in tables {
                let current = erc20_forwarder_address(environment, chain);
                for entry in &table.entries {
                    let Some(Metadata::Erc20 { token, status, .. }) = &entry.metadata else {
                        assert!(
                            !entry.is_alias(),
                            "{module} {chain}: an entry outside every fungibility domain is not assigned its own kind"
                        );
                        continue;
                    };
                    let current = current.unwrap_or_else(|| {
                        panic!("{module} {chain}: an ERC20 entry but no forwarder recorded")
                    });
                    let active = crate::circuits::erc20_active();
                    let active_label = kind::erc20_label_ref(&current, token);
                    let domain =
                        kind::point(&active.logic_ref, &active_label).expect("a kind derives");
                    assert_eq!(
                        entry.kind_point, domain,
                        "{module} {chain}: an ERC20 entry is not assigned its fungibility domain's kind point"
                    );
                    let listed =
                        crate::circuits::erc20_version(&entry.logic_ref).unwrap_or_else(|| {
                            panic!("{module} {chain}: an ERC20 entry of an unlisted version")
                        });
                    assert_eq!(
                        *status, listed.status,
                        "{module} {chain}: an entry's status differs from its version's"
                    );
                    match entry.metadata.as_ref().and_then(Metadata::alias_of) {
                        None => {
                            assert!(
                                !entry.is_alias(),
                                "{module} {chain}: an entry without alias_of is an alias"
                            );
                            assert_eq!(
                                (entry.logic_ref, entry.label_ref),
                                (active.logic_ref, active_label),
                                "{module} {chain}: only the active version under the current forwarder has no alias_of"
                            );
                        }
                        Some(alias_of) => {
                            assert!(
                                entry.is_alias(),
                                "{module} {chain}: an entry with alias_of is not an alias"
                            );
                            assert_eq!(
                                (&alias_of.version, alias_of.logic_ref, alias_of.label_ref),
                                (&active.version, active.logic_ref, active_label),
                                "{module} {chain}: alias_of does not name the active version under the current forwarder"
                            );
                        }
                    }
                }
            }
        }
    }

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
