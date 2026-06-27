use futures::future::join_all;
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::api::client::{get_results, resolve_tie};

#[derive(Clone, Copy)]
struct Movie {
    index: u8,
    title: &'static str,
    director: &'static str,
    poster: &'static str,
}

fn movies_for_decade(decade_id: u8) -> Vec<Movie> {
    match decade_id {
        0 => vec![
            Movie {
                index: 0,
                title: "Taxi Driver",
                director: "Martin Scorsese",
                poster: "/public/posters/1.jpg",
            },
            Movie {
                index: 1,
                title: "The Godfather",
                director: "Francis Ford Coppola",
                poster: "/public/posters/2.jpg",
            },
            Movie {
                index: 2,
                title: "Apocalypse Now",
                director: "Francis Ford Coppola",
                poster: "/public/posters/3.jpg",
            },
            Movie {
                index: 3,
                title: "Barry Lyndon",
                director: "Stanley Kubrick",
                poster: "/public/posters/4.jpg",
            },
            Movie {
                index: 4,
                title: "Suspiria",
                director: "Dario Argento",
                poster: "/public/posters/5.jpg",
            },
            Movie {
                index: 5,
                title: "Eraserhead",
                director: "David Lynch",
                poster: "/public/posters/6.jpg",
            },
            Movie {
                index: 6,
                title: "A Clockwork Orange",
                director: "Stanley Kubrick",
                poster: "/public/posters/7.jpg",
            },
            Movie {
                index: 7,
                title: "Rocky",
                director: "John G. Avildsen",
                poster: "/public/posters/8.jpg",
            },
        ],
        1 => vec![
            Movie {
                index: 0,
                title: "The Shining",
                director: "Stanley Kubrick",
                poster: "/public/posters/9.jpg",
            },
            Movie {
                index: 1,
                title: "Scarface",
                director: "Brian De Palma",
                poster: "/public/posters/10.jpg",
            },
            Movie {
                index: 2,
                title: "Blue Velvet",
                director: "David Lynch",
                poster: "/public/posters/11.jpg",
            },
            Movie {
                index: 3,
                title: "Paris, Texas",
                director: "Wim Wenders",
                poster: "/public/posters/12.jpg",
            },
            Movie {
                index: 4,
                title: "Cinema Paradiso",
                director: "Giuseppe Tornatore",
                poster: "/public/posters/13.jpg",
            },
            Movie {
                index: 5,
                title: "After Hours",
                director: "Martin Scorsese",
                poster: "/public/posters/14.jpg",
            },
            Movie {
                index: 6,
                title: "Grave of the Fireflies",
                director: "Isao Takahata",
                poster: "/public/posters/15.jpg",
            },
            Movie {
                index: 7,
                title: "Blade Runner",
                director: "Ridley Scott",
                poster: "/public/posters/16.jpg",
            },
        ],
        2 => vec![
            Movie {
                index: 0,
                title: "Fight Club",
                director: "David Fincher",
                poster: "/public/posters/17.jpg",
            },
            Movie {
                index: 1,
                title: "Pulp Fiction",
                director: "Quentin Tarantino",
                poster: "/public/posters/18.jpg",
            },
            Movie {
                index: 2,
                title: "Se7en",
                director: "David Fincher",
                poster: "/public/posters/19.jpg",
            },
            Movie {
                index: 3,
                title: "Goodfellas",
                director: "Martin Scorsese",
                poster: "/public/posters/20.jpg",
            },
            Movie {
                index: 4,
                title: "Eyes Wide Shut",
                director: "Stanley Kubrick",
                poster: "/public/posters/21.jpg",
            },
            Movie {
                index: 5,
                title: "Casino",
                director: "Martin Scorsese",
                poster: "/public/posters/22.jpg",
            },
            Movie {
                index: 6,
                title: "Fallen Angels",
                director: "Wong Kar-wai",
                poster: "/public/posters/23.jpg",
            },
            Movie {
                index: 7,
                title: "Reservoir Dogs",
                director: "Quentin Tarantino",
                poster: "/public/posters/24.jpg",
            },
        ],
        3 => vec![
            Movie {
                index: 0,
                title: "There Will Be Blood",
                director: "Paul Thomas Anderson",
                poster: "/public/posters/25.jpg",
            },
            Movie {
                index: 1,
                title: "Inglourious Basterds",
                director: "Quentin Tarantino",
                poster: "/public/posters/26.jpg",
            },
            Movie {
                index: 2,
                title: "Mulholland Drive",
                director: "David Lynch",
                poster: "/public/posters/27.jpg",
            },
            Movie {
                index: 3,
                title: "American Psycho",
                director: "Mary Harron",
                poster: "/public/posters/28.jpg",
            },
            Movie {
                index: 4,
                title: "The Dark Knight",
                director: "Christopher Nolan",
                poster: "/public/posters/29.jpg",
            },
            Movie {
                index: 5,
                title: "In the Mood for Love",
                director: "Wong Kar-wai",
                poster: "/public/posters/30.jpg",
            },
            Movie {
                index: 6,
                title: "Oldboy",
                director: "Park Chan-wook",
                poster: "/public/posters/31.jpg",
            },
            Movie {
                index: 7,
                title: "Howl's Moving Castle",
                director: "Hayao Miyazaki",
                poster: "/public/posters/32.jpg",
            },
        ],
        4 => vec![
            Movie {
                index: 0,
                title: "Whiplash",
                director: "Damien Chazelle",
                poster: "/public/posters/33.jpg",
            },
            Movie {
                index: 1,
                title: "Interstellar",
                director: "Christopher Nolan",
                poster: "/public/posters/34.jpg",
            },
            Movie {
                index: 2,
                title: "Once Upon a Time in Hollywood",
                director: "Quentin Tarantino",
                poster: "/public/posters/35.jpg",
            },
            Movie {
                index: 3,
                title: "Shutter Island",
                director: "Martin Scorsese",
                poster: "/public/posters/36.jpg",
            },
            Movie {
                index: 4,
                title: "Django Unchained",
                director: "Quentin Tarantino",
                poster: "/public/posters/37.jpg",
            },
            Movie {
                index: 5,
                title: "The Hateful Eight",
                director: "Quentin Tarantino",
                poster: "/public/posters/38.jpg",
            },
            Movie {
                index: 6,
                title: "Phantom Thread",
                director: "Paul Thomas Anderson",
                poster: "/public/posters/39.jpg",
            },
            Movie {
                index: 7,
                title: "Lucky",
                director: "John Carroll Lynch",
                poster: "/public/posters/40.jpg",
            },
        ],
        _ => vec![
            Movie {
                index: 0,
                title: "Marty Supreme",
                director: "Josh Safdie",
                poster: "/public/posters/41.jpg",
            },
            Movie {
                index: 1,
                title: "Oppenheimer",
                director: "Christopher Nolan",
                poster: "/public/posters/42.jpg",
            },
            Movie {
                index: 2,
                title: "Perfect Days",
                director: "Wim Wenders",
                poster: "/public/posters/43.jpg",
            },
            Movie {
                index: 3,
                title: "The Holdovers",
                director: "Alexander Payne",
                poster: "/public/posters/44.jpg",
            },
            Movie {
                index: 4,
                title: "Anora",
                director: "Sean Baker",
                poster: "/public/posters/45.jpg",
            },
            Movie {
                index: 5,
                title: "Hamnet",
                director: "Chloé Zhao",
                poster: "/public/posters/46.jpg",
            },
            Movie {
                index: 6,
                title: "Dune: Part Two",
                director: "Denis Villeneuve",
                poster: "/public/posters/47.jpg",
            },
            Movie {
                index: 7,
                title: "Poor Things",
                director: "Yorgos Lanthimos",
                poster: "/public/posters/48.jpg",
            },
        ],
    }
}

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

#[component]
pub fn TieResolutionPage(
    page: RwSignal<&'static str>,
    selected_decade: RwSignal<u8>,
    wallet_id: RwSignal<Option<String>>,
    wallet_address: RwSignal<Option<String>>,
) -> impl IntoView {
    let current_decade = RwSignal::new(selected_decade.get_untracked());
    let selected_movie = RwSignal::new(None::<usize>);
    let tie_indices = RwSignal::new(Vec::<usize>::new());
    let unresolved_decades = RwSignal::new(Vec::<u8>::new());
    let loading = RwSignal::new(true);
    let submitting = RwSignal::new(false);
    let message = RwSignal::new(None::<String>);
    let error = RwSignal::new(None::<String>);

    let load_unresolved_ties = move || {
        loading.set(true);
        error.set(None);

        spawn_local(async move {
            let futures: Vec<_> = (0_u8..6_u8).map(|id| get_results(id)).collect();
            let all_results = join_all(futures).await;

            let mut unresolved = Vec::new();
            let mut tie_map: std::collections::HashMap<u8, Vec<usize>> = Default::default();

            for (decade_id, result) in (0_u8..6_u8).zip(all_results) {
                match result {
                    Ok(results) => {
                        if results.final_winner_index.is_none() && results.tie_indices.len() >= 2 {
                            unresolved.push(decade_id);
                            tie_map.insert(decade_id, results.tie_indices);
                        }
                    }
                    Err(api_error) => {
                        error.set(Some(api_error));
                        loading.set(false);
                        return;
                    }
                }
            }

            unresolved_decades.set(unresolved.clone());

            if unresolved.is_empty() {
                tie_indices.set(Vec::new());
                message.set(Some("There are no unresolved ties.".to_string()));
                loading.set(false);
                return;
            }

            let requested = current_decade.get_untracked();
            let decade_id = if unresolved.contains(&requested) {
                requested
            } else {
                unresolved[0]
            };

            current_decade.set(decade_id);
            selected_decade.set(decade_id);
            selected_movie.set(None);

            if let Some(indices) = tie_map.remove(&decade_id) {
                tie_indices.set(indices);
            }

            loading.set(false);
        });
    };

    Effect::new(move |_| {
        load_unresolved_ties();
    });

    let load_decade = move |decade_id: u8| {
        loading.set(true);
        selected_movie.set(None);
        message.set(None);
        error.set(None);
        current_decade.set(decade_id);
        selected_decade.set(decade_id);

        spawn_local(async move {
            match get_results(decade_id).await {
                Ok(results) => {
                    tie_indices.set(results.tie_indices);
                }
                Err(api_error) => {
                    error.set(Some(api_error));
                }
            }

            loading.set(false);
        });
    };

    let confirm_winner = move |_| {
        let Some(winner_index) = selected_movie.get() else {
            error.set(Some("Select one of the tied movies.".to_string()));
            return;
        };

        let Some(current_wallet_id) = wallet_id.get() else {
            error.set(Some("Chairperson wallet is missing.".to_string()));
            return;
        };

        let Some(current_public_key) = wallet_address.get() else {
            error.set(Some("Chairperson public key is missing.".to_string()));
            return;
        };

        let decade_id = current_decade.get();

        submitting.set(true);
        message.set(None);
        error.set(None);

        spawn_local(async move {
            match resolve_tie(
                current_wallet_id,
                current_public_key,
                decade_id,
                winner_index,
            )
            .await
            {
                Ok(_) => {
                    let resolved_decade = current_decade.get_untracked();

                    unresolved_decades.update(|decades| {
                        decades.retain(|decade| *decade != resolved_decade);
                    });

                    selected_movie.set(None);
                    tie_indices.set(Vec::new());

                    let remaining = unresolved_decades.get_untracked();

                    if let Some(next_decade) = remaining.first().copied() {
                        current_decade.set(next_decade);
                        selected_decade.set(next_decade);

                        match get_results(next_decade).await {
                            Ok(results) => {
                                tie_indices.set(results.tie_indices);
                            }
                            Err(api_error) => {
                                error.set(Some(api_error));
                            }
                        }
                    } else {
                        message.set(Some("All tied votes have been resolved.".to_string()));
                    }
                }

                Err(api_error) => {
                    error.set(Some(api_error));
                }
            }

            submitting.set(false);
        });
    };

    view! {
        <section class="movies-page">
            <div class="movies-container">
                <header class="movies-header">
                    <button
                        class="back-button"
                        on:click=move |_| page.set("decades")
                    >
                        "← Back to chairperson"
                    </button>

                    <div>
                        <h1>
                            "Resolve tie for the "
                            <span class="decade-number">
                                {move || decade_number(current_decade.get())}
                            </span>
                            <span class="decade-suffix">"s"</span>
                        </h1>

                        <p>
                            "Choose the final winner among the tied movies."
                        </p>
                    </div>
                </header>

                {move || {
                    let decades = unresolved_decades.get();

                    if decades.len() > 1 {
                        view! {
                            <div class="tie-decade-navigation">
                                {decades
                                    .into_iter()
                                    .map(|decade_id| {
                                        view! {
                                            <button
                                                class=move || {
                                                    if current_decade.get() == decade_id {
                                                        "tie-decade-button selected"
                                                    } else {
                                                        "tie-decade-button"
                                                    }
                                                }
                                                disabled=move || submitting.get()
                                                on:click=move |_| load_decade(decade_id)
                                            >
                                                {format!("{}s", decade_number(decade_id))}
                                            </button>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        }
                        .into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }
                }}

                {move || {
                    if loading.get() {
                        view! {
                            <p class="chairperson-empty">"Loading tied movies..."</p>
                        }
                        .into_any()
                    } else {
                        let decade_id = current_decade.get();
                        let movies = movies_for_decade(decade_id);
                        let tied = tie_indices.get();

                        view! {
                            <div class="movie-grid">
                                {movies
                                    .iter()
                                    .copied()
                                    .filter(|movie| tied.contains(&(movie.index as usize)))
                                    .map(|movie| {
                                        let index = movie.index as usize;

                                        view! {
                                            <button
                                                class=move || {
                                                    if selected_movie.get() == Some(index) {
                                                        "movie-card selected"
                                                    } else {
                                                        "movie-card"
                                                    }
                                                }
                                                disabled=move || submitting.get()
                                                on:click=move |_| {
                                                    selected_movie.set(Some(index));
                                                    message.set(None);
                                                    error.set(None);
                                                }
                                            >
                                                <div class="movie-poster-wrapper">
                                                    <img
                                                        class="movie-poster"
                                                        src=movie.poster
                                                        alt=movie.title
                                                    />

                                                    <div class="movie-selection-mark">
                                                        "✓"
                                                    </div>
                                                </div>

                                                <div class="movie-information">
                                                    <h2>{movie.title}</h2>

                                                    <p>
                                                        <span>"Directed by "</span>
                                                        {movie.director}
                                                    </p>
                                                </div>
                                            </button>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        }
                        .into_any()
                    }
                }}

                <button
                    class="submit-vote-btn movie-submit-button"
                    disabled=move || {
                        loading.get()
                            || submitting.get()
                            || selected_movie.get().is_none()
                            || tie_indices.get().len() < 2
                    }
                    on:click=confirm_winner
                >
                    {move || {
                        if submitting.get() {
                            "Confirming winner..."
                        } else {
                            "Confirm final winner"
                        }
                    }}
                </button>

                {move || {
                    message.get().map(|text| {
                        view! { <p class="vote-success">{text}</p> }
                    })
                }}

                {move || {
                    error.get().map(|text| {
                        view! { <p class="vote-error">{text}</p> }
                    })
                }}
            </div>
        </section>
    }
}
