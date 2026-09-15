# The hash is owned here and cross-checked

arm-risc0 keeps its table hashing private behind a process-global, load-once API, which cannot produce the dozen tables the tests need in one process, so this repo implements the commitment hash (SHA-256 over `logic_ref ‖ label_ref ‖ kind_point` in order) and hash-to-curve locally. The tie to the circuit is a permanent test that loads one table through arm-risc0's public `init_kind_table_from_file` path and asserts the local hash equals `kind_table_hash()` — the singleton allows exactly one load per process, which is one more than the test needs.

## Consequences

- The cross-check is not scaffolding: even after arm-risc0 exposes its hashing, it stays as the only assertion tying published commitments to what the circuit computes.
- Upstreaming is a follow-up, not a blocker. The point-carrying JSON schema landed in `anoma-rm-risc0` 2.0.0-rc.5: the loader reads each `kind_point` from the file, so a prover can load a fungibility domain member. A public hash and a non-global table are still open — `init_kind_table_from_entries` takes a list instead of a file, but installs it in the same singleton.
