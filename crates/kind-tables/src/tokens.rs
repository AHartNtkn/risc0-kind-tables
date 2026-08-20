//! The supported tokens — the authored identity list the ERC20 entries of every chain table are built from.
//! Being supported is a standing commitment: a chain may list tokens before anything is deployed to it.

use alloy::primitives::Address;
use alloy_chains::NamedChain;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::LazyLock;

/// The identity of a supported ERC20 token: what the contract itself reports.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Token {
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub address: Address,
}

static TOKENS: LazyLock<BTreeMap<NamedChain, Vec<Token>>> = LazyLock::new(|| {
    let raw: BTreeMap<String, Vec<Token>> =
        serde_json::from_str(include_str!("../data/tokens.json"))
            .expect("tokens.json: invalid JSON");
    raw.into_iter()
        .map(|(chain, tokens)| {
            // A chain that fails to resolve must fail loudly: dropping it would silently drop its kinds.
            let chain: NamedChain = chain
                .parse()
                .unwrap_or_else(|_| panic!("unknown chain name: {chain}"));
            (chain, tokens)
        })
        .collect()
});

/// All supported tokens, per chain.
pub fn all() -> &'static BTreeMap<NamedChain, Vec<Token>> {
    &TOKENS
}

/// The supported tokens on the chain.
pub fn on(chain: NamedChain) -> &'static [Token] {
    TOKENS.get(&chain).map_or(&[], Vec::as_slice)
}
