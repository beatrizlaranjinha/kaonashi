use leptos::prelude::*;

#[component]
pub fn Navbar(page: RwSignal<&'static str>) -> impl IntoView {
    view! {
        <nav class="navbar">
            <button
                class="logo-button"
                on:click=move |_| page.set("home")
            >
                <span class="logo-word">"Kaonashi"</span>
            </button>

            <div class="navbar-links">
                <button on:click=move |_| page.set("home")>
                    "Home"
                </button>

                <button on:click=move |_| page.set("decades")>
                    "Ballots"
                </button>
            </div>
        </nav>
    }
}
