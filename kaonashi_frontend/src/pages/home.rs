use leptos::prelude::*;

#[component]
pub fn HomePage(page: RwSignal<&'static str>) -> impl IntoView {
    view! {
        <section class="hero">
            <div class="hero-card">
                <h1>
                    "Private "
                    <span class="highlight">"voting"</span>
                    " for hidden identities."
                </h1>

                <p>
                    "I do believe Marsellus Wallace, my husband, your boss, told you to take ME out and do WHATEVER I WANTED. Now I wanna dance, I wanna win. I want that trophy, so dance good"
                </p>

                <button
                    class="start-voting-button"
                    on:click=move |_| page.set("wallet")
                >
                    "Connect Wallet"
                </button>
            </div>
        </section>
    }
}
