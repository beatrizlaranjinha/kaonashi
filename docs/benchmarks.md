# Benchmarking Kaonashi

This guide covers the Criterion off-chain benchmarks, the optional localnet
rollup measurements, and the code/setup changes required to add Groth16 batch
sizes beyond the currently supported `10`, `50`, and `100`.

Run API commands from `kaonashi-api/`.

## Benchmark suites

### Standalone Merkle suite

`benches/merkle.rs` benchmarks three operations for leaf counts
`10, 50, 100, 250, 500, 1000, 2500, 5000, 10000`:

- Building the Merkle root.
- Generating a proof for the middle leaf.
- Verifying that proof.

Run it with:

```sh
cargo bench --bench merkle
```

Those nine sizes are independent of Groth16 support. 
Adding a Merkle-only size means adding it to the `sizes` arrays in this benchmark; 
no proving key or on-chain verifier is needed.

### Combined off-chain suite

`benches/offchain.rs` runs Merkle construction, proof-generation, and
proof-verification benchmarks for each value in `BATCH_SIZES`. 

That array currently contains `[10, 50, 100]`. 
It also benchmarks Groth16 proof generation and local verification for those sizes.

```sh
cargo bench --bench offchain
```

Criterion filters can run one group:

```sh
cargo bench --bench offchain -- merkle
cargo bench --bench offchain -- groth16
```

The Groth16 cases load `data/groth16/batch{N}_proving_key.bin` before timing.

The load is excluded from Criterion's proof-generation measurement; the
benchmark separately pre-generates the verification proof before timing local
verification. Trusted setup is not benchmarked. Merkle data, proving keys,
public inputs, and the prepared verifying key are prepared outside the
individual timed iteration where possible.

The overall Criterion configuration uses a 2-second warm-up and 5-second
measurement with 20 samples. Groth16 overrides this to 1-second warm-up,
30-second measurement, and 10 samples per operation because proving is
expensive. Results are written to `target/criterion/`.

### Localnet/on-chain rollup measurements

`src/blockchain.rs` optionally records end-to-end localnet metrics when a batch
is submitted through the API. This is separate from Criterion and includes
transaction submission/confirmation and RPC metadata collection.

1. Start a dedicated local validator, deploy the Anchor program, and start the
   API. Use a fresh validator/ballot for a run; benchmark repetitions mutate
   ballot state.
2. Set `KAONASHI_BATCH_SIZE` to a supported size before starting the API.
   The API caches this setting on first use.
3. Enable measurement and optionally select repetitions:

   ```sh
   export KAONASHI_BATCH_SIZE=10
   export KAONASHI_ONCHAIN_BENCHMARK=1
   export KAONASHI_ONCHAIN_REPETITIONS=1
   ```

4. Provide the test flow's chairperson key variables and run
   `cargo run --bin submit_test_vote -- <decade_id>` from `kaonashi-api/`.
   The utility creates ballots, submits test votes, closes and flushes the
   election, then finalizes it. It requires at least one test wallet per vote
   in the selected batch.
5. Inspect the console output and `data/benchmarks/onchain.csv`. The CSV records
   batch size, run index, send-to-confirmed latency, compute units, fee,
   effective votes/second, compute units/vote, fee/vote, and transaction
   signature. `plot_onchain.py` reads this CSV and writes figures under
   `data/benchmarks/figures/`.

The latency is **send-to-confirmed**, not isolated program execution time.
`KAONASHI_ONCHAIN_REPETITIONS > 1` resubmits the same valid payload and updates
the benchmark ballot each time; use a disposable localnet. The test-vote
utility and program currently also restrict sizes to `10`, `50`, and `100`.

## Adding another Groth16 batch size

For example, to add `25`, treat it as a new fixed circuit shape and update every
size gate before generating its proving/verifying keys:

1. **API supported-size source** — add `25` to
   `SUPPORTED_GROTH16_BATCH_SIZES` and the `proving_key_path` match in
   `src/groth16.rs`. This enables proof generation and runtime batch selection
   (`KAONASHI_BATCH_SIZE`).
2. **Groth16 Criterion cases** — add `25` to `BATCH_SIZES` in
   `benches/offchain.rs`. This causes the Groth16 benchmark to require and load
   the corresponding proving key.
3. **Test-vote utility** — include `25` in the accepted sizes in
   `src/bin/submit_test_vote.rs`; ensure `kaonashi-api/test-wallets/wallets.json`
   contains at least 25 usable test wallets.
4. **Setup generator destination** — add a unique contract module destination
   for `25` in the `verifying_key_path` match in
   `src/bin/generate_groth16_setup.rs`, and declare that module in the Anchor
   program's `src/lib.rs`.
5. **Generate the fixed-shape setup** — from `kaonashi-api/`, run:

   ```sh
   cargo run --bin generate_groth16_setup -- 25
   ```

   This writes `data/groth16/batch25_proving_key.bin` and generates the
   corresponding Rust verifying-key module under
   `kaonashi-smart-contract/programs/projeto-kaonashi/src/`. The command refuses
   to overwrite an existing proving key. Keep the proving key available to the
   API and do not commit it unless the repository's key-management policy
   explicitly requires it. This generator uses `test_rng`; its output is for
   this project's development/benchmark setup, not a production trusted setup.
6. **On-chain size selection** — in
   `kaonashi-smart-contract/programs/projeto-kaonashi/src/rollups.rs`, import
   the new `VERIFYING_KEY`, allow the new batch size in validation, and map that
   size to the generated key. The key must correspond to the exact same
   `BatchCircuit` shape and Groth16 setup used by the API.
7. **Build and verify** — run focused API tests and benchmarks, build the
   Anchor program, and exercise an end-to-end localnet batch at the new size.
   Check on-chain compute-unit use, transaction size/limits, proof-generation
   time, and proving-key memory before treating a larger size as usable.

The circuit shape fixes the number of private Merkle leaves. Thus adding an
entry only to the Criterion `BATCH_SIZES` array is not sufficient: the API,
test utility, setup generator, proving key, contract module, and on-chain key
selection must all agree. Keep the sizes in sync and use a distinct setup/key
for every new fixed batch size.

After adding a supported size, run:

```sh
cargo bench --bench offchain -- groth16
```

Then run the localnet procedure above with `KAONASHI_BATCH_SIZE` set to the new
size. To graph updated Groth16 measurements, update the measured data in
`plot_groth16.py`; that script currently contains fixed, manually entered
batch-size and proof-time arrays.
