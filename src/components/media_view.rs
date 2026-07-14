use crate::progress::{blur_from_progress, opacity_from_progress};
use crate::reddit::{MediaItem, MediaKind};
use leptos::prelude::*;

/// Render one image or video with blur/opacity from reveal progress (0..1).
#[component]
pub fn MediaView(
    item: Signal<Option<MediaItem>>,
    progress: Signal<f32>,
    /// Extra opacity multiplier (ambient bg uses < 1).
    #[prop(default = 1.0)]
    opacity_mul: f32,
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
        let p = progress.get();
        let blur = blur_from_progress(p);
        let opacity = opacity_from_progress(p) * opacity_mul;
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
                        controls=false
                    ></video>
                }.into_any(),
            }
        }}
    }
}
