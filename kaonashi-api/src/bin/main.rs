use axum::{
    routing::{get, post},
    Router,
};
use kaonashi_api::{handler, keeping_votes::KeepingVotes};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() {
    let keeping_votes = Arc::new(KeepingVotes::new());

    // usar Any para permitir qualquer origem.
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    //create the axum router
    let router = Router::new()
        .route("/api/is_running", get(handler::is_running))
        .route("/api/vote", post(handler::submit_vote))
        .route("/api/movies/{decade_id}", get(handler::get_movies))
        .route("/api/results/{decade_id}", get(handler::get_results))
        .route("/api/winner/{decade_id}", get(handler::get_winner))
        .route("/api/admin/create-ballots", post(handler::create_ballots))
        .route("/api/auth/challenge", post(handler::create_auth_challenge))
        .route("/api/auth/login", post(handler::login_with_signature))
        .route(
            "/api/admin/flush-batch/{decade_id}",
            post(handler::flush_batch),
        )
        .route(
            "/api/election/{decade_id}/elgamal-public-key",
            get(handler::get_elgamal_public_key),
        )
        .route(
            "/api/vote/receipt/{vote_hash}",
            get(handler::get_vote_receipt),
        )
        .route(
            "/api/vote/verify-receipt",
            post(handler::verify_vote_receipt),
        )
        .route(
            "/api/blockchain/ballot/{decade_id}",
            get(handler::get_blockchain_ballot),
        )
        .with_state(keeping_votes) // state in axum means shared data used by handlers
        .layer(cors);

    //define the ip and port listener (TCP)
    let address = "127.0.0.1:3000";
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();

    //axum serve launch the web server
    axum::serve(listener, router).await.unwrap(); //await pausa um assunc fn até outra (exemplo : pedido http) terminar
}
