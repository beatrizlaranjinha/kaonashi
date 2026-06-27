use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::api::client::submit_vote;

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
pub fn VotePage(
    page: RwSignal<&'static str>,
    selected_decade: RwSignal<u8>,
    wallet_id: RwSignal<Option<String>>,
    wallet_address: RwSignal<Option<String>>,
) -> impl IntoView {
    let selected_movie = RwSignal::new(None::<u8>);
    let movie_page = RwSignal::new(0_usize);

    // Private key local.
    // Só existe nesta página e é usada apenas no momento de votar.
    let private_key = RwSignal::new(String::new());

    let submitting = RwSignal::new(false);
    let vote_submitted = RwSignal::new(false);
    let vote_message = RwSignal::new(None::<String>);
    let vote_error = RwSignal::new(None::<String>);

    let decade_id = selected_decade.get_untracked();
    let movies = movies_for_decade(decade_id);

    let confirm_vote = move |_| {
        let Some(movie_index) = selected_movie.get() else {
            vote_error.set(Some(
                "Select a movie before submitting your vote.".to_string(),
            ));
            return;
        };

        let Some(current_wallet_id) = wallet_id.get() else {
            vote_error.set(Some("You must connect a wallet before voting.".to_string()));
            return;
        };

        let Some(current_public_key) = wallet_address.get() else {
            vote_error.set(Some("The wallet public key is missing.".to_string()));
            return;
        };

        // A private key é lida apenas agora, no momento do voto.
        let current_private_key = private_key.get().trim().to_string();

        if current_private_key.is_empty() {
            vote_error.set(Some(
                "Enter your private key to sign this vote.".to_string(),
            ));
            return;
        }

        let Some(movie_name) = movies_for_decade(decade_id)
            .into_iter()
            .find(|movie| movie.index == movie_index)
            .map(|movie| movie.title.to_string())
        else {
            vote_error.set(Some(
                "The selected movie is not valid for this decade.".to_string(),
            ));
            return;
        };

        submitting.set(true);
        vote_message.set(None);
        vote_error.set(None);

        spawn_local(async move {
            match submit_vote(
                current_wallet_id,
                current_public_key,
                current_private_key,
                decade_id,
                movie_index as usize,
                movie_name,
            )
            .await
            {
                Ok(response) => {
                    let message = if response.batch_submitted {
                        format!(
                            "Your vote for {} was submitted. The encrypted batch was recorded on Solana.",
                            response.movie
                        )
                    } else {
                        format!(
                            "Your vote for {} was accepted. Pending votes in the batch: {}.",
                            response.movie, response.pending_votes
                        )
                    };

                    vote_message.set(Some(message));
                    vote_submitted.set(true);

                    // Limpa a private key depois de usar.
                    private_key.set(String::new());
                }

                Err(error) => {
                    vote_error.set(Some(error));

                    // Mesmo em erro, limpamos a private key.
                    private_key.set(String::new());
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
                        "← Back to decades"
                    </button>

                    <div>
                        <h1>
                            "Best movie of the "
                            <span class="decade-number">
                                {decade_number(decade_id)}
                            </span>
                            <span class="decade-suffix">
                                "s"
                            </span>
                        </h1>

                        <p>
                            "Choose wisely. Your vote is signed locally before being submitted."
                        </p>
                    </div>
                </header>

                <div class="movie-grid">
                    {move || {
                        let current_page = movie_page.get();
                        let start = current_page * 4;
                        let end = start + 4;

                        movies[start..end]
                            .iter()
                            .copied()
                            .map(|movie| {
                                view! {
                                    <button
                                        class=move || {
                                            if selected_movie.get() == Some(movie.index) {
                                                "movie-card selected"
                                            } else {
                                                "movie-card"
                                            }
                                        }
                                        disabled=move || {
                                            submitting.get()
                                                || vote_submitted.get()
                                        }
                                        on:click=move |_| {
                                            selected_movie.set(Some(movie.index));
                                            vote_message.set(None);
                                            vote_error.set(None);
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
                            .collect_view()
                    }}
                </div>

                <div class="movie-pagination">
                    <button
                        class="pagination-arrow"
                        disabled=move || {
                            movie_page.get() == 0
                                || submitting.get()
                                || vote_submitted.get()
                        }
                        on:click=move |_| movie_page.set(0)
                        aria-label="Previous movies"
                    >
                        "←"
                    </button>

                    <div class="pagination-indicator">
                        <span
                            class=move || {
                                if movie_page.get() == 0 {
                                    "pagination-dot active"
                                } else {
                                    "pagination-dot"
                                }
                            }
                        ></span>

                        <span
                            class=move || {
                                if movie_page.get() == 1 {
                                    "pagination-dot active"
                                } else {
                                    "pagination-dot"
                                }
                            }
                        ></span>
                    </div>

                    <button
                        class="pagination-arrow"
                        disabled=move || {
                            movie_page.get() == 1
                                || submitting.get()
                                || vote_submitted.get()
                        }
                        on:click=move |_| movie_page.set(1)
                        aria-label="Next movies"
                    >
                        "→"
                    </button>
                </div>

                <div class="wallet-login-fields">
                    <input
                        type="password"
                        placeholder="Private key to sign this vote"
                        prop:value=move || private_key.get()
                        on:input=move |event| {
                            private_key.set(event_target_value(&event));
                        }
                    />
                </div>

                <button
                    class="submit-vote-btn movie-submit-button"
                    disabled=move || {
                        selected_movie.get().is_none()
                            || private_key.get().trim().is_empty()
                            || submitting.get()
                            || vote_submitted.get()
                    }
                    on:click=confirm_vote
                >
                    {move || {
                        if submitting.get() {
                            "Signing and submitting vote..."
                        } else if vote_submitted.get() {
                            "Vote submitted"
                        } else {
                            "Cast signed private vote"
                        }
                    }}
                </button>

                {move || {
                    vote_message.get().map(|message| {
                        view! {
                            <p class="vote-success">
                                {message}
                            </p>
                        }
                    })
                }}

                {move || {
                    vote_error.get().map(|error| {
                        view! {
                            <p class="vote-error">
                                {error}
                            </p>
                        }
                    })
                }}

                <div class="vote-footer">
                    <p>
                        "Your private key is only used locally to sign this vote and is cleared after submission."
                    </p>
                </div>
            </div>
        </section>
    }
}
