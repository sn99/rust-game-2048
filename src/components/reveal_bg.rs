use crate::components::media_view::MediaView;
use crate::progress::veil_from_progress;
use crate::reddit::MediaItem;
use leptos::prelude::*;

#[component]
pub fn RevealBackground(
    item: Signal<Option<MediaItem>>,
    progress: Signal<f32>,
) -> impl IntoView {
    view! {
        <div class="reveal-bg" aria-hidden="true">
            <Show when=move || item.get().is_some()>
                <div class="reveal-bg-fill"></div>
                <MediaView
                    item=item
                    progress=progress
                    opacity_mul=0.55
                    class="reveal-bg-img"
                />
                <div
                    class="reveal-bg-veil"
                    style=move || {
                        let v = veil_from_progress(progress.get());
                        format!("opacity: {v:.3};")
                    }
                ></div>
            </Show>
        </div>
    }
}
