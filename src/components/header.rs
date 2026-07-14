use leptos::prelude::*;

#[component]
pub fn Header(
    score: Signal<u32>,
    best: Signal<u32>,
    on_new_game: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="header">
            <div class="header-top">
                <h1 class="title">"2048"</h1>
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
                    "Join the tiles, get to "
                    <strong>"2048"</strong>
                    "!"
                </p>
                <button class="btn btn-new" type="button" on:click=move |_| on_new_game.run(())>
                    "New Game"
                </button>
            </div>
        </div>
    }
}
