//! Pins this repo's commitment hash and kind derivation to the circuit's, through `anoma-rm-risc0`'s public
//! loader. The loader assigns every kind itself as its kind point, so it can express every entry that is not an
//! alias and no alias; the check therefore runs on the non-alias subset of a table, written to a temporary file. The
//! aliases stay beyond it until the upstream schema carries points. The loader is
//! process-global and admits one table per process, which is one more than this test needs.

use alloy_chains::NamedChain;
use anoma_risc0_kind_tables::{Entry, Table, commitment};
use anoma_rm_risc0::constants::{global_kind_table_hash, init_kind_table_from_file};
use std::path::PathBuf;

fn staging_table(chain: NamedChain) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../kind-tables/data/generated/staging")
        .join(format!("{}.json", chain as u64))
}

#[test]
fn the_local_commitment_matches_the_circuit_loader() {
    let table = Table::load(staging_table(NamedChain::BaseSepolia))
        .expect("the staging base-sepolia table exists");
    let derived: Vec<Entry> = table
        .entries
        .iter()
        .filter(|entry| !entry.is_alias())
        .cloned()
        .collect();
    assert!(
        !derived.is_empty(),
        "the table has no non-alias entry to check"
    );

    let path = std::env::temp_dir().join("anoma-risc0-kind-tables-derived-84532.json");
    std::fs::write(&path, serde_json::to_string(&derived).unwrap())
        .expect("the non-alias subset is written");

    init_kind_table_from_file(&path).expect("the upstream loader accepts the non-alias subset");
    assert_eq!(
        global_kind_table_hash().expect("the loader installed the table"),
        &commitment::of(&derived),
    );
}
