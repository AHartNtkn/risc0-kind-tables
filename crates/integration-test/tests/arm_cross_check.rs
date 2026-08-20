//! Pins this repo's commitment hash and point derivation to the circuit's, through `anoma-rm-risc0`'s public
//! loader. The loader is a process-global that admits exactly one table per process, which is one more than
//! this test needs; it also derives every point from its key, so the check runs on an alias-free table and an
//! aliased one is skipped until the upstream schema carries points.

use anoma_kind_tables::Table;
use anoma_rm_risc0::constants::{global_kind_table_hash, init_kind_table_from_file};
use std::path::PathBuf;

fn staging_table(chain: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../kind-tables/data/generated/staging")
        .join(format!("{chain}.json"))
}

#[test]
fn the_local_commitment_matches_the_circuit_loader() {
    let path = staging_table("base-sepolia");
    let table = Table::load(&path).expect("the staging base-sepolia table exists");

    if !table.aliases().is_empty() {
        eprintln!(
            "skipped: the table carries aliases, which the upstream loader cannot express yet"
        );
        return;
    }

    init_kind_table_from_file(&path).expect("the upstream loader accepts the generated table");
    assert_eq!(
        global_kind_table_hash().expect("the loader installed the table"),
        &table.commitment(),
    );
}
