use crate::progress::{blur_px, image_opacity};
use leptos::prelude::*;
use web_sys::KeyboardEvent;

#[component]
pub fn Lightbox(
    open: RwSignal<bool>,
    image_urls: Signal<Vec<String>>,
    image_title: Signal<String>,
    slide_index: RwSignal<usize>,
    max_tile: Signal<u32>,
    win_tile: Signal<u32>,
    sharp: RwSignal<bool>,
) -> impl IntoView {
    let n_slides = Signal::derive(move || image_urls.get().len().max(1));
    let current_url = Signal::derive(move || {
        let urls = image_urls.get();
        let i = slide_index.get().min(urls.len().saturating_sub(1));
        urls.get(i).cloned().unwrap_or_default()
    });

    let on_key = move |ev: KeyboardEvent| {
        if !open.get_untracked() {
            return;
        }
        match ev.key().as_str() {
            "Escape" => open.set(false),
            "ArrowLeft" => {
                let n = n_slides.get_untracked();
                if n > 1 {
                    slide_index.update(|i| *i = if *i == 0 { n - 1 } else { *i - 1 });
                }
            }
            "ArrowRight" => {
                let n = n_slides.get_untracked();
                if n > 1 {
                    slide_index.update(|i| *i = (*i + 1) % n);
                }
            }
            _ => {}
        }
    };

    view! {
        <Show when=move || open.get() && !image_urls.get().is_empty()>
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
                            let n = n_slides.get();
                            let i = slide_index.get() + 1;
                            if t.is_empty() {
                                if n > 1 { format!("Image {i}/{n}") } else { "Full image".into() }
                            } else if n > 1 {
                                format!("{t} ({i}/{n})")
                            } else {
                                t
                            }
                        }}
                    </p>
                    <div class="lightbox-actions">
                        <Show when=move || (n_slides.get() > 1)>
                            <button
                                type="button"
                                class="btn btn-ghost lightbox-btn"
                                on:click=move |_| {
                                    let n = n_slides.get();
                                    slide_index.update(|i| *i = if *i == 0 { n - 1 } else { *i - 1 });
                                }
                            >
                                "‹ Prev"
                            </button>
                            <button
                                type="button"
                                class="btn btn-ghost lightbox-btn"
                                on:click=move |_| {
                                    let n = n_slides.get();
                                    slide_index.update(|i| *i = (*i + 1) % n);
                                }
                            >
                                "Next ›"
                            </button>
                        </Show>
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
                        src=move || current_url.get()
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
