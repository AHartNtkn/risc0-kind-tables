# TODO

## List every token a V1 forwarder wrapped

A chain's table carries a V1 member only for the tokens in `data/tokens.json`. Before the table is installed on a chain that migrates, `tokens.json` must list every token its V1 forwarder wrapped, and the ERC20 forwarder repository's migration must move the same tokens. A token missing from `tokens.json` gets no V1 member, so its V1 resources cannot leave. A token whose balance does not move must not keep its V1 member when the protocol adapter unpauses. Nothing checks either list; @heueristik makes sure both are right.

## Carry the forwarder's contract version in `_metadata`

A member's `forwarder` is an address. Its contract version — `getVersion()` on a V1 forwarder, `VERSION()` on the current one — would tell a reader which generation a row belongs to without knowing the addresses. There is no offline source for it today: neither the V1 record nor the current forwarder's record carries a version.

## Take the table hashing and the loader out of the process-global

`anoma-rm-risc0` 2.0.0-rc.5 reads `kind_point` from the file and checks it, so a table with aliases loads and hashes to the commitment the protocol adapter stores. That was the blocking piece and it is done: `arm_cross_check` loads a full staging table through `init_kind_table_from_file` and matches the local commitment.

Two upstream follow-ups remain, and neither blocks anything here. The hashing stays private — `hash_kind_table_entries` is not public, and `kind_table_hash` reports only the loaded table — which is why this repo keeps `commitment::of` (ADR-0004). The table stays a process-global that admits one load per process; `init_kind_table_from_entries` builds it from entries instead of a file but installs it in the same place, so a test that needs two tables in one process still cannot have them.

## Decide what a listed circuit version names

`circuit-versions.json` records one `version` per circuit, and the generator requires the pinned crate's version to be listed with the logic ref that crate compiles to. The crate version and the circuit move apart: `transfer_library` went from 2.0.0 to 3.0.0-rc.2 with the same logic ref `bc1232…`, because the release kept the circuit binaries. The list holds one entry per logic ref, so the only way through was to rewrite the version of the entry that was there. Every generated row of that circuit then changed its `_metadata.version`, while its kind point, and so each chain's commitment, stayed as it was.

Four questions follow. They need a call with Xuyang, who cuts the circuit releases.

- Does `version` name the crate or the circuit? If it names the circuit, the generator must stop matching it against the crate version, and the crate version belongs in `_metadata` alone.
- When several crate versions ship one logic ref, which one is listed? Only the pinned crate can be checked against a compiled logic ref.
- Should a release that keeps the logic ref change the tables at all? Today it does, through `_metadata`, so a reviewer reads a table diff that moves no kind.
- Does a release candidate belong in the list? `3.0.0-rc.2` is listed today, and `3.0.0` would replace it by the same rewrite.

`circuits::check_erc20` compares versions with semver and requires the active one to be the highest. A crate republished at a higher version meets that rule without anything changing in the resource machine.

## Store a copy of the published tables, and test against it

No test catches a deleted entry. Deleting an entry that is not an alias is safe, because the circuit uses the kind itself. Deleting an alias is not: the kind is then its own kind point, outside every fungibility domain, and its resources can no longer convert. Review is the only guard, and `CODEOWNERS` routes it to a named person.

The fix is a second copy of the tables inside the repository — `crates/kind-tables/data/published/<environment>/<chain id>.json`, holding the tables as of the last release. A test reads both folders. For every entry in the stored copy it finds the same kind in the generated table and compares the two kind points. A missing kind fails the test, unless the stored entry was not an alias. It costs one more copy of the tables and one step in `RELEASE_CHECKLIST.md`: copy `data/generated/` to `data/published/` and commit it.
