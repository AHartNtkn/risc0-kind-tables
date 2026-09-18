//! The ERC20 circuit versions, read from `data/circuit-versions.json`. Every listed version is a member of every
//! ERC20 fungibility domain, so its resources are fungible with every other listed version's behind the same
//! forwarder. Exactly one version is active: under the current forwarder's label it keeps its own kind, which is
//! the kind point of every ERC20 fungibility domain, and the backend creates its resources. Every other version is
//! deprecated: an alias of the active one, which the backend consumes and converts. Nothing is removed. A version
//! the protocol adapter refuses stays listed as deprecated, with rows that nothing can use.

use crate::entry::hex_digest;
use risc0_zkvm::Digest;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// Whether a circuit version, or a member, is the one the backend creates resources of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// The version that keeps its own kind under the current forwarder's label. The backend creates its resources.
    Active,
    /// An alias of the active version. The backend consumes its resources and converts them.
    Deprecated,
}

/// One listed version of the ERC20 transfer circuit.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CircuitVersion {
    /// The version of the crate that ships the circuit.
    pub version: String,
    /// The circuit's verifying key, which every resource of this version carries as its logic ref.
    #[serde(with = "hex_digest")]
    pub logic_ref: Digest,
    pub status: Status,
}

#[derive(Deserialize)]
struct File {
    #[serde(rename = "ERC20Resource")]
    erc20: Vec<CircuitVersion>,
}

static ERC20: LazyLock<Vec<CircuitVersion>> = LazyLock::new(|| {
    let file: File = serde_json::from_str(include_str!("../data/circuit-versions.json"))
        .expect("circuit-versions.json: invalid JSON");
    file.erc20
});

/// The listed ERC20 circuit versions, in file order.
pub fn erc20() -> &'static [CircuitVersion] {
    &ERC20
}

/// The active ERC20 circuit version. Its kind under the current forwarder's label is the kind point of every
/// ERC20 fungibility domain. `check_erc20` says why there is exactly one.
pub fn erc20_active() -> &'static CircuitVersion {
    ERC20
        .iter()
        .find(|circuit| circuit.status == Status::Active)
        .expect("circuit-versions.json lists no active ERC20 circuit version")
}

/// The listed version a logic ref belongs to, if any.
pub fn erc20_version(logic_ref: &Digest) -> Option<&'static CircuitVersion> {
    ERC20.iter().find(|circuit| circuit.logic_ref == *logic_ref)
}

/// Checks the list: every version and logic ref once, exactly one active version, and the active version is
/// the highest listed. The last rule rules out making an old version active again, which is never the plan: a
/// broken release is refused at the protocol adapter and replaced by a higher one.
pub fn check_erc20() -> Result<(), String> {
    let versions = erc20();
    let mut seen_versions = std::collections::BTreeSet::new();
    let mut seen_refs = std::collections::BTreeSet::new();
    let mut parsed = Vec::new();
    for circuit in versions {
        if !seen_versions.insert(&circuit.version) {
            return Err(format!("{} is listed twice", circuit.version));
        }
        if !seen_refs.insert(circuit.logic_ref) {
            return Err(format!(
                "the logic ref of {} is listed twice",
                circuit.version
            ));
        }
        let version = semver::Version::parse(&circuit.version)
            .map_err(|error| format!("{} is not a version: {error}", circuit.version))?;
        parsed.push((version, circuit));
    }
    let mut actives = parsed.iter().filter(|(_, c)| c.status == Status::Active);
    let (active_version, active) = actives
        .next()
        .ok_or_else(|| "no version is active".to_string())?;
    if let Some((_, other)) = actives.next() {
        return Err(format!(
            "{} and {} are both active; exactly one version is",
            active.version, other.version
        ));
    }
    if let Some((highest, _)) = parsed.iter().max_by(|a, b| a.0.cmp(&b.0))
        && highest > active_version
    {
        return Err(format!(
            "{} is active, but {highest} is listed and higher; an older version never becomes active again",
            active.version
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_list_passes_its_own_checks() {
        check_erc20().unwrap();
        assert_eq!(erc20_active().status, Status::Active);
    }
}
