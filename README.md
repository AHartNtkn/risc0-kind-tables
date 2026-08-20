# Anoma Kind Tables

[![Crates](https://github.com/anoma/kind-tables/actions/workflows/crates.yml/badge.svg)](https://github.com/anoma/kind-tables/actions/workflows/crates.yml)

The kind tables the Anoma protocol adapters are committed to — one per chain per environment — and the supported-token list they are built from. The `anoma-kind-tables` crate computes the commitments the way the compliance circuit does, so one source answers what every protocol adapter must store.

## How it fits together

A kind table maps `(logic_ref, label_ref)` keys to kind points. Its commitment — SHA-256 over the ordered entries — is what a protocol adapter stores via `setKindTableCommitment` and what every compliance proof reproduces. A chain's table is derived: the padding entry from `anoma-rm-risc0`, the generic call entry from the recorded forwarder, and one ERC20 entry per supported token from the recorded ERC20 forwarder. Aliases — entries whose point belongs to another key, the migration path between resource logic versions — are the only authored rows; everything else is machine-checked against its key.

## Layout

```
crates/kind-tables/            the library and the generator
├── data/
│   ├── tokens.json            authored: the supported tokens, per chain
│   ├── aliases.json           authored: the aliasing decisions, per chain
│   └── generated/
│       ├── staging/           <chain>.json tables + commitments.json
│       └── production/
└── src/
crates/integration-test/       on-chain token validation, promotion freshness gate, arm-risc0 cross-check
docs/adr/                      the decisions behind this layout
```

## Workflows

Add a token: edit `data/tokens.json`, run `just generate`, commit both. The token validation test checks the contract reports the recorded identity on every pull request and push.

Add an alias: edit `data/aliases.json`, run `just generate`, review the alias in the generated diff — an alias makes two kinds fungible, so it carries the weight of a mint authorization.

Update the deployed commitments: after a merge into `next`, install each chain's commitment from `data/generated/<environment>/commitments.json` with the protocol adapter repo's `contracts-*-kind-table-*` recipes. The promotion pull request into `staging` or `main` then proves every protocol adapter of that environment stores what this source generates.

## Verifying

```sh
just crates-fmt-check && just crates-build && just crates-lint && just generate-check && just crates-test
```

The tests read `ALCHEMY_API_KEY` from the environment (or `.env`, see `.env-example`).
