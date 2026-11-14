
BenchMark Notes


cd  $PROJect/kaonashi/kaonashi-api

Off-chain benchmarks
Merkle tree construction, proof generation, and proof verification:

cargo bench --bench merkle

The off-chain suite, which also benchmarks Groth16 proof generation and verification:
cargo bench --bench offchain


You can run just one group with a Criterion filter:

cargo bench --bench offchain -- merkle
cargo bench --bench offchain -- groth16

The Groth16 benchmark needs the proving-key file at data/groth16/batch10_proving_key.bin. 
It measures proof generation and verification, not trusted setup. 
Expect it to take a while: proof-generation measurements run for about 30 seconds. Results are saved under target/criterion/.

cargo bench --bench merkle -- benchmarks Merkle
cargo bench --bench offchain -- groth16 -- benchmarks Groth16

cargo bench --bench offchain -- "groth16/batch_100"

Note: the current offchain benchmarkoffchain.rs may only enables batch size 10. check this

change batch size - runthe APi with the configured number
cd kaonashi-api
KAONASHI_BATCH_SIZE=2 cargo run --bin main