use crate::game::GameStatus;
use leptos::prelude::*;

#[component]
pub fn Overlay(
    status: Signal<GameStatus>,
    win_tile: Signal<u32>,
    on_keep_going: Callback<()>,
    on_try_again: Callback<()>,
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
                        <Show when=move || status.get() == GameStatus::Won>
                            <button
                                class="btn btn-keep"
                                type="button"
                                on:click=move |_| on_keep_going.run(())
                            >
                                "Keep going"
                            </button>
                        </Show>
                        <button
                            class="btn btn-retry"
                            type="button"
                            on:click=move |_| on_try_again.run(())
                        >
                            "Try again"
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
