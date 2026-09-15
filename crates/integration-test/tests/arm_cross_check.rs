//! Pins this repo's commitment hash to the circuit's, through `anoma-rm-risc0`'s public loader. The loader reads
//! every kind point from the file, so an aliased table is checked like any other. It is a process-global that
//! admits one table per process, which is one more than this test needs.

use alloy_chains::NamedChain;
use anoma_risc0_kind_tables::Table;
use anoma_rm_risc0::constants::{init_kind_table_from_file, kind_table_hash};
use std::path::PathBuf;

fn staging_table(chain: NamedChain) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../kind-tables/data/generated/staging")
        .join(format!("{}.json", chain as u64))
}

#[test]
fn the_local_commitment_matches_the_circuit_loader() {
    let path = staging_table(NamedChain::Sepolia);
    let table = Table::load(&path).expect("the staging sepolia table exists");

    init_kind_table_from_file(&path).expect("the upstream loader accepts the generated table");
    assert_eq!(
        kind_table_hash().expect("the loader installed the table"),
        &table.commitment(),
    );
}
