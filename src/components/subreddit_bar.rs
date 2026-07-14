use leptos::prelude::*;

#[component]
pub fn SubredditBar(
    subreddit: RwSignal<String>,
    status: Signal<String>,
    loading: Signal<bool>,
    on_load: Callback<()>,
    has_image: Signal<bool>,
) -> impl IntoView {
    view! {
        <section class="panel subreddit-bar">
            <h2 class="panel-title">"Background"</h2>
            <p class="panel-sub">
                "Load a top image from a subreddit. It stays fully framed and unblurs as you approach your goal."
            </p>
            <div class="subreddit-row">
                <span class="subreddit-prefix">"r/"</span>
                <input
                    id="subreddit-input"
                    class="subreddit-input"
                    type="text"
                    prop:value=move || subreddit.get()
                    prop:disabled=move || loading.get()
                    placeholder="pics"
                    maxlength="32"
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
                            "Load image"
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
