# Kaonashi Architecture

Kaonashi is a browser-based encrypted voting application built from a Rust/WASM
frontend, a Rust/Axum API, and an Anchor program on Solana. The diagram shows
the normal batched-voting flow, together with the separate chairperson and
results paths.

```mermaid
flowchart LR
    voter["Voter / Chairperson"]
    browser["Browser<br/>Leptos + Rust/WASM<br/>kaonashi_frontend"]
    api["Axum API :3000<br/>kaonashi-api"]
    state[("API process memory<br/>ElGamal keys, pending votes,<br/>batches, receipts, ballot addresses")]
    solana["Solana RPC :8899"]
    program["Anchor program<br/>projeto-kaonashi"]
    accounts[("On-chain accounts<br/>Ballots, encrypted tallies,<br/>Merkle roots, election state")]

    voter --> browser
    browser <-->|"HTTP / JSON"| api
    api <--> state
    api <-->|"Solana client transactions / account reads"| solana
    solana --> program
    program <--> accounts

    subgraph vote["Voter flow"]
        pick["Choose decade and movie"]
        encrypt["One-hot vote vector<br/>ElGamal ciphertexts"]
        prove["Per-option 0-or-1 proofs<br/>and sum-to-1 proof"]
        sign["Sign encrypted-vote message"]
        accept["API checks signature, hash,<br/>ciphertexts and vote proofs"]
        pending["Hold verified encrypted vote<br/>in API memory"]
        batch["At configured batch size:<br/>homomorphic tally + Merkle tree<br/>+ Groth16 proof"]
        receipt["Store Merkle receipt in API memory<br/>and expose lookup / verification"]
        pick --> encrypt --> prove --> sign --> accept --> pending --> batch --> receipt
    end
    browser -. "prepare and submit vote" .-> pick
    accept -. "valid vote" .-> api
    batch -. "rollup transaction" .-> solana
    receipt -. "receipt shown / checked" .-> browser

    subgraph admin["Chairperson and finalization"]
        adminsign["Sign chairperson action"]
        create["Create decade ballots"]
        close["Close election"]
        flush["Flush pending batch(es)"]
        decrypt["API reads encrypted tally<br/>and decrypts off-chain"]
        tie["Resolve any tie"]
        winner["Set final winner on-chain"]
        adminsign --> create --> close --> flush --> decrypt --> tie --> winner
    end
    browser -. "signed admin requests" .-> adminsign
    create -.-> solana
    close -.-> solana
    flush -.-> solana
    decrypt -. "read tally" .-> solana
    winner -.-> solana

    results["Results page / receipt verification"]
    browser -. "query results and receipts" .-> results
    results -. "API requests" .-> api
```

## Component responsibilities

- **Frontend (`kaonashi_frontend`)**: Leptos pages and browser-side crypto. The
  voter selects a movie, encrypts the one-hot vector with the decade's ElGamal
  public key, generates vote proofs, signs the submission, and posts it to the
  API. It also provides the chairperson workflow and results views.
- **API (`kaonashi-api`)**: Axum routes, request/signature/proof validation,
  temporary election coordination, batching, receipts, and Solana RPC
  operations. It keeps pending votes, generated ElGamal keypairs, receipts, and
  ballot addresses in process memory; restarting the API loses that state.
- **Smart contract (`kaonashi-smart-contract`)**: Anchor ballot and voter
  accounts, election state transitions, encrypted tally updates, and on-chain
  proof verification for submitted rollup batches.
- **Solana local validator / cluster**: Stores the program and ballot state and
  executes the instructions submitted by the API.

The current frontend/API batched path differs from the program's direct
`cast_vote` instruction: the frontend submits encrypted votes to the API, which
validates them and aggregates them off-chain before submitting a rollup batch.
The Groth16 batch circuit proves that the private leaf hashes form the supplied
Merkle root; the current circuit does not prove that the encrypted batch tally
was computed from those leaves.

## Local development endpoints

The API binds to `127.0.0.1:3000`
the Solana client uses local RPC at `127.0.0.1:8899`. 
Frontend assets are built and served with Trunk. 
The frontend API URL is currently hardcoded to `http://127.0.0.1:3000`.
