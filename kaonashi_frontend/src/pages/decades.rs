use leptos::prelude::*;

#[component]
pub fn DecadesPage(
    page: RwSignal<&'static str>,
    selected_decade: RwSignal<u8>,
    wallet_id: RwSignal<Option<String>>,
    wallet_address: RwSignal<Option<String>>,
) -> impl IntoView {
    let decades = [
        (0_u8, "1970s"),
        (1_u8, "1980s"),
        (2_u8, "1990s"),
        (3_u8, "2000s"),
        (4_u8, "2010s"),
        (5_u8, "2020s"),
    ];

    let error_message = RwSignal::new(None::<String>);

    view! {
        <section class="vote-page">
            <div class="vote-app">
                <header class="vote-header">
                    <h1>"Choose a decade"</h1>
                    <p>"Select a decade to see its movie ballot."</p>
                </header>

                <div class="vote-options">
                    {decades
                        .into_iter()
                        .map(|(decade_id, decade_name)| {
                            view! {
                                <button
                                    class="vote-card"
                                    on:click=move |_| {
                                        let Some(_) = wallet_id.get() else {
                                            error_message.set(Some(
                                                "You must log in before voting.".to_string()
                                            ));
                                            return;
                                        };

                                        let Some(_) = wallet_address.get() else {
                                            error_message.set(Some(
                                                "The wallet public key is missing.".to_string()
                                            ));
                                            return;
                                        };

                                        error_message.set(None);
                                        selected_decade.set(decade_id);
                                        page.set("vote");
                                    }
                                >
                                    <span class="vote-name">{decade_name}</span>
                                </button>
                            }
                        })
                        .collect_view()}
                </div>

                {move || {
                    error_message.get().map(|error| {
                        view! {
                            <p class="vote-error">{error}</p>
                        }
                    })
                }}
            </div>
        </section>
    }
}
