use leptos::prelude::*;

#[component]
pub fn SubredditBar(
    subreddit: RwSignal<String>,
    status: Signal<String>,
    loading: Signal<bool>,
    on_load: Callback<()>,
    on_clear: Callback<()>,
    has_image: Signal<bool>,
    reveal_pct: Signal<u32>,
) -> impl IntoView {
    view! {
        <div class="subreddit-bar">
            <label class="subreddit-label" for="subreddit-input">
                "Background subreddit"
            </label>
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
                    {move || if loading.get() { "Loading…" } else { "Load image" }}
                </button>
                <Show when=move || has_image.get()>
                    <button
                        class="btn btn-clear"
                        type="button"
                        on:click=move |_| on_clear.run(())
                    >
                        "Clear"
                    </button>
                </Show>
            </div>
            <p class="subreddit-status">
                {move || status.get()}
            </p>
            <Show when=move || has_image.get()>
                <p class="subreddit-hint">
                    "Image unblurs toward your goal as tiles climb — "
                    {move || format!("{}% revealed", reveal_pct.get())}
                </p>
            </Show>
        </div>
    }
}
