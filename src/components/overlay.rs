use crate::game::GameStatus;
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;

#[component]
pub fn Overlay(
    status: Signal<GameStatus>,
    win_tile: Signal<u32>,
    /// Same as chrome “Next”: new board + next post.
    on_next_game: Callback<()>,
) -> impl IntoView {
    // One window listener for the app lifetime: Enter/Space → Next game while dialog is up.
    Effect::new(move |_| {
        let handler = Closure::<dyn FnMut(KeyboardEvent)>::new(move |ev: KeyboardEvent| {
            if !matches!(
                status.get_untracked(),
                GameStatus::Won | GameStatus::Over
            ) {
                return;
            }
            // Don't steal keys while typing a subreddit.
            if let Some(t) = ev.target() {
                if let Some(el) = t.dyn_ref::<web_sys::HtmlElement>() {
                    let tag = el.tag_name();
                    if tag == "INPUT" || tag == "TEXTAREA" || tag == "SELECT" {
                        return;
                    }
                }
            }
            let key = ev.key();
            if key == "Enter" || key == " " || key == "Spacebar" {
                ev.prevent_default();
                on_next_game.run(());
            }
        });

        if let Some(window) = web_sys::window() {
            let _ = window
                .add_event_listener_with_callback("keydown", handler.as_ref().unchecked_ref());
            // SPA lifetime (same pattern as board keyboard input).
            handler.forget();
        }
    });

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
                aria-label=move || match status.get() {
                    GameStatus::Won => "You won".to_string(),
                    GameStatus::Over => "Game over".to_string(),
                    GameStatus::Playing => String::new(),
                }
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
                            title="New board and next post (Enter)"
                            on:click=move |_| on_next_game.run(())
                        >
                            "Next game"
                        </button>
                    </div>
                    <p class="overlay-hint">"Press Enter"</p>
                </div>
            </div>
        </Show>
    }
}
