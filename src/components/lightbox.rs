use crate::progress::{blur_px, image_opacity};
use leptos::prelude::*;
use web_sys::KeyboardEvent;

/// Fullscreen lightbox: entire image, aspect preserved, same unblur as the game.
#[component]
pub fn Lightbox(
    open: RwSignal<bool>,
    image_url: Signal<Option<String>>,
    image_title: Signal<String>,
    max_tile: Signal<u32>,
    win_tile: Signal<u32>,
    sharp: RwSignal<bool>,
) -> impl IntoView {
    let on_key = move |ev: KeyboardEvent| {
        if ev.key() == "Escape" {
            open.set(false);
        }
    };

    view! {
        <Show when=move || open.get() && image_url.get().is_some()>
            <div
                class="lightbox"
                role="dialog"
                aria-modal="true"
                aria-label="Full image view"
                tabindex="0"
                on:click=move |_| open.set(false)
                on:keydown=on_key
            >
                <div class="lightbox-toolbar" on:click=move |ev| ev.stop_propagation()>
                    <p class="lightbox-title">
                        {move || {
                            let t = image_title.get();
                            if t.is_empty() { "Full image".into() } else { t }
                        }}
                    </p>
                    <div class="lightbox-actions">
                        <button
                            type="button"
                            class="btn btn-ghost lightbox-btn"
                            on:click=move |_| sharp.update(|s| *s = !*s)
                        >
                            {move || if sharp.get() { "Show progress blur" } else { "Show sharp" }}
                        </button>
                        <button
                            type="button"
                            class="btn btn-ghost lightbox-btn"
                            on:click=move |_| open.set(false)
                        >
                            "Close"
                        </button>
                    </div>
                </div>
                <div class="lightbox-stage" on:click=move |ev| ev.stop_propagation()>
                    <img
                        class="lightbox-img"
                        src=move || image_url.get().unwrap_or_default()
                        alt=move || image_title.get()
                        style=move || {
                            if sharp.get() {
                                "filter: none; opacity: 1;".into()
                            } else {
                                let tile = max_tile.get();
                                let goal = win_tile.get();
                                let blur = blur_px(tile, goal);
                                let opacity = image_opacity(tile, goal).max(0.85);
                                format!("filter: blur({blur:.1}px); opacity: {opacity:.3};")
                            }
                        }
                    />
                </div>
            </div>
        </Show>
    }
}
