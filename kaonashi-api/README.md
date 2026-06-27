# Kaonashi API

Separate Axum API for the Kaonashi frontend.

## Run

```bash
cargo run
```

The API runs at:

```text
http://127.0.0.1:3000
```

## Endpoints

```text
GET  /api/health
GET  /api/ballot
POST /api/vote
GET  /api/results
```

## Example vote

```bash
curl -X POST http://127.0.0.1:3000/api/vote \
  -H "Content-Type: application/json" \
  -d '{"movie_index":1}'
```

This first version keeps votes in memory. Restarting the API clears them.

The next integration step is to enable the `zk-client` path dependency and replace the in-memory queue with encrypted rollup preparation and Solana submission.
