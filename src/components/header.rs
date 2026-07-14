use leptos::prelude::*;

#[component]
pub fn Header(
    score: Signal<u32>,
    best: Signal<u32>,
    win_tile: Signal<u32>,
    on_new_game: Callback<()>,
) -> impl IntoView {
    view! {
        <header class="header panel header-panel compact-panel">
            <div class="header-top">
                <div class="brand">
                    <h1 class="title">{move || win_tile.get().to_string()}</h1>
                </div>
                <div class="scores">
                    <div class="score-box">
                        <div class="score-label">"SCORE"</div>
                        <div class="score-value">{score}</div>
                    </div>
                    <div class="score-box">
                        <div class="score-label">"BEST"</div>
                        <div class="score-value">{best}</div>
                    </div>
                </div>
                <button class="btn btn-new" type="button" on:click=move |_| on_new_game.run(())>
                    "New Game"
                </button>
            </div>
        </header>
    }
}
