use crate::components::media_view::MediaView;
use crate::reddit::MediaItem;
use leptos::prelude::*;
use web_sys::KeyboardEvent;

#[component]
pub fn Lightbox(
    open: RwSignal<bool>,
    items: Signal<Vec<MediaItem>>,
    image_title: Signal<String>,
    slide_index: RwSignal<usize>,
    progress: Signal<f32>,
    sharp: RwSignal<bool>,
) -> impl IntoView {
    let n_slides = Signal::derive(move || items.get().len().max(1));
    let current_item = Signal::derive(move || {
        let list = items.get();
        let i = slide_index.get().min(list.len().saturating_sub(1));
        list.get(i).cloned()
    });
    let sharp_sig = Signal::derive(move || sharp.get());

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
        <Show when=move || open.get() && !items.get().is_empty()>
            <div
                class="lightbox"
                role="dialog"
                aria-modal="true"
                aria-label="Full media view"
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
                                if n > 1 { format!("Media {i}/{n}") } else { "Full view".into() }
                            } else if n > 1 {
                                format!("{t} ({i}/{n})")
                            } else {
                                t
                            }
                        }}
                    </p>
                    <div class="lightbox-actions">
                        <Show when=move || (n_slides.get() > 1)>
                            <button type="button" class="btn btn-ghost lightbox-btn" on:click=move |_| {
                                let n = n_slides.get();
                                slide_index.update(|i| *i = if *i == 0 { n - 1 } else { *i - 1 });
                            }>"‹ Prev"</button>
                            <button type="button" class="btn btn-ghost lightbox-btn" on:click=move |_| {
                                let n = n_slides.get();
                                slide_index.update(|i| *i = (*i + 1) % n);
                            }>"Next ›"</button>
                        </Show>
                        <button type="button" class="btn btn-ghost lightbox-btn" on:click=move |_| sharp.update(|s| *s = !*s)>
                            {move || if sharp.get() { "Show progress blur" } else { "Show sharp" }}
                        </button>
                        <button type="button" class="btn btn-ghost lightbox-btn" on:click=move |_| open.set(false)>"Close"</button>
                    </div>
                </div>
                <div class="lightbox-stage" on:click=move |ev| ev.stop_propagation()>
                    <MediaView item=current_item progress=progress class="lightbox-img" sharp=sharp_sig />
                </div>
            </div>
        </Show>
    }
}
