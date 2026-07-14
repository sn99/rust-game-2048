//! Dense top chrome: scores, goal, subreddit — one primary action + SFW/NSFW.

use crate::difficulty::{self, TARGETS};
use crate::storage::{save_subreddit, save_subreddit_pool};
use crate::subreddits::{
    curated_blurb, fetch_subreddit_description, pick_random_subreddit_live, SubredditPool,
};
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn Chrome(
    score: Signal<u32>,
    best: Signal<u32>,
    win_tile: Signal<u32>,
    target: Signal<u32>,
    on_select: Callback<u32>,
    subreddit: RwSignal<String>,
    pool: RwSignal<SubredditPool>,
    /// Errors only (empty on success) — avoids “top week” chrome noise.
    status: Signal<String>,
    loading: Signal<bool>,
    /// Reset board + load media for current sub (or start if empty).
    on_play: Callback<()>,
    /// User is changing sub / skipping a slow fetch — abandon in-flight load.
    on_sub_edit: Callback<()>,
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

    let run_play = move || {
        refresh_description(subreddit.get_untracked());
        on_play.run(());
    };

    let on_random_pool = move |p: SubredditPool| {
        // Always allow skip: cancel whatever is loading for the previous sub.
        on_sub_edit.run(());
        pool.set(p);
        save_subreddit_pool(p);
        let current = subreddit.get_untracked();
        let gen = desc_gen.get_untracked() + 1;
        desc_gen.set(gen);
        description.set("Finding a community…".into());
        spawn_local(async move {
            match pick_random_subreddit_live(p, Some(current.as_str())).await {
                Ok(entry) => {
                    if desc_gen.get_untracked() != gen {
                        return;
                    }
                    subreddit.set(entry.name.clone());
                    save_subreddit(&entry.name);
                    if entry.blurb.is_empty() {
                        description.set(format!("r/{}", entry.name));
                    } else {
                        description.set(entry.blurb);
                    }
                    on_play.run(());
                }
                Err(e) => {
                    if desc_gen.get_untracked() == gen {
                        description.set(e.to_string());
                    }
                }
            }
        });
    };

    let action_label = move || {
        if loading.get() {
            "Fetching…"
        } else if has_image.get() {
            "Next"
        } else {
            "Play"
        }
    };

    let action_title = move || {
        if loading.get() {
            "Loading media…".to_string()
        } else if has_image.get() {
            "New board + another top post from this sub".to_string()
        } else {
            "Load a top post and start playing".to_string()
        }
    };

    view! {
        <header class="panel chrome compact-panel" aria-label="Game controls">
            <div class="chrome-row chrome-row-main">
                <div class="chrome-zone chrome-brand">
                    <h1 class="title">{move || win_tile.get().to_string()}</h1>
                    <div class="scores">
                        <div class="score-box">
                            <div class="score-label">"SCORE"</div>
                            <div class="score-value">{score}</div>
                        </div>
                        <div class="score-box">
                            <div class="score-label">"BEST"</div>
                            <div class="score-value">{best}</div>
                        </div>
                    </div>
                </div>

                <div class="chrome-sep" aria-hidden="true"></div>

                <div class="chrome-zone chrome-goals" role="group" aria-label="Difficulty — win target">
                    <span class="chrome-zone-label">"Goal"</span>
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

                <div class="chrome-sep" aria-hidden="true"></div>

                <div class="chrome-zone chrome-sub" role="group" aria-label="Subreddit">
                    <span class="subreddit-prefix" aria-hidden="true">"r/"</span>
                    <input
                        id="subreddit-input"
                        class="subreddit-input chrome-input"
                        type="text"
                        prop:value=move || subreddit.get()
                        placeholder="pics"
                        maxlength="200"
                        autocomplete="off"
                        spellcheck="false"
                        on:input=move |ev| {
                            let v = event_target_value(&ev);
                            // Typing abandons any in-flight fetch for the previous sub.
                            on_sub_edit.run(());
                            subreddit.set(v.clone());
                            if let Some(b) = curated_blurb(&v) {
                                description.set(b.to_string());
                            } else if v.trim().is_empty() {
                                description.set(String::new());
                            } else {
                                description.set(format!("r/{v}"));
                            }
                        }
                        on:keydown=move |ev| {
                            if ev.key() == "Enter" {
                                ev.prevent_default();
                                run_play();
                            }
                        }
                    />
                    <button
                        class="btn btn-primary-action"
                        type="button"
                        title=action_title
                        on:click=move |_| run_play()
                    >
                        {action_label}
                    </button>
                    <button
                        type="button"
                        class="btn btn-discover btn-discover-sfw"
                        title="Surprise me with a random SFW subreddit (cancels current fetch)"
                        on:click=move |_| on_random_pool(SubredditPool::Sfw)
                    >
                        "SFW"
                    </button>
                    <button
                        type="button"
                        class="btn btn-discover btn-discover-nsfw"
                        title="Surprise me with a random NSFW (18+) subreddit (cancels current fetch)"
                        on:click=move |_| on_random_pool(SubredditPool::NsfwOnly)
                    >
                        "NSFW"
                    </button>
                </div>
            </div>

            <Show when=move || {
                !description.get().is_empty()
                    || !subreddit.get().trim().is_empty()
                    || !status.get().trim().is_empty()
            }>
                <div class="chrome-row chrome-row-meta" aria-live="polite">
                    <Show when=move || !status.get().trim().is_empty()>
                        <p class="chrome-meta-status">{move || status.get()}</p>
                    </Show>
                    <p class="chrome-meta-blurb">
                        {move || {
                            let name = subreddit.get();
                            let name = name.trim().to_string();
                            let desc = description.get();
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
            </Show>
        </header>
    }
}
