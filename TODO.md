# TODO

## Read retired forwarders from the forwarder bindings

A forwarder whose tokens moved to the current one must keep its label as a member of every fungibility domain it backed, for the logic refs it accepted, or the resources created behind it cannot convert and leave (R2 in ADR-0008). The record belongs to `anomapay-erc20-forwarder`'s `deployments.json`, next to the current proxy — `{ "address", "version", "logicRefs" }` — verified there against the chain by the fork tests, as the current forwarders already are. The generator's `retired_erc20_forwarders` returns nothing until the bindings expose that record. The Galileo v1 to v2 transition needs it on every chain.

## Carry the forwarder's contract version in `_metadata`

A member's `forwarder` is an address. Its contract version — `version()` on a v1 forwarder, `VERSION()` on v2 — would tell a reader which generation a row belongs to without knowing the addresses. There is no offline source for it today; the retired record above carries it for retired forwarders, and the current forwarder's would need the same in the bindings.

## Make `anoma-rm-risc0` load a point

`init_kind_table_from_file` assigns every kind itself as its kind point and ignores the file's `kind_point`, so a table loaded through it keeps only the active version in each fungibility domain, silently assigns every other member its own kind, and no longer hashes to the commitment the protocol adapter stores. The backend builds entries from this crate directly and is not affected. Local verification in arm (`Transaction::verify`) is, and so is any other consumer that loads a file. The loader should read the kind point when present (ADR-0004).

## Store a copy of the published tables, and test against it

No test catches a deleted entry. Deleting an entry that is not an alias is safe, because the circuit uses the kind itself. Deleting an alias is not: the kind is then its own kind point, outside every fungibility domain, and its resources can no longer convert. Review is the only guard, and `CODEOWNERS` routes it to a named person.

The fix is a second copy of the tables inside the repository — `crates/kind-tables/data/published/<environment>/<chain id>.json`, holding the tables as of the last release. A test reads both folders. For every entry in the stored copy it finds the same kind in the generated table and compares the two kind points. A missing kind fails the test, unless the stored entry was not an alias. It costs one more copy of the tables and one step in `RELEASE_CHECKLIST.md`: copy `data/generated/` to `data/published/` and commit it.
