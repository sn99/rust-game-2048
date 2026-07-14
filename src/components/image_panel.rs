use crate::progress::{blur_px, image_opacity, reveal_progress};
use leptos::prelude::*;

/// In-game preview: full image, aspect ratio preserved, progressive unblur.
#[component]
pub fn ImagePanel(
    image_url: Signal<Option<String>>,
    image_title: Signal<String>,
    max_tile: Signal<u32>,
    win_tile: Signal<u32>,
    reveal_pct: Signal<u32>,
    on_open_full: Callback<()>,
    on_clear: Callback<()>,
) -> impl IntoView {
    view! {
        <Show when=move || image_url.get().is_some()>
            <section class="panel image-panel">
                <div class="image-panel-head">
                    <div>
                        <h2 class="panel-title">"Reveal"</h2>
                        <p class="image-panel-meta">
                            {move || {
                                let t = image_title.get();
                                if t.is_empty() {
                                    "Background image".into()
                                } else if t.len() > 72 {
                                    format!("{}…", &t[..69])
                                } else {
                                    t
                                }
                            }}
                        </p>
                    </div>
                    <div class="image-panel-actions">
                        <button
                            type="button"
                            class="btn btn-ghost"
                            on:click=move |_| on_open_full.run(())
                        >
                            "View full"
                        </button>
                        <button
                            type="button"
                            class="btn btn-ghost"
                            on:click=move |_| on_clear.run(())
                        >
                            "Clear"
                        </button>
                    </div>
                </div>

                <div class="image-frame">
                    <img
                        class="image-frame-img"
                        src=move || image_url.get().unwrap_or_default()
                        alt=move || image_title.get()
                        style=move || {
                            let tile = max_tile.get();
                            let goal = win_tile.get();
                            let blur = blur_px(tile, goal);
                            let opacity = image_opacity(tile, goal);
                            format!(
                                "filter: blur({blur:.1}px); opacity: {opacity:.3};"
                            )
                        }
                    />
                    <div class="image-frame-badge">
                        {move || format!("{}%", reveal_pct.get())}
                    </div>
                </div>

                <div class="reveal-meter" aria-hidden="true">
                    <div
                        class="reveal-meter-fill"
                        style=move || {
                            let p = reveal_progress(max_tile.get(), win_tile.get()) * 100.0;
                            format!("width: {p:.1}%;")
                        }
                    ></div>
                </div>
                <p class="image-panel-hint">
                    {move || {
                        format!(
                            "Unblurs fully at {} · now {}% clear",
                            win_tile.get(),
                            reveal_pct.get()
                        )
                    }}
                </p>
            </section>
        </Show>
    }
}
