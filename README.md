# rust-game-2048

Classic **2048** in the browser, written in **Rust** with **[Leptos](https://leptos.dev/)** (CSR) and bundled by **[Trunk](https://trunkrs.dev/)**. Designed to host on **GitHub Pages**.

Inspired by [play2048.co](https://play2048.co/) (Gabriele Cirulli). This is an original reimplementation.

## Play

After enabling GitHub Pages (Actions source):

`https://<user>.github.io/rust-game-2048/`

## Features

- Classic 4×4 rules (slide, merge once, 90%/10% spawn of 2/4)
- Score + best (persisted in `localStorage`)
- Win at 2048 with **Keep going**, then **Game over**
- Keyboard (arrows / WASD) and touch swipe
- Classic tile colors and pop animation
- Optional **subreddit background**: load a random top image; it unblurs as your highest tile approaches 2048
  - Uses [Pullpush](https://pullpush.io) / Arctic Shift archives (Reddit blocks direct browser API calls)

## Develop

Requirements:

- Rust stable
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- [Trunk](https://trunkrs.dev/): `cargo install trunk`

```bash
# Unit tests (native)
cargo test

# Dev server (http://127.0.0.1:8080)
trunk serve

# Production build → dist/
trunk build --release
```

## Deploy (GitHub Pages)

1. Push this repo to GitHub.
2. **Settings → Pages → Build and deployment → Source: GitHub Actions**.
3. Push to `main` (or run the **Deploy to GitHub Pages** workflow manually).

The workflow builds with Trunk and deploys the `dist/` folder.

## Project layout

| Path | Role |
|------|------|
| `src/game/` | Pure game logic (unit tested) |
| `src/components/` | Leptos UI |
| `src/input.rs` | Keyboard + swipe |
| `src/storage.rs` | Best score in `localStorage` |
| `style.css` | Classic look |
| `.github/workflows/deploy.yml` | Pages CI |

## License

MIT. 2048 is a known game concept; assets and code here are original.
