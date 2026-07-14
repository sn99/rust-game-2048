use crate::storage::{save_subreddit, save_subreddit_pool};
use crate::subreddits::{pick_random_subreddit, SubredditPool};
use leptos::prelude::*;

#[component]
pub fn SubredditBar(
    subreddit: RwSignal<String>,
    pool: RwSignal<SubredditPool>,
    status: Signal<String>,
    loading: Signal<bool>,
    on_load: Callback<()>,
    has_image: Signal<bool>,
) -> impl IntoView {
    let set_pool = move |p: SubredditPool| {
        pool.set(p);
        save_subreddit_pool(p);
    };

    let on_random = move |_| {
        if loading.get_untracked() {
            return;
        }
        let current = subreddit.get_untracked();
        let name = pick_random_subreddit(pool.get_untracked(), Some(current.as_str()));
        subreddit.set(name.to_string());
        save_subreddit(name);
        on_load.run(());
    };

    view! {
        <section class="panel subreddit-bar compact-panel">
            <div class="subreddit-pool-row" role="group" aria-label="Random subreddit pool">
                <span class="panel-title inline-title">"Random"</span>
                <div class="pool-toggle">
                    <button
                        type="button"
                        class=move || {
                            if pool.get() == SubredditPool::Sfw {
                                "pool-btn pool-btn-active"
                            } else {
                                "pool-btn"
                            }
                        }
                        prop:disabled=move || loading.get()
                        aria-pressed=move || (pool.get() == SubredditPool::Sfw).to_string()
                        on:click=move |_| set_pool(SubredditPool::Sfw)
                    >
                        "SFW"
                    </button>
                    <button
                        type="button"
                        class=move || {
                            if pool.get() == SubredditPool::NsfwOnly {
                                "pool-btn pool-btn-active pool-btn-nsfw"
                            } else {
                                "pool-btn pool-btn-nsfw"
                            }
                        }
                        prop:disabled=move || loading.get()
                        aria-pressed=move || (pool.get() == SubredditPool::NsfwOnly).to_string()
                        title="Adult image subreddits only (18+)"
                        on:click=move |_| set_pool(SubredditPool::NsfwOnly)
                    >
                        "NSFW only"
                    </button>
                </div>
                <button
                    class="btn btn-random"
                    type="button"
                    prop:disabled=move || loading.get()
                    title=move || {
                        format!(
                            "Pick a random {} subreddit and load media",
                            pool.get().label()
                        )
                    }
                    on:click=on_random
                >
                    {move || {
                        if loading.get() {
                            "…"
                        } else {
                            "🎲 Random"
                        }
                    }}
                </button>
            </div>
            <div class="subreddit-row">
                <span class="panel-title inline-title">"Sub"</span>
                <input
                    id="subreddit-input"
                    class="subreddit-input"
                    type="text"
                    prop:value=move || subreddit.get()
                    prop:disabled=move || loading.get()
                    placeholder="pics or reddit.com/r/pics"
                    maxlength="200"
                    autocomplete="off"
                    spellcheck="false"
                    on:input=move |ev| {
                        subreddit.set(event_target_value(&ev));
                    }
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            ev.prevent_default();
                            on_load.run(());
                        }
                    }
                />
                <button
                    class="btn btn-load"
                    type="button"
                    prop:disabled=move || loading.get()
                    on:click=move |_| on_load.run(())
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
            <p class="subreddit-status">
                {move || status.get()}
            </p>
        </section>
    }
}
