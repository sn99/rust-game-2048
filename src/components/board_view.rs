use leptos::prelude::*;

#[component]
pub fn BoardView(cells: Signal<[[u32; 4]; 4]>) -> impl IntoView {
    view! {
        <div class="board" role="grid" aria-label="2048 game board">
            <div class="grid-bg">
                {(0..16).map(|_| view! { <div class="cell-bg"></div> }).collect_view()}
            </div>
            <div class="tiles">
                {move || {
                    let c = cells.get();
                    (0..4)
                        .flat_map(|r| {
                            (0..4).filter_map(move |col| {
                                let v = c[r][col];
                                if v == 0 {
                                    None
                                } else {
                                    Some(view! {
                                        <div
                                            class=tile_class(v)
                                            style=tile_style(r, col)
                                            role="gridcell"
                                        >
                                            {v}
                                        </div>
                                    })
                                }
                            })
                        })
                        .collect_view()
                }}
            </div>
        </div>
    }
}

fn tile_class(value: u32) -> String {
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
    format!("tile {tone}")
}

fn tile_style(row: usize, col: usize) -> String {
    format!("grid-row: {}; grid-column: {};", row + 1, col + 1)
}
