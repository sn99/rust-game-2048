use leptos::prelude::*;

#[component]
pub fn Header(
    score: Signal<u32>,
    best: Signal<u32>,
    win_tile: Signal<u32>,
    on_new_game: Callback<()>,
) -> impl IntoView {
    view! {
        <header class="header panel header-panel">
            <div class="header-top">
                <div class="brand">
                    <h1 class="title">{move || win_tile.get().to_string()}</h1>
                    <p class="tagline">"merge · reach your goal"</p>
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
            </div>
            <div class="header-bottom">
                <p class="subtitle">
                    "Join matching tiles until you create a "
                    <strong>{move || win_tile.get().to_string()}</strong>
                    " tile."
                </p>
                <button class="btn btn-new" type="button" on:click=move |_| on_new_game.run(())>
                    "New Game"
                </button>
            </div>
        </header>
    }
}
