use crate::progress::{blur_px, image_opacity, veil_opacity};
use leptos::prelude::*;

/// Soft full-viewport ambient image (aspect preserved via contain).
#[component]
pub fn RevealBackground(
    image_url: Signal<Option<String>>,
    max_tile: Signal<u32>,
    win_tile: Signal<u32>,
) -> impl IntoView {
    view! {
        <div class="reveal-bg" aria-hidden="true">
            <Show when=move || image_url.get().is_some()>
                // Dim fill so letterboxing never looks empty
                <div class="reveal-bg-fill"></div>
                <img
                    class="reveal-bg-img"
                    src=move || image_url.get().unwrap_or_default()
                    alt=""
                    style=move || {
                        let tile = max_tile.get();
                        let goal = win_tile.get();
                        let blur = blur_px(tile, goal);
                        let opacity = image_opacity(tile, goal) * 0.55;
                        format!(
                            "filter: blur({blur:.1}px); opacity: {opacity:.3};"
                        )
                    }
                />
                <div
                    class="reveal-bg-veil"
                    style=move || {
                        let v = veil_opacity(max_tile.get(), win_tile.get());
                        format!("opacity: {v:.3};")
                    }
                ></div>
            </Show>
        </div>
    }
}
