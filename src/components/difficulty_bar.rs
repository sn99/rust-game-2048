use crate::difficulty::{self, TARGETS};
use leptos::prelude::*;

#[component]
pub fn DifficultyBar(
    target: Signal<u32>,
    on_select: Callback<u32>,
) -> impl IntoView {
    view! {
        <div class="difficulty-bar" role="group" aria-label="Difficulty — win target">
            <div class="difficulty-label">"Goal"</div>
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
    }
}
