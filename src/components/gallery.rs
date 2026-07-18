//! Session gallery of unlocked posts (this tab only) — collapsible so Reveal keeps space.

use crate::storage::GalleryEntry;
use leptos::prelude::*;

#[component]
pub fn Gallery(
    entries: Signal<Vec<GalleryEntry>>,
    /// When set, open this gallery item as the active media (does not reset board).
    on_select: Callback<GalleryEntry>,
    /// Fired when the dropdown opens — parent re-checks for deleted posts.
    on_open: Callback<()>,
) -> impl IntoView {
    // Collapsed by default — full side column stays with Reveal.
    let open = RwSignal::new(false);

    view! {
        <Show when=move || !entries.get().is_empty()>
            <section
                class=move || {
                    if open.get() {
                        "panel gallery-panel gallery-panel-open"
                    } else {
                        "panel gallery-panel gallery-panel-closed"
                    }
                }
                aria-label="Unlocked this session"
            >
                <button
                    type="button"
                    class="gallery-toggle"
                    aria-expanded=move || open.get().to_string()
                    aria-controls="gallery-dropdown"
                    on:click=move |_| {
                        open.update(|v| {
                            *v = !*v;
                            if *v {
                                on_open.run(());
                            }
                        });
                    }
                >
                    <span class="gallery-toggle-label">
                        <span class="gallery-toggle-title">"Unlocked"</span>
                        <span class="gallery-toggle-count">
                            {move || format!("({})", entries.get().len())}
                        </span>
                    </span>
                    <span class="gallery-toggle-chevron" aria-hidden="true">
                        {move || if open.get() { "▾" } else { "▸" }}
                    </span>
                </button>

                <Show when=move || open.get()>
                    <ul class="gallery-list" id="gallery-dropdown" role="list">
                        {move || {
                            entries
                                .get()
                                .into_iter()
                                .enumerate()
                                .map(|(idx, entry)| {
                                    let thumb = entry
                                        .media
                                        .items
                                        .first()
                                        .map(|i| {
                                            i.poster
                                                .clone()
                                                .unwrap_or_else(|| i.url.clone())
                                        })
                                        .unwrap_or_default();
                                    let title = if entry.media.title.is_empty() {
                                        format!("r/{}", entry.media.subreddit)
                                    } else if entry.media.title.len() > 48 {
                                        format!("{}…", &entry.media.title[..45])
                                    } else {
                                        entry.media.title.clone()
                                    };
                                    let entry_click = entry.clone();
                                    view! {
                                        <li class="gallery-item">
                                            <button
                                                type="button"
                                                class="gallery-card"
                                                title=format!("View · goal {}", entry.goal)
                                                on:click=move |_| on_select.run(entry_click.clone())
                                            >
                                                <span class="gallery-thumb-wrap">
                                                    <img
                                                        class="gallery-thumb"
                                                        src=thumb
                                                        alt=""
                                                        loading="lazy"
                                                        decoding="async"
                                                    />
                                                    <span class="gallery-idx">{idx + 1}</span>
                                                </span>
                                                <span class="gallery-card-copy">
                                                    <span class="gallery-card-title">{title}</span>
                                                    <span class="gallery-card-sub">
                                                        {format!("r/{} · {}", entry.media.subreddit, entry.max_tile)}
                                                    </span>
                                                </span>
                                            </button>
                                        </li>
                                    }
                                })
                                .collect_view()
                        }}
                    </ul>
                </Show>
            </section>
        </Show>
    }
}
