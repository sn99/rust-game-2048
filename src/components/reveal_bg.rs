use crate::progress::{blur_px, image_opacity};
use leptos::prelude::*;

/// Full-viewport background image that unblurs as `max_tile` rises toward 2048.
#[component]
pub fn RevealBackground(
    image_url: Signal<Option<String>>,
    max_tile: Signal<u32>,
) -> impl IntoView {
    view! {
        <div class="reveal-bg" aria-hidden="true">
            <Show when=move || image_url.get().is_some()>
                <div
                    class="reveal-bg-image"
                    style=move || {
                        let url = image_url.get().unwrap_or_default();
                        let tile = max_tile.get();
                        let blur = blur_px(tile);
                        let opacity = image_opacity(tile);
                        format!(
                            "background-image: url(\"{url}\"); \
                             filter: blur({blur:.1}px); \
                             opacity: {opacity:.3}; \
                             transform: scale(1.08);"
                        )
                    }
                ></div>
                <div class="reveal-bg-veil"></div>
            </Show>
        </div>
    }
}
