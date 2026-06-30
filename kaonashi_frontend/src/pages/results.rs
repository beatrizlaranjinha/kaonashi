use futures::future::join_all;
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::api::client::{
    get_results, verify_vote_receipt, ResultsResponse, VerifyReceiptResponse,
};

#[derive(Clone)]
struct LocalReceipt {
    decade_id: u8,
    decade: String,
    movie_index: String,
    movie_title: String,
    vote_hash: String,
}

// ---------------------------------------------------
// Helpers
// ---------------------------------------------------

fn decade_number(decade_id: u8) -> &'static str {
    match decade_id {
        0 => "1970",
        1 => "1980",
        2 => "1990",
        3 => "2000",
        4 => "2010",
        _ => "2020",
    }
}

fn result_status(result: &ResultsResponse) -> String {
    if let Some(final_winner) = result.final_winner.clone() {
        return format!("Final winner: {final_winner}");
    }

    if let Some(winner) = result.winner.clone() {
        return format!("Winner: {winner}");
    }

    if result.tie_indices.len() >= 2 {
        return "Tie pending resolution".to_string();
    }

    if result.total_votes == 0 {
        return "No votes".to_string();
    }

    "Results pending".to_string()
}

fn result_pill_label(result: &ResultsResponse) -> &'static str {
    if result.final_winner.is_some() || result.winner.is_some() {
        "Winner"
    } else if result.tie_indices.len() >= 2 {
        "Tie"
    } else {
        "No votes"
    }
}

fn result_pill_class(result: &ResultsResponse) -> &'static str {
    if result.final_winner.is_some() || result.winner.is_some() {
        "result-pill finalized"
    } else if result.tie_indices.len() >= 2 {
        "result-pill tie"
    } else {
        "result-pill empty"
    }
}

fn parse_local_receipts(public_key: &str) -> Vec<LocalReceipt> {
    let window = leptos::prelude::window();

    let Ok(Some(storage)) = window.local_storage() else {
        return Vec::new();
    };

    let storage_key = format!("kaonashi_receipts:{public_key}");

    let Some(raw_receipts) = storage.get_item(&storage_key).ok().flatten() else {
        return Vec::new();
    };

    raw_receipts
        .lines()
        .filter_map(|line| {
            let parts = line.split('|').collect::<Vec<&str>>();

            if parts.len() != 5 {
                return None;
            }

            let decade_id = parts[0].parse::<u8>().ok()?;

            Some(LocalReceipt {
                decade_id,
                decade: parts[1].to_string(),
                movie_index: parts[2].to_string(),
                movie_title: parts[3].to_string(),
                vote_hash: parts[4].to_string(),
            })
        })
        .collect()
}

// ---------------------------------------------------
// Results page
// ---------------------------------------------------

#[component]
pub fn ResultsPage(page: RwSignal<&'static str>) -> impl IntoView {
    let loading_results = RwSignal::new(true);
    let results_error = RwSignal::new(None::<String>);
    let decade_results = RwSignal::new(Vec::<ResultsResponse>::new());

    let public_key_input = RwSignal::new(String::new());
    let local_receipts = RwSignal::new(Vec::<LocalReceipt>::new());
    let receipt_message = RwSignal::new(None::<String>);

    let receipt_hash_input = RwSignal::new(String::new());
    let verifying_receipt = RwSignal::new(false);
    let verify_error = RwSignal::new(None::<String>);
    let verify_result = RwSignal::new(None::<VerifyReceiptResponse>);

    // ---------------------------------------------------
    // Load final results
    // ---------------------------------------------------

    let load_results = move || {
        loading_results.set(true);
        results_error.set(None);

        spawn_local(async move {
            let futures = (0_u8..6_u8).map(get_results).collect::<Vec<_>>();
            let responses = join_all(futures).await;

            let mut loaded_results = Vec::new();

            for response in responses {
                match response {
                    Ok(result) => loaded_results.push(result),
                    Err(error) => {
                        results_error.set(Some(error));
                        loading_results.set(false);
                        return;
                    }
                }
            }

            decade_results.set(loaded_results);
            loading_results.set(false);
        });
    };

    Effect::new(move |_| {
        load_results();
    });

    // ---------------------------------------------------
    // Local receipts
    // ---------------------------------------------------

    let show_local_receipts = move |_| {
        let public_key = public_key_input.get().trim().to_string();

        if public_key.is_empty() {
            receipt_message.set(Some("Enter your public key first.".to_string()));
            local_receipts.set(Vec::new());
            return;
        }

        let receipts = parse_local_receipts(&public_key);

        if receipts.is_empty() {
            receipt_message.set(Some(
                "No local receipts found for this public key in this browser.".to_string(),
            ));
        } else {
            receipt_message.set(Some(format!("{} local receipt(s) found.", receipts.len())));
        }

        local_receipts.set(receipts);
    };

    // ---------------------------------------------------
    // Verify receipt
    // ---------------------------------------------------

    let verify_receipt = move |_| {
        let vote_hash = receipt_hash_input.get().trim().to_string();

        if vote_hash.is_empty() {
            verify_error.set(Some("Insert a vote hash / receipt code first.".to_string()));
            verify_result.set(None);
            return;
        }

        verifying_receipt.set(true);
        verify_error.set(None);
        verify_result.set(None);

        spawn_local(async move {
            match verify_vote_receipt(vote_hash).await {
                Ok(result) => verify_result.set(Some(result)),
                Err(error) => verify_error.set(Some(error)),
            }

            verifying_receipt.set(false);
        });
    };

    // ---------------------------------------------------
    // View
    // ---------------------------------------------------

    view! {
        <section class="results-page">
            <div class="results-container">
                <header class="movies-header">
                    <button
                        class="back-button"
                        on:click=move |_| page.set("decades")
                    >
                        "← Back"
                    </button>

                    <div>
                        <p class="chairperson-kicker">"Results"</p>
                        <h1>"The final cut"</h1>
                        <p>
                            "See the final winners, recover your local receipt codes, and verify that your encrypted vote made it into the batch."
                        </p>
                    </div>
                </header>

                // ---------------------------------------------------
                // Final winners
                // ---------------------------------------------------

                <section class="results-section">
                    <div class="results-section-header">
                        <h2>"Final winners by decade"</h2>

                        <button
                            class="results-small-button"
                            disabled=move || loading_results.get()
                            on:click=move |_| load_results()
                        >
                            {move || {
                                if loading_results.get() {
                                    "Refreshing..."
                                } else {
                                    "Refresh"
                                }
                            }}
                        </button>
                    </div>

                    {move || {
                        if loading_results.get() {
                            view! {
                                <p class="chairperson-empty">
                                    "Loading final results..."
                                </p>
                            }
                            .into_any()
                        } else if let Some(error) = results_error.get() {
                            view! {
                                <p class="vote-error">
                                    {error}
                                </p>
                            }
                            .into_any()
                        } else {
                            view! {
                                <div class="results-grid">
                                    {decade_results
                                        .get()
                                        .into_iter()
                                        .map(|result| {
                                            let decade_label = format!(
                                                "{}s",
                                                decade_number(result.decade_id)
                                            );

                                            let status = result_status(&result);
                                            let pill_label = result_pill_label(&result);
                                            let pill_class = result_pill_class(&result);

                                            view! {
                                                <article class="result-card">
                                                    <div class="result-card-top">
                                                        <span class="result-decade">
                                                            {decade_label}
                                                        </span>

                                                        <span class=pill_class>
                                                            {pill_label}
                                                        </span>
                                                    </div>

                                                    <h3>{status}</h3>

                                                    <p>
                                                        "Total votes: "
                                                        <strong>{result.total_votes}</strong>
                                                    </p>
                                                </article>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            }
                            .into_any()
                        }
                    }}
                </section>

                // ---------------------------------------------------
                // Local receipts
                // ---------------------------------------------------

                <section class="results-section">
                    <div class="results-section-header">
                        <h2>"Your local voting receipts"</h2>
                    </div>

                    <p class="results-muted">
                        "Enter your public key to see receipt codes saved locally in this browser."
                    </p>

                    <div class="results-form-row">
                        <input
                            class="results-input"
                            type="text"
                            placeholder="Public key"
                            prop:value=move || public_key_input.get()
                            on:input=move |event| {
                                public_key_input.set(event_target_value(&event));
                            }
                        />

                        <button
                            class="submit-vote-btn results-action-button"
                            on:click=show_local_receipts
                        >
                            "Show my receipts"
                        </button>
                    </div>

                    {move || {
                        receipt_message.get().map(|message| {
                            view! {
                                <p class="vote-success">
                                    {message}
                                </p>
                            }
                        })
                    }}

                    {move || {
                        if local_receipts.get().is_empty() {
                            view! { <div></div> }.into_any()
                        } else {
                            view! {
                                <div class="receipt-list">
                                    {local_receipts
                                        .get()
                                        .into_iter()
                                        .map(|receipt| {
                                            view! {
                                                <article class="receipt-card">
                                                    <div class="receipt-card-top">
                                                        <span>{receipt.decade}</span>
                                                        <span>
                                                            "Movie index "
                                                            {receipt.movie_index}
                                                        </span>
                                                    </div>

                                                    <h3>{receipt.movie_title}</h3>

                                                    <p class="results-muted">
                                                        "Receipt for decade id "
                                                        {receipt.decade_id}
                                                    </p>

                                                    <p class="receipt-code">
                                                        {receipt.vote_hash}
                                                    </p>
                                                </article>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            }
                            .into_any()
                        }
                    }}
                </section>

                // ---------------------------------------------------
                // Receipt verification
                // ---------------------------------------------------

                <section class="results-section">
                    <div class="results-section-header">
                        <h2>"Verify your receipt"</h2>
                    </div>

                    <p class="results-muted">
                        "Paste your vote hash / receipt code to verify that your encrypted vote was included in a batch."
                    </p>

                    <div class="results-form-row">
                        <input
                            class="results-input"
                            type="text"
                            placeholder="Vote hash / receipt code"
                            prop:value=move || receipt_hash_input.get()
                            on:input=move |event| {
                                receipt_hash_input.set(event_target_value(&event));
                            }
                        />

                        <button
                            class="submit-vote-btn results-action-button"
                            disabled=move || verifying_receipt.get()
                            on:click=verify_receipt
                        >
                            {move || {
                                if verifying_receipt.get() {
                                    "Verifying..."
                                } else {
                                    "Verify receipt"
                                }
                            }}
                        </button>
                    </div>

                    {move || {
                        verify_error.get().map(|error| {
                            view! {
                                <p class="vote-error">
                                    {error}
                                </p>
                            }
                        })
                    }}

                    {move || {
                        verify_result.get().map(|result| {
                            view! {
                                <div class="receipt-verification-card">
                                    <span class=move || {
                                        if result.verified {
                                            "result-pill finalized"
                                        } else {
                                            "result-pill tie"
                                        }
                                    }>
                                        {if result.verified {
                                            "Verified"
                                        } else {
                                            "Not verified"
                                        }}
                                    </span>

                                    <p>
                                        <strong>"Status: "</strong>
                                        {result.status}
                                    </p>

                                    <p>
                                        <strong>"Batch ID: "</strong>
                                        {result.batch_id}
                                    </p>

                                    <p>
                                        <strong>"Merkle root:"</strong>
                                    </p>

                                    <p class="receipt-code">
                                        {result.merkle_root}
                                    </p>
                                </div>
                            }
                        })
                    }}
                </section>
            </div>
        </section>
    }
}
