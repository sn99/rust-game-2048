//! Compact top controls: goal difficulty + subreddit/random in one bar.

use crate::difficulty::{self, TARGETS};
use crate::storage::{save_subreddit, save_subreddit_pool};
use crate::subreddits::{
    curated_blurb, fetch_subreddit_description, pick_random_entry, SubredditPool,
};
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn TopBar(
    target: Signal<u32>,
    on_select: Callback<u32>,
    subreddit: RwSignal<String>,
    pool: RwSignal<SubredditPool>,
    status: Signal<String>,
    loading: Signal<bool>,
    on_load: Callback<()>,
    has_image: Signal<bool>,
) -> impl IntoView {
    let description = RwSignal::new(String::new());
    let desc_gen = RwSignal::new(0u32);

    let refresh_description = move |name: String| {
        let curated = curated_blurb(&name).unwrap_or("").to_string();
        description.set(curated);
        let gen = desc_gen.get_untracked() + 1;
        desc_gen.set(gen);
        if name.trim().is_empty() {
            return;
        }
        spawn_local(async move {
            if let Some(live) = fetch_subreddit_description(&name).await {
                if desc_gen.get_untracked() == gen {
                    description.set(live);
                }
            } else if desc_gen.get_untracked() == gen && description.get_untracked().is_empty() {
                description.set("custom subreddit".into());
            }
        });
    };

    {
        let initial = subreddit.get_untracked();
        if !initial.trim().is_empty() {
            refresh_description(initial);
        }
    }

    let load_named = move || {
        refresh_description(subreddit.get_untracked());
        on_load.run(());
    };

    let on_random_pool = move |p: SubredditPool| {
        if loading.get_untracked() {
            return;
        }
        pool.set(p);
        save_subreddit_pool(p);
        let current = subreddit.get_untracked();
        let entry = pick_random_entry(p, Some(current.as_str()));
        subreddit.set(entry.name.to_string());
        save_subreddit(entry.name);
        description.set(entry.blurb.to_string());
        let gen = desc_gen.get_untracked() + 1;
        desc_gen.set(gen);
        let name = entry.name.to_string();
        spawn_local(async move {
            if let Some(live) = fetch_subreddit_description(&name).await {
                if desc_gen.get_untracked() == gen {
                    description.set(live);
                }
            }
        });
        on_load.run(());
    };

    view! {
        <section class="panel top-bar compact-panel" aria-label="Goal and subreddit">
            <div class="top-bar-row">
                <div class="top-bar-goal" role="group" aria-label="Difficulty — win target">
                    <span class="panel-title inline-title">"Goal"</span>
                    <div class="difficulty-buttons">
                        {TARGETS
                            .iter()
                            .copied()
                            .map(|t| {
                                view! {
                                    <button
                                        type="button"
                                        class=move || {
                                            if target.get() == t {
                                                "diff-btn diff-btn-active"
                                            } else {
                                                "diff-btn"
                                            }
                                        }
                                        title=format!("{} — reach {}", difficulty::label(t), t)
                                        on:click=move |_| on_select.run(t)
                                    >
                                        <span class="diff-value">{t}</span>
                                        <span class="diff-name">{difficulty::label(t)}</span>
                                    </button>
                                }
                            })
                            .collect_view()}
                    </div>
                </div>

                <div class="top-bar-divider" aria-hidden="true"></div>

                <div class="top-bar-sub" role="group" aria-label="Subreddit">
                    <span class="panel-title inline-title">"Sub"</span>
                    <span class="subreddit-prefix" aria-hidden="true">"r/"</span>
                    <input
                        id="subreddit-input"
                        class="subreddit-input top-bar-input"
                        type="text"
                        prop:value=move || subreddit.get()
                        prop:disabled=move || loading.get()
                        placeholder="pics"
                        maxlength="200"
                        autocomplete="off"
                        spellcheck="false"
                        on:input=move |ev| {
                            let v = event_target_value(&ev);
                            subreddit.set(v.clone());
                            if let Some(b) = curated_blurb(&v) {
                                description.set(b.to_string());
                            } else if v.trim().is_empty() {
                                description.set(String::new());
                            }
                        }
                        on:keydown=move |ev| {
                            if ev.key() == "Enter" {
                                ev.prevent_default();
                                load_named();
                            }
                        }
                    />
                    <button
                        class="btn btn-load"
                        type="button"
                        prop:disabled=move || loading.get()
                        on:click=move |_| load_named()
                    >
                        {move || {
                            if loading.get() {
                                "…"
                            } else if has_image.get() {
                                "New"
                            } else {
                                "Load"
                            }
                        }}
                    </button>
                    <button
                        type="button"
                        class="btn btn-discover btn-discover-sfw"
                        prop:disabled=move || loading.get()
                        title="Random SFW subreddit"
                        on:click=move |_| on_random_pool(SubredditPool::Sfw)
                    >
                        "SFW"
                    </button>
                    <button
                        type="button"
                        class="btn btn-discover btn-discover-nsfw"
                        prop:disabled=move || loading.get()
                        title="Random NSFW (18+) subreddit"
                        on:click=move |_| on_random_pool(SubredditPool::NsfwOnly)
                    >
                        "NSFW"
                    </button>
                </div>
            </div>
            <p class="top-bar-meta" aria-live="polite">
                {move || {
                    let name = subreddit.get();
                    let name = name.trim().to_string();
                    let desc = description.get();
                    let st = status.get();
                    let mut parts = Vec::new();
                    if !name.is_empty() {
                        if desc.is_empty() {
                            parts.push(format!("r/{name}"));
                        } else {
                            parts.push(format!("r/{name} — {desc}"));
                        }
                    }
                    if !st.is_empty() {
                        parts.push(st);
                    }
                    parts.join(" · ")
                }}
            </p>
        </section>
    }
}
