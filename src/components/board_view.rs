use crate::game::Tile;
use leptos::prelude::*;

#[component]
pub fn BoardView(tiles: Signal<Vec<Tile>>) -> impl IntoView {
    view! {
        <div class="board" role="grid" aria-label="2048 game board">
            <div class="grid-bg">
                {(0..16).map(|_| view! { <div class="cell-bg"></div> }).collect_view()}
            </div>
            <div class="tiles">
                <For
                    each=move || {
                        // Only ids drive mount/unmount; order is stable by id.
                        let mut ids: Vec<u64> = tiles.get().into_iter().map(|t| t.id).collect();
                        ids.sort_unstable();
                        ids
                    }
                    key=|id| *id
                    children=move |id| {
                        // Reactive lookups keep one DOM node per id so transform can transition.
                        let style = move || {
                            tiles.with(|list| {
                                list.iter()
                                    .find(|t| t.id == id)
                                    .map(|t| tile_transform(t.row, t.col))
                                    .unwrap_or_default()
                            })
                        };
                        let outer_class = move || {
                            tiles.with(|list| {
                                let t = list.iter().find(|t| t.id == id);
                                let mut c = String::from("tile");
                                if let Some(t) = t {
                                    // No slide transition on brand-new tiles (avoids flying from 0,0).
                                    if t.is_new {
                                        c.push_str(" tile-spawn");
                                    }
                                    if t.is_merged {
                                        c.push_str(" tile-merged-layer");
                                    }
                                }
                                c
                            })
                        };
                        let inner_class = move || {
                            tiles.with(|list| {
                                list.iter()
                                    .find(|t| t.id == id)
                                    .map(|t| tile_inner_class(t.value, t.is_new, t.is_merged))
                                    .unwrap_or_else(|| "tile-inner".into())
                            })
                        };
                        let value = move || {
                            tiles.with(|list| {
                                list.iter()
                                    .find(|t| t.id == id)
                                    .map(|t| t.value)
                                    .unwrap_or(0)
                            })
                        };
                        view! {
                            <div class=outer_class style=style role="gridcell">
                                <div class=inner_class>{value}</div>
                            </div>
                        }
                    }
                />
            </div>
        </div>
    }
}

fn tile_inner_class(value: u32, is_new: bool, is_merged: bool) -> String {
    let tone = match value {
        2 => "tile-2",
        4 => "tile-4",
        8 => "tile-8",
        16 => "tile-16",
        32 => "tile-32",
        64 => "tile-64",
        128 => "tile-128",
        256 => "tile-256",
        512 => "tile-512",
        1024 => "tile-1024",
        2048 => "tile-2048",
        _ => "tile-super",
    };
    let mut s = format!("tile-inner {tone}");
    if is_new {
        s.push_str(" tile-new");
    }
    if is_merged {
        s.push_str(" tile-merged");
    }
    s
}

/// Position via transform so CSS can transition slides without fighting scale anims on the inner.
fn tile_transform(row: u8, col: u8) -> String {
    format!(
        "transform: translate(calc({col} * (var(--cell) + var(--gap))), calc({row} * (var(--cell) + var(--gap))));"
    )
}
