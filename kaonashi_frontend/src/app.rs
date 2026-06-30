use leptos::prelude::*;

use crate::{
    components::navbar::Navbar,
    pages::{
        chairperson::ChairpersonPage, decades::DecadesPage, home::HomePage, results::ResultsPage,
        tie_resolution::TieResolutionPage, vote::VotePage, wallet::WalletPage,
    },
};

#[component]
pub fn App() -> impl IntoView {
    // Página atual da aplicação.
    let page = RwSignal::new("home");

    // Década escolhida pelo utilizador.
    let selected_decade = RwSignal::new(0_u8);

    // Dados públicos da wallet.
    // A private key NÃO é guardada globalmente.
    let wallet_id = RwSignal::new(None::<String>);
    let wallet_address = RwSignal::new(None::<String>);

    // Estado do fluxo da chairperson.
    let chairperson_step = RwSignal::new(1_u8);

    view! {
        <Navbar page=page />

        <main class="page">
            {move || {
                match page.get() {
                    "home" => {
                        view! {
                            <HomePage page=page />
                        }
                        .into_any()
                    }

                    "wallet" => {
                        view! {
                            <WalletPage
                                page=page
                                logged_wallet_id=wallet_id
                                logged_wallet_address=wallet_address
                            />
                        }
                        .into_any()
                    }

                    "decades" => {
                        view! {
                            <DecadesPage
                                page=page
                                selected_decade=selected_decade
                                wallet_id=wallet_id
                                wallet_address=wallet_address
                            />
                        }
                        .into_any()
                    }

                    "vote" => {
                        view! {
                            <VotePage
                                page=page
                                selected_decade=selected_decade
                                wallet_id=wallet_id
                                wallet_address=wallet_address
                            />
                        }
                        .into_any()
                    }

                    "results" => {
                        view! {
                            <ResultsPage page=page />
                        }
                        .into_any()
                    }

                    "chairperson" => {
                        view! {
                            <ChairpersonPage
                                page=page
                                selected_decade=selected_decade
                                wallet_id=wallet_id
                                wallet_address=wallet_address
                                current_step=chairperson_step
                            />
                        }
                        .into_any()
                    }

                    "tie-resolution" => {
                        view! {
                            <TieResolutionPage
                                page=page
                                selected_decade=selected_decade
                                wallet_id=wallet_id
                                wallet_address=wallet_address
                            />
                        }
                        .into_any()
                    }

                    _ => {
                        view! {
                            <HomePage page=page />
                        }
                        .into_any()
                    }
                }
            }}
        </main>
    }
}
