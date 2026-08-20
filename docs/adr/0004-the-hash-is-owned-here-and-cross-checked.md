# The hash is owned here and cross-checked

arm-risc0 keeps its table hashing private behind a process-global, load-once, file-only API, which cannot produce the dozen tables the tests need in one process, so this repo implements the commitment hash (SHA-256 over `logic_ref ‖ label_ref ‖ kind_point` in order) and hash-to-curve locally. The tie to the circuit is a permanent test that loads one table through arm-risc0's public `init_kind_table_from_file` path and asserts the local hash equals `global_kind_table_hash()` — the singleton allows exactly one load per process, which is one more than the test needs.

## Consequences

- The cross-check is not scaffolding: even after arm-risc0 exposes its hashing, it stays as the only assertion tying published commitments to what the circuit computes.
- Upstreaming (public hash, non-global table, point-carrying JSON schema) is a follow-up, not a blocker; the schema change is the one genuinely blocking piece, since no prover can load an alias until the loader stops deriving points from keys.
