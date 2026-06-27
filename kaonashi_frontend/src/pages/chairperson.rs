use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::api::client::{
    close_election, finalize_election, flush_batches, get_election_completion,
    ElectionCompletionResponse,
};

#[component]
pub fn ChairpersonPage(
    page: RwSignal<&'static str>,
    selected_decade: RwSignal<u8>,
    wallet_id: RwSignal<Option<String>>,
    wallet_address: RwSignal<Option<String>>,
) -> impl IntoView {
    let _ = selected_decade;

    let loading_completion = RwSignal::new(true);
    let submitting = RwSignal::new(false);
    let completion = RwSignal::new(None::<ElectionCompletionResponse>);

    // 1: close, 2: batches, 3: ties, 4: finalize, 5: complete
    let current_step = RwSignal::new(1_u8);

    let success_message = RwSignal::new(None::<String>);
    let error_message = RwSignal::new(None::<String>);

    let refresh_completion = move || {
        loading_completion.set(true);

        spawn_local(async move {
            match get_election_completion().await {
                Ok(response) => completion.set(Some(response)),
                Err(error) => error_message.set(Some(error)),
            }

            loading_completion.set(false);
        });
    };

    Effect::new(move |_| refresh_completion());

    let credentials = move || {
        let wallet = wallet_id
            .get()
            .ok_or_else(|| "Chairperson wallet is missing.".to_string())?;

        let public_key = wallet_address
            .get()
            .ok_or_else(|| "Chairperson public key is missing.".to_string())?;

        Ok::<_, String>((wallet, public_key))
    };

    let close_global_election = move |_| {
        let Ok((wallet, public_key)) = credentials() else {
            error_message.set(Some("Chairperson credentials are missing.".to_string()));
            return;
        };

        submitting.set(true);
        success_message.set(None);
        error_message.set(None);

        spawn_local(async move {
            match close_election(wallet, public_key).await {
                Ok(response) => {
                    let closed = response.results.iter().filter(|r| r.success).count();

                    success_message.set(Some(format!(
                        "Election closed across {closed} decade ballots."
                    )));
                    current_step.set(2);
                    refresh_completion();
                }
                Err(error) => error_message.set(Some(error)),
            }

            submitting.set(false);
        });
    };

    let submit_all_batches = move |_| {
        let Ok((wallet, public_key)) = credentials() else {
            error_message.set(Some("Chairperson credentials are missing.".to_string()));
            return;
        };

        submitting.set(true);
        success_message.set(None);
        error_message.set(None);

        spawn_local(async move {
            match flush_batches(wallet, public_key).await {
                Ok(response) => {
                    success_message.set(Some(format!(
                        "{} batch(es) submitted with {} pending vote(s).",
                        response.total_batches, response.total_votes
                    )));
                    current_step.set(3);
                }
                Err(error) => error_message.set(Some(error)),
            }

            submitting.set(false);
        });
    };

    let open_tie_resolution = move |_| {
        success_message.set(None);
        error_message.set(None);

        // Finalization becomes the next step when the chairperson returns.
        current_step.set(4);
        page.set("tie-resolution");
    };

    let finalize_global_election = move |_| {
        let Ok((wallet, public_key)) = credentials() else {
            error_message.set(Some("Chairperson credentials are missing.".to_string()));
            return;
        };

        submitting.set(true);
        success_message.set(None);
        error_message.set(None);

        spawn_local(async move {
            match finalize_election(wallet, public_key).await {
                Ok(response) => {
                    let finalized = response
                        .results
                        .iter()
                        .filter(|result| {
                            result.status == "Finalized" || result.status == "Already finalized"
                        })
                        .count();

                    let ties = response
                        .results
                        .iter()
                        .filter(|result| result.status == "Tie")
                        .count();

                    if ties == 0 {
                        current_step.set(5);
                    }

                    success_message.set(Some(format!(
                        "{finalized} decade ballot(s) finalized. \
                         {ties} tie(s) require resolution."
                    )));
                }
                Err(error) => error_message.set(Some(error)),
            }

            submitting.set(false);
        });
    };

    let step_class = move |step: u8| {
        if current_step.get() == step {
            "chairperson-step active"
        } else if current_step.get() > step {
            "chairperson-step completed"
        } else {
            "chairperson-step locked"
        }
    };

    view! {
        <section class="chairperson-page">
            <div class="chairperson-container">
                <header class="chairperson-header">
                    <p class="chairperson-kicker">"Chairperson"</p>

                    <h1>
                        "Election control"
                    </h1>

                </header>

                <section class="election-progress">
                    {move || {
                        if loading_completion.get() {
                            view! {
                                <p class="election-progress-loading">
                                    "Checking voter completion..."
                                </p>
                            }
                            .into_any()
                        } else if let Some(status) = completion.get() {
                            let percentage = if status.eligible_voters == 0 {
                                0
                            } else {
                                status.completed_voters * 100 / status.eligible_voters
                            };

                            view! {
                                <div>
                                    <div class="election-progress-heading">
                                        <span>"Voting status"</span>
                                        <strong>
                                            {format!(
                                                "{} / {}",
                                                status.completed_voters,
                                                status.eligible_voters
                                            )}
                                        </strong>
                                    </div>

                                    <div class="election-progress-track">
                                        <div
                                            class="election-progress-fill"
                                            style=format!("width: {percentage}%")
                                        ></div>
                                    </div>

                                    {if status.complete {
                                        view! {
                                            <p class="election-ready">
                                                "All voters completed every decade."
                                            </p>
                                        }
                                        .into_any()
                                    } else {
                                        view! {
                                            <details class="incomplete-voters">
                                                <summary>
                                                    {format!(
                                                        "{} voter(s) still incomplete",
                                                        status.incomplete_voters.len()
                                                    )}
                                                </summary>

                                                <div class="incomplete-voters-list">
                                                    {status
                                                        .incomplete_voters
                                                        .into_iter()
                                                        .map(|voter| {
                                                            view! {
                                                                <div class="incomplete-voter-row">
                                                                    <strong>{voter.wallet_id}</strong>
                                                                    <span>
                                                                        {voter
                                                                            .missing_decade_names
                                                                            .join(", ")}
                                                                    </span>
                                                                </div>
                                                            }
                                                        })
                                                        .collect_view()}
                                                </div>
                                            </details>
                                        }
                                        .into_any()
                                    }}
                                </div>
                            }
                            .into_any()
                        } else {
                            view! {
                                <p class="election-progress-loading">
                                    "Completion status unavailable."
                                </p>
                            }
                            .into_any()
                        }
                    }}
                </section>

                <div class="chairperson-steps">
                    <article class=move || step_class(1)>
                        <div class="chairperson-step-number">"01"</div>

                        <div class="chairperson-step-content">

                            <button
                                class="chairperson-step-button"
                                disabled=move || {
                                    submitting.get()
                                        || current_step.get() != 1
                                        || loading_completion.get()
                                        || completion
                                            .get()
                                            .map(|status| !status.complete)
                                            .unwrap_or(true)
                                }
                                on:click=close_global_election
                            >
                                {move || {
                                    if submitting.get() && current_step.get() == 1 {
                                        "Closing..."
                                    } else {
                                        "Close election"
                                    }
                                }}
                            </button>
                        </div>
                    </article>

                    <article class=move || step_class(2)>
                        <div class="chairperson-step-number">"02"</div>

                        <div class="chairperson-step-content">

                            <button
                                class="chairperson-step-button"
                                disabled=move || submitting.get() || current_step.get() != 2
                                on:click=submit_all_batches
                            >
                                {move || {
                                    if submitting.get() && current_step.get() == 2 {
                                        "Submitting..."
                                    } else {
                                        "Submit pending batches"
                                    }
                                }}
                            </button>
                        </div>
                    </article>

                    <article class=move || step_class(3)>
                        <div class="chairperson-step-number">"03"</div>

                        <div class="chairperson-step-content">
                            <button
                                class="chairperson-step-button"
                                disabled=move || submitting.get() || current_step.get() != 3
                                on:click=open_tie_resolution
                            >
                                "Resolve tied votes"
                            </button>
                        </div>
                    </article>

                    <article class=move || step_class(4)>
                        <div class="chairperson-step-number">"04"</div>

                        <div class="chairperson-step-content">
                            <button
                                class="chairperson-step-button"
                                disabled=move || submitting.get() || current_step.get() != 4
                                on:click=finalize_global_election
                            >
                                {move || {
                                    if submitting.get() && current_step.get() == 4 {
                                        "Finalizing..."
                                    } else {
                                        "Finalize election"
                                    }
                                }}
                            </button>
                        </div>
                    </article>
                </div>

                {move || {
                    success_message.get().map(|message| {
                        view! { <p class="vote-success">{message}</p> }
                    })
                }}

                {move || {
                    error_message.get().map(|message| {
                        view! { <p class="vote-error">{message}</p> }
                    })
                }}
            </div>
        </section>
    }
}
