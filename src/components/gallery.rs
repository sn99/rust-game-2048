//! Session gallery of unlocked posts (this tab only).

use crate::storage::GalleryEntry;
use leptos::prelude::*;

#[component]
pub fn Gallery(
    entries: Signal<Vec<GalleryEntry>>,
    /// When set, open this gallery item as the active media (does not reset board).
    on_select: Callback<GalleryEntry>,
) -> impl IntoView {
    view! {
        <Show when=move || !entries.get().is_empty()>
            <section class="panel gallery-panel" aria-label="Unlocked this session">
                <div class="gallery-head">
                    <h2 class="panel-title">"Unlocked"</h2>
                    <p class="gallery-meta">
                        {move || format!("{} this session", entries.get().len())}
                    </p>
                </div>
                <ul class="gallery-list">
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
            </section>
        </Show>
    }
}
