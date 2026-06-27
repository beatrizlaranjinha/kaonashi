use leptos::prelude::*;

#[component]
pub fn WalletPage(
    page: RwSignal<&'static str>,
    logged_wallet_id: RwSignal<Option<String>>,
    logged_wallet_address: RwSignal<Option<String>>,
) -> impl IntoView {
    let wallet_id = RwSignal::new(String::new());
    let public_key = RwSignal::new(String::new());

    let error = RwSignal::new(None::<String>);

    let connect_wallet = move |_| {
        // valor dos inputs
        let id = wallet_id.get().trim().to_string();
        let key = public_key.get().trim().to_string();

        // A private key só será usada depois, na página de voto.
        if id.is_empty() || key.is_empty() {
            error.set(Some("Enter the wallet ID and public key.".to_string()));
            return;
        }

        // Guardamos wallet id e adress
        logged_wallet_id.set(Some(id));
        logged_wallet_address.set(Some(key));

        // Avançar para a escolha da década.
        page.set("decades");
    };

    view! {
        <section class="wallet-login-page">
            <div class="wallet-login-card">
                <p class="wallet-login-kicker">
                    "Kaonashi"
                </p>

                <h1>
                    "Connect wallet to start voting"
                </h1>

                <p class="wallet-login-description">
                    "Enter the wallet ID and public key. The private key is only used when casting a vote."
                </p>

                <div class="wallet-login-divider"></div>

                <div class="wallet-login-fields">
                    <input
                        type="text"
                        placeholder="Wallet ID"
                        prop:value=move || wallet_id.get()
                        on:input=move |event| {
                            wallet_id.set(event_target_value(&event));
                        }
                    />

                    <input
                        type="text"
                        placeholder="Solana public key"
                        prop:value=move || public_key.get()
                        on:input=move |event| {
                            public_key.set(event_target_value(&event));
                        }
                    />
                </div>

                {move || {
                    error.get().map(|message| {
                        view! {
                            <p class="wallet-login-error">
                                {message}
                            </p>
                        }
                    })
                }}

                <button
                    class="wallet-signin-button"
                    on:click=connect_wallet
                >
                    "Continue to voting"
                </button>

                <p class="wallet-login-note">
                    "The private key is not stored. It will only be requested when submitting a vote."
                </p>
            </div>
        </section>
    }
}
