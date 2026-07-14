use crate::progress::{blur_px, image_opacity};
use crate::reddit::{MediaItem, MediaKind};
use leptos::prelude::*;

/// Render one image or video with the same blur/opacity style.
#[component]
pub fn MediaView(
    item: Signal<Option<MediaItem>>,
    max_tile: Signal<u32>,
    win_tile: Signal<u32>,
    /// Extra opacity multiplier (ambient bg uses < 1).
    #[prop(default = 1.0)]
    opacity_mul: f32,
    /// CSS class on the media element.
    #[prop(into)]
    class: String,
    /// When true, force unblurred (lightbox sharp mode).
    #[prop(optional)]
    sharp: Option<Signal<bool>>,
) -> impl IntoView {
    let style = move || {
        if sharp.map(|s| s.get()).unwrap_or(false) {
            return "filter: none; opacity: 1;".to_string();
        }
        let tile = max_tile.get();
        let goal = win_tile.get();
        let blur = blur_px(tile, goal);
        let opacity = image_opacity(tile, goal) * opacity_mul;
        format!("filter: blur({blur:.1}px); opacity: {opacity:.3};")
    };

    view! {
        {move || {
            let Some(m) = item.get() else {
                return view! { <></> }.into_any();
            };
            let class = class.clone();
            match m.kind {
                MediaKind::Image => view! {
                    <img
                        class=class
                        src=m.url
                        alt=""
                        style=style
                        draggable="false"
                    />
                }.into_any(),
                MediaKind::Video => view! {
                    <video
                        class=class
                        src=m.url
                        poster=m.poster.unwrap_or_default()
                        style=style
                        autoplay
                        loop
                        muted
                        playsinline
                        // Keep audio off; object-fit set in CSS
                        controls=false
                    ></video>
                }.into_any(),
            }
        }}
    }
}
