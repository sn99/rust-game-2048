use crate::components::media_view::MediaView;
use crate::reddit::MediaItem;
use leptos::prelude::*;
use web_sys::TouchEvent;

#[component]
pub fn ImagePanel(
    items: Signal<Vec<MediaItem>>,
    image_title: Signal<String>,
    image_permalink: Signal<Option<String>>,
    slide_index: RwSignal<usize>,
    post_unlocked: Signal<bool>,
    progress: Signal<f32>,
    reveal_pct: Signal<u32>,
    reveal_hint: Signal<String>,
    on_open_full: Callback<()>,
    on_clear: Callback<()>,
) -> impl IntoView {
    let touch_x = RwSignal::new(None::<f64>);

    let n_slides = Signal::derive(move || items.get().len().max(1));
    let current_item = Signal::derive(move || {
        let list = items.get();
        let i = slide_index.get().min(list.len().saturating_sub(1));
        list.get(i).cloned()
    });

    let step = move |delta: i32| {
        let n = n_slides.get() as i32;
        if n <= 1 {
            return;
        }
        slide_index.update(|i| {
            let cur = *i as i32;
            *i = (cur + delta).rem_euclid(n) as usize;
        });
    };

    let on_touch_start = move |ev: TouchEvent| {
        if let Some(t) = ev.touches().get(0) {
            touch_x.set(Some(t.client_x() as f64));
        }
    };
    let on_touch_end = move |ev: TouchEvent| {
        let Some(start) = touch_x.get() else {
            return;
        };
        touch_x.set(None);
        let Some(t) = ev.changed_touches().get(0) else {
            return;
        };
        let dx = t.client_x() as f64 - start;
        if dx.abs() < 40.0 {
            return;
        }
        if dx < 0.0 {
            step(1);
        } else {
            step(-1);
        }
    };

    view! {
        <Show when=move || !items.get().is_empty()>
            <section class="panel image-panel">
                <div class="image-panel-head">
                    <div class="image-panel-copy">
                        <h2 class="panel-title">"Reveal"</h2>
                        <p class="image-panel-meta">
                            {move || {
                                let t = image_title.get();
                                if t.is_empty() {
                                    "Background media".into()
                                } else if t.len() > 64 {
                                    format!("{}…", &t[..61])
                                } else {
                                    t
                                }
                            }}
                        </p>
                        <Show when=move || image_permalink.get().is_some()>
                            {move || {
                                if post_unlocked.get() {
                                    view! {
                                        <a
                                            class="reddit-post-link"
                                            href=move || image_permalink.get().unwrap_or_default()
                                            target="_blank"
                                            rel="noopener noreferrer"
                                        >
                                            "Open Reddit post ↗"
                                        </a>
                                    }.into_any()
                                } else {
                                    view! {
                                        <span class="reddit-post-locked" title="Reach your goal tile first">
                                            "🔒 Reddit post unlocks when you win"
                                        </span>
                                    }.into_any()
                                }
                            }}
                        </Show>
                    </div>
                    <div class="image-panel-actions">
                        <button type="button" class="btn btn-ghost" on:click=move |_| on_open_full.run(())>
                            "View full"
                        </button>
                        <button type="button" class="btn btn-ghost" on:click=move |_| on_clear.run(())>
                            "Clear"
                        </button>
                    </div>
                </div>

                <div
                    class="image-frame"
                    on:touchstart=on_touch_start
                    on:touchend=on_touch_end
                >
                    <MediaView item=current_item progress=progress class="image-frame-img" />
                    <div class="image-frame-badge">
                        {move || format!("{}%", reveal_pct.get())}
                    </div>

                    <Show when=move || (n_slides.get() > 1)>
                        <button type="button" class="carousel-nav carousel-prev" aria-label="Previous" on:click=move |_| step(-1)>"‹"</button>
                        <button type="button" class="carousel-nav carousel-next" aria-label="Next" on:click=move |_| step(1)>"›"</button>
                        <div class="carousel-dots" aria-hidden="true">
                            {move || {
                                let n = n_slides.get();
                                let cur = slide_index.get();
                                (0..n)
                                    .map(|i| {
                                        view! {
                                            <button
                                                type="button"
                                                class=if i == cur { "carousel-dot carousel-dot-active" } else { "carousel-dot" }
                                                on:click=move |_| slide_index.set(i)
                                            ></button>
                                        }
                                    })
                                    .collect_view()
                            }}
                        </div>
                        <div class="carousel-count">
                            {move || format!("{} / {}", slide_index.get() + 1, n_slides.get())}
                        </div>
                    </Show>
                </div>

                <div class="reveal-meter" aria-hidden="true">
                    <div
                        class="reveal-meter-fill"
                        style=move || format!("width: {:.1}%;", progress.get() * 100.0)
                    ></div>
                </div>
                <p class="image-panel-hint">
                    {move || {
                        let base = reveal_hint.get();
                        if n_slides.get() > 1 {
                            format!("{base} · swipe gallery")
                        } else {
                            base
                        }
                    }}
                </p>
            </section>
        </Show>
    }
}
