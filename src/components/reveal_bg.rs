use crate::components::media_view::MediaView;
use crate::progress::veil_opacity;
use crate::reddit::MediaItem;
use leptos::prelude::*;

#[component]
pub fn RevealBackground(
    item: Signal<Option<MediaItem>>,
    max_tile: Signal<u32>,
    win_tile: Signal<u32>,
) -> impl IntoView {
    view! {
        <div class="reveal-bg" aria-hidden="true">
            <Show when=move || item.get().is_some()>
                <div class="reveal-bg-fill"></div>
                <MediaView
                    item=item
                    max_tile=max_tile
                    win_tile=win_tile
                    opacity_mul=0.55
                    class="reveal-bg-img"
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
