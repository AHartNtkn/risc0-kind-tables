//! Regenerates `data/generated/` from the authored inputs (`tokens.json`, `aliases.json`) and the pinned
//! dependencies (circuit IDs, forwarder and protocol adapter deployment records). CI reruns this and fails on
//! any diff, so the committed artifacts always match the pins.

use anoma_kind_tables::{Entry, commitment, kind, tokens};
use anoma_pa_evm_bindings::addresses::{Environment, protocol_adapter_deployments_map};
use anyhow::{Context, Result, bail};
use risc0_zkvm::Digest;
use risc0_zkvm::sha::{Impl, Sha256};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// One aliasing decision: the `alias` key takes the point of the canonical `of` key, making the two kinds one.
#[derive(Deserialize)]
struct Alias {
    #[serde(rename = "_comment", default)]
    comment: Option<String>,
    alias: Key,
    of: Key,
}

#[derive(Deserialize)]
struct Key {
    logic_ref: String,
    label_ref: String,
}

impl Key {
    fn parse(&self) -> Result<(Digest, Digest)> {
        use hex::FromHex;
        Ok((
            Digest::from_hex(&self.logic_ref).context("invalid logic_ref")?,
            Digest::from_hex(&self.label_ref).context("invalid label_ref")?,
        ))
    }
}

fn digest(bytes: &[u8]) -> Digest {
    Digest::try_from(bytes).expect("a circuit ID is 32 bytes")
}

fn sha256(bytes: &[u8]) -> Digest {
    *Impl::hash_bytes(bytes)
}

fn entry(comment: String, logic_ref: Digest, label_ref: Digest) -> Result<Entry> {
    Ok(Entry {
        comment: Some(comment),
        kind_point: kind::point(&logic_ref, &label_ref)?,
        logic_ref,
        label_ref,
    })
}

fn chain_entries(chain: alloy_chains::NamedChain, aliases: &[Alias]) -> Result<Vec<Entry>> {
    let padding_logic = digest(anoma_rm_risc0::constants::PADDING_LOGIC_VK.as_bytes());
    let transfer_logic = digest(transfer_library::TOKEN_TRANSFER_ID.as_bytes());
    let generic_call_logic = digest(anoma_generic_call_library::GENERIC_CALL_ID.as_bytes());

    let mut entries = vec![entry(
        "padding (trivial logic)".into(),
        padding_logic,
        Digest::default(),
    )?];

    if let Some(forwarder) =
        anoma_generic_call_forwarder_bindings::addresses::generic_call_forwarder_address(&chain)
    {
        entries.push(entry(
            format!("generic call via forwarder {forwarder}"),
            generic_call_logic,
            sha256(forwarder.as_slice()),
        )?);
    }

    let supported = tokens::on(chain);
    match anomapay_erc20_forwarder_bindings::addresses::erc20_forwarder_address(&chain) {
        Some(forwarder) => {
            for token in supported {
                entries.push(entry(
                    format!("{} via ERC20 forwarder {forwarder}", token.symbol),
                    transfer_logic,
                    sha256(&[forwarder.as_slice(), token.address.as_slice()].concat()),
                )?);
            }
        }
        None if supported.is_empty() => {}
        None => eprintln!(
            "{chain}: {} supported tokens but no ERC20 forwarder recorded - no token kinds emitted",
            supported.len()
        ),
    }

    for alias in aliases {
        let (logic_ref, label_ref) = alias.alias.parse()?;
        let of = alias.of.parse()?;
        let Some(canonical) = entries
            .iter()
            .find(|e| (e.logic_ref, e.label_ref) == of && e.is_canonical())
        else {
            bail!(
                "{chain}: alias target ({}, {}) is not a canonical entry of this table",
                alias.of.logic_ref,
                alias.of.label_ref
            );
        };
        let point = canonical.kind_point.clone();
        if entries
            .iter()
            .any(|e| (e.logic_ref, e.label_ref) == (logic_ref, label_ref))
        {
            bail!(
                "{chain}: alias key ({}, {}) collides with an existing entry",
                alias.alias.logic_ref,
                alias.alias.label_ref
            );
        }
        entries.push(Entry {
            comment: alias.comment.clone(),
            kind_point: point,
            logic_ref,
            label_ref,
        });
    }

    entries.sort_by_key(Entry::key);
    if entries
        .windows(2)
        .any(|pair| pair[0].key() == pair[1].key())
    {
        bail!("{chain}: duplicate entry key");
    }
    Ok(entries)
}

fn main() -> Result<()> {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    let aliases: BTreeMap<String, Vec<Alias>> =
        serde_json::from_str(&fs::read_to_string(data.join("aliases.json"))?)
            .context("aliases.json")?;

    for (environment, name) in [
        (Environment::Staging, "staging"),
        (Environment::Production, "production"),
    ] {
        let out = data.join("generated").join(name);
        if out.exists() {
            fs::remove_dir_all(&out)?;
        }
        fs::create_dir_all(&out)?;

        // The forwarder records carry no environment sections yet; both environments read the same maps until
        // the forwarder repos gain them.
        let mut chains: Vec<_> = protocol_adapter_deployments_map(environment)
            .into_keys()
            .collect();
        chains.sort();

        let mut commitments = BTreeMap::new();
        for chain in chains {
            let entries = chain_entries(
                chain,
                aliases.get(&chain.to_string()).map_or(&[], Vec::as_slice),
            )?;
            fs::write(
                out.join(format!("{chain}.json")),
                serde_json::to_string_pretty(&entries)? + "\n",
            )?;
            commitments.insert(
                chain.to_string(),
                hex::encode(commitment::of(&entries).as_bytes()),
            );
        }
        fs::write(
            out.join("commitments.json"),
            serde_json::to_string_pretty(&commitments)? + "\n",
        )?;
        println!("{name}: {} chains", commitments.len());
    }
    Ok(())
}
