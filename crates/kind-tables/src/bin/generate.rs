//! Regenerates `data/generated/` from the authored inputs (`tokens.json`, `circuit-versions.json`) and the
//! pinned dependencies (the padding and generic-call logic refs, the forwarder and protocol adapter deployment
//! records). CI reruns this and fails on any diff, so the committed artifacts always match the pins.

use alloy::primitives::Address;
use alloy_chains::NamedChain;
use anoma_generic_call_forwarder_bindings::addresses::Environment as GenericCallEnvironment;
use anoma_kind_tables::{
    AliasOf, CircuitVersion, Entry, Metadata, Status, circuits, commitment, kind, tokens,
};
use anoma_pa_evm_bindings::addresses::{Environment, protocol_adapter_deployments_map};
use anomapay_erc20_forwarder_bindings::addresses::Environment as Erc20Environment;
use anyhow::{Context, Result, bail, ensure};
use risc0_zkvm::Digest;
use risc0_zkvm::sha::{Impl, Sha256};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

/// One row of `commitments.json`: the chain it belongs to, named for review, and the table commitment.
#[derive(Serialize)]
struct ChainCommitment {
    #[serde(rename = "_comment")]
    comment: String,
    commitment: String,
}

/// A forwarder whose tokens moved to the current one. Its label stays a member of every fungibility domain it backed,
/// so a resource created behind it can still convert and leave: a kind with unspent resources must always have
/// a way out.
struct RetiredForwarder {
    address: Address,
    /// The logic refs it accepted. Only these kinds ever existed under its label.
    logic_refs: Vec<Digest>,
}

/// The retired ERC20 forwarders of a chain. The forwarder repository's deployment record is to carry them,
/// verified there against the chain; until the bindings expose that record, there are none.
fn retired_erc20_forwarders(
    _environment: Erc20Environment,
    _chain: &NamedChain,
) -> Vec<RetiredForwarder> {
    Vec::new()
}

fn digest(bytes: &[u8]) -> Digest {
    Digest::try_from(bytes).expect("a logic ref is 32 bytes")
}

fn sha256(bytes: &[u8]) -> Digest {
    *Impl::hash_bytes(bytes)
}

fn hex(digest: &Digest) -> String {
    hex::encode(digest.as_bytes())
}

/// The protocol adapter's environment is the generator's; each forwarder crate declares its own.
fn erc20_environment(environment: Environment) -> Erc20Environment {
    match environment {
        Environment::Staging => Erc20Environment::Staging,
        Environment::Production => Erc20Environment::Production,
    }
}

fn generic_call_environment(environment: Environment) -> GenericCallEnvironment {
    match environment {
        Environment::Staging => GenericCallEnvironment::Staging,
        Environment::Production => GenericCallEnvironment::Production,
    }
}

/// An entry outside every fungibility domain: it is assigned its own kind.
fn derived(metadata: Metadata, logic_ref: Digest, label_ref: Digest) -> Result<Entry> {
    Ok(Entry {
        metadata: Some(metadata),
        kind_point: kind::point(&logic_ref, &label_ref)?,
        logic_ref,
        label_ref,
    })
}

/// A member of a token's fungibility domain: one circuit version under one forwarder's label, assigned the
/// fungibility domain's kind point. `alias_of` names the kind that point is; the active version's own entry
/// carries none.
fn member(
    circuit: &CircuitVersion,
    token: &tokens::Token,
    forwarder: Address,
    domain_point: &[u8],
    alias_of: Option<AliasOf>,
) -> Entry {
    Entry {
        metadata: Some(Metadata::Erc20 {
            version: circuit.version.clone(),
            name: token.symbol.clone(),
            token: token.address,
            forwarder,
            status: circuit.status,
            alias_of,
        }),
        logic_ref: circuit.logic_ref,
        label_ref: kind::erc20_label_ref(&forwarder, &token.address),
        kind_point: domain_point.to_vec(),
    }
}

/// The circuit versions behind the padding and generic-call logic refs, and of the pinned ERC20 crates.
struct Versions {
    padding: String,
    transfer: String,
    generic_call: String,
}

/// Reads the circuit versions from the resolved dependency graph, so bumping a pin cannot leave a stale
/// version in the metadata.
fn versions() -> Result<Versions> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let output = Command::new(cargo)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--locked",
            "--all-features",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .context("failed to run cargo metadata")?;
    if !output.status.success() {
        bail!(
            "cargo metadata: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let metadata: Value =
        serde_json::from_slice(&output.stdout).context("cargo metadata output")?;

    let version_by_id: BTreeMap<&str, &str> = metadata["packages"]
        .as_array()
        .context("cargo metadata reported no packages")?
        .iter()
        .filter_map(|package| Some((package["id"].as_str()?, package["version"].as_str()?)))
        .collect();

    let node = metadata["resolve"]["nodes"]
        .as_array()
        .context("cargo metadata reported no resolve graph")?
        .iter()
        .find(|node| {
            node["id"]
                .as_str()
                .is_some_and(|id| id.contains("anoma-kind-tables"))
        })
        .context("this package is missing from the resolve graph")?;

    let version_of = |lib: &str| -> Result<String> {
        node["deps"]
            .as_array()
            .context("the resolve node carries no dependencies")?
            .iter()
            .find(|dep| dep["name"].as_str() == Some(lib))
            .and_then(|dep| version_by_id.get(dep["pkg"].as_str()?).copied())
            .map(str::to_string)
            .with_context(|| format!("{lib} is not a resolved dependency"))
    };

    Ok(Versions {
        padding: version_of("anoma_rm_risc0")?,
        transfer: version_of("transfer_library")?,
        generic_call: version_of("anoma_generic_call_library")?,
    })
}

/// The ERC20 circuit crates this generator pins, each with the logic ref it compiles to. A release joins as
/// a renamed dependency and one more line here. A version whose crate is no longer pinned stays listed in
/// `circuit-versions.json` and is checked against nothing.
fn pinned_erc20_circuits(versions: &Versions) -> Vec<(String, Digest)> {
    vec![(
        versions.transfer.clone(),
        digest(transfer_library::TOKEN_TRANSFER_ID.as_bytes()),
    )]
}

/// The list must pass its own checks, and every pinned ERC20 crate must be listed with the logic ref it
/// compiles to. Raising a pin without listing the release stops here, and so does a mistyped logic ref.
fn check_circuit_versions(versions: &Versions) -> Result<()> {
    circuits::check_erc20().map_err(|error| anyhow::anyhow!("circuit-versions.json: {error}"))?;
    for (version, compiled) in pinned_erc20_circuits(versions) {
        let listed = circuits::erc20()
            .iter()
            .find(|circuit| circuit.version == version)
            .with_context(|| {
                format!("an ERC20 circuit crate at {version} is pinned, but circuit-versions.json does not list it")
            })?;
        ensure!(
            listed.logic_ref == compiled,
            "circuit-versions.json records {version} with logic ref {}, but the pinned crate compiles to {}",
            hex(&listed.logic_ref),
            hex(&compiled)
        );
    }
    Ok(())
}

fn chain_entries(
    environment: Environment,
    chain: NamedChain,
    versions: &Versions,
) -> Result<Vec<Entry>> {
    let mut entries = vec![derived(
        Metadata::Padding {
            version: versions.padding.clone(),
        },
        digest(anoma_rm_risc0::constants::PADDING_LOGIC_VK.as_bytes()),
        Digest::default(),
    )?];

    if let Some(forwarder) =
        anoma_generic_call_forwarder_bindings::addresses::generic_call_forwarder_address(
            generic_call_environment(environment),
            &chain,
        )
    {
        entries.push(derived(
            Metadata::GenericCall {
                version: versions.generic_call.clone(),
                forwarder,
            },
            digest(anoma_generic_call_library::GENERIC_CALL_ID.as_bytes()),
            sha256(forwarder.as_slice()),
        )?);
    }

    // One fungibility domain per token: every listed circuit version under the current forwarder's label, and
    // under the label of every forwarder whose tokens moved to the current one, all assigned the kind of the
    // active version under the current forwarder's label. The active version keeps its own kind, so it needs
    // no table to know its kind point; the deprecated versions are what the table is for.
    let supported = tokens::on(chain);
    let erc20_environment = erc20_environment(environment);
    match anomapay_erc20_forwarder_bindings::addresses::erc20_forwarder_address(
        erc20_environment,
        &chain,
    ) {
        Some(current) => {
            let retired = retired_erc20_forwarders(erc20_environment, &chain);
            let active = circuits::erc20_active();
            for token in supported {
                let current_label = kind::erc20_label_ref(&current, &token.address);
                let domain = kind::point(&active.logic_ref, &current_label)?;
                let active_kind = AliasOf {
                    version: active.version.clone(),
                    logic_ref: active.logic_ref,
                    label_ref: current_label,
                };
                for circuit in circuits::erc20() {
                    let alias_of =
                        (circuit.status == Status::Deprecated).then(|| active_kind.clone());
                    entries.push(member(circuit, token, current, &domain, alias_of));
                }
                for forwarder in &retired {
                    for logic_ref in &forwarder.logic_refs {
                        let circuit = circuits::erc20_version(logic_ref).with_context(|| {
                            format!(
                                "{chain}: the retired forwarder {} accepted logic ref {}, which circuit-versions.json does not list",
                                forwarder.address,
                                hex(logic_ref)
                            )
                        })?;
                        entries.push(member(
                            circuit,
                            token,
                            forwarder.address,
                            &domain,
                            Some(active_kind.clone()),
                        ));
                    }
                }
            }
        }
        None if supported.is_empty() => {}
        None => eprintln!(
            "{chain}: {} supported tokens but no ERC20 forwarder recorded - no token kinds emitted",
            supported.len()
        ),
    }

    entries.sort_by_key(Entry::key);
    if entries
        .windows(2)
        .any(|pair| pair[0].key() == pair[1].key())
    {
        bail!("{chain}: duplicate kind");
    }
    Ok(entries)
}

fn main() -> Result<()> {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");

    let versions = versions()?;
    check_circuit_versions(&versions)?;

    // Everything is generated before anything is written: a rejected input must not leave the tree without
    // the tables the crate embeds.
    let mut generated = Vec::new();
    for (environment, name) in [
        (Environment::Staging, "staging"),
        (Environment::Production, "production"),
    ] {
        let mut chains: Vec<_> = protocol_adapter_deployments_map(environment)
            .into_keys()
            .collect();
        chains.sort();

        let mut files = Vec::new();
        let mut commitments = BTreeMap::new();
        for chain in chains {
            let entries = chain_entries(environment, chain, &versions)?;
            commitments.insert(
                chain as u64,
                ChainCommitment {
                    comment: chain.to_string(),
                    commitment: hex::encode(commitment::of(&entries).as_bytes()),
                },
            );
            files.push((
                format!("{}.json", chain as u64),
                serde_json::to_string_pretty(&entries)? + "\n",
            ));
        }
        files.push((
            "commitments.json".to_string(),
            serde_json::to_string_pretty(&commitments)? + "\n",
        ));
        generated.push((name, files, commitments.len()));
    }

    for (name, files, chains) in generated {
        let out = data.join("generated").join(name);
        if out.exists() {
            fs::remove_dir_all(&out)?;
        }
        fs::create_dir_all(&out)?;
        for (file, contents) in files {
            fs::write(out.join(file), contents)?;
        }
        println!("{name}: {chains} chains");
    }
    Ok(())
}
