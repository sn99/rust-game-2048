use crate::game::GameStatus;
use leptos::prelude::*;

#[component]
pub fn Overlay(
    status: Signal<GameStatus>,
    win_tile: Signal<u32>,
    /// Same as chrome “Next”: new board + next post.
    on_next_game: Callback<()>,
) -> impl IntoView {
    view! {
        <Show when=move || matches!(status.get(), GameStatus::Won | GameStatus::Over)>
            <div
                class=move || {
                    match status.get() {
                        GameStatus::Won => "overlay overlay-won",
                        GameStatus::Over => "overlay overlay-over",
                        GameStatus::Playing => "overlay",
                    }
                }
                role="dialog"
                aria-modal="true"
            >
                <div class="overlay-inner">
                    <p class="overlay-message">
                        {move || match status.get() {
                            GameStatus::Won => format!("You hit {}!", win_tile.get()),
                            GameStatus::Over => "Game over!".into(),
                            GameStatus::Playing => String::new(),
                        }}
                    </p>
                    <div class="overlay-actions">
                        <button
                            class="btn btn-retry"
                            type="button"
                            title="New board and next post"
                            on:click=move |_| on_next_game.run(())
                        >
                            "Next game"
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
