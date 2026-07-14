use crate::storage::{save_subreddit, save_subreddit_pool};
use crate::subreddits::{
    curated_blurb, fetch_subreddit_description, pick_random_entry, SubredditPool,
};
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn SubredditBar(
    subreddit: RwSignal<String>,
    pool: RwSignal<SubredditPool>,
    status: Signal<String>,
    loading: Signal<bool>,
    on_load: Callback<()>,
    has_image: Signal<bool>,
) -> impl IntoView {
    // Shown under the sub name: curated blurb, then refined from archive API.
    let description = RwSignal::new(String::new());
    let desc_gen = RwSignal::new(0u32);

    let refresh_description = move |name: String| {
        let curated = curated_blurb(&name).unwrap_or("").to_string();
        description.set(curated.clone());
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
                description.set(format!("r/{name} — custom subreddit"));
            }
        });
    };

    // Initial description for restored sub.
    {
        let initial = subreddit.get_untracked();
        if !initial.trim().is_empty() {
            refresh_description(initial);
        }
    }

    let load_named = move || {
        let name = subreddit.get_untracked();
        refresh_description(name);
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
        <section class="panel subreddit-bar compact-panel">
            <div class="sub-section">
                <h3 class="panel-title">"Your subreddit"</h3>
                <p class="sub-section-hint">
                    "Type any community name, then load media for this run."
                </p>
                <div class="subreddit-row">
                    <span class="subreddit-prefix" aria-hidden="true">"r/"</span>
                    <input
                        id="subreddit-input"
                        class="subreddit-input"
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
                            // Instant curated blurb while typing known names.
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
                                "Loading…"
                            } else if has_image.get() {
                                "New image"
                            } else {
                                "Load"
                            }
                        }}
                    </button>
                </div>
                <p class="subreddit-description" aria-live="polite">
                    {move || {
                        let name = subreddit.get();
                        let desc = description.get();
                        let name = name.trim();
                        if name.is_empty() {
                            String::new()
                        } else if desc.is_empty() {
                            format!("r/{name}")
                        } else {
                            format!("r/{name} — {desc}")
                        }
                    }}
                </p>
            </div>

            <div class="sub-section sub-section-discover">
                <h3 class="panel-title">"Surprise me"</h3>
                <p class="sub-section-hint">
                    "Picks a random image-friendly community for you and loads it. "
                    "Not related to the text box above until it fills one in."
                </p>
                <div class="discover-row">
                    <button
                        type="button"
                        class="btn btn-discover btn-discover-sfw"
                        prop:disabled=move || loading.get()
                        title="Random safe-for-work photography / aesthetics subreddit"
                        on:click=move |_| on_random_pool(SubredditPool::Sfw)
                    >
                        {move || {
                            if loading.get() && pool.get() == SubredditPool::Sfw {
                                "Finding…"
                            } else {
                                "Random SFW"
                            }
                        }}
                    </button>
                    <button
                        type="button"
                        class="btn btn-discover btn-discover-nsfw"
                        prop:disabled=move || loading.get()
                        title="Random adult (18+) image subreddit only"
                        on:click=move |_| on_random_pool(SubredditPool::NsfwOnly)
                    >
                        {move || {
                            if loading.get() && pool.get() == SubredditPool::NsfwOnly {
                                "Finding…"
                            } else {
                                "Random NSFW (18+)"
                            }
                        }}
                    </button>
                </div>
            </div>

            <p class="subreddit-status">
                {move || status.get()}
            </p>
        </section>
    }
}
