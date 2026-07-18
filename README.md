# rust-game-2048

Classic **2048** in the browser, written in **Rust** with **[Leptos](https://leptos.dev/)** (CSR) and bundled by **[Trunk](https://trunkrs.dev/)**. Hosted on **GitHub Pages**.

Inspired by [play2048.co](https://play2048.co/) (Gabriele Cirulli). This is an original reimplementation.

**Core loop:** pick a difficulty goal → load media from a subreddit (typed or random) → play 2048 while the image unblurs as your highest tile approaches the goal.

## Play

After enabling GitHub Pages (Actions source):

`https://<user>.github.io/rust-game-2048/`

## Features

- Classic 4×4 rules (slide, merge once, 90%/10% spawn of 2/4)
- **Difficulty / goal** tiles: 16 (Baby) through 4096 (Insane); default **256** (Medium)
- Score + best score persisted in `localStorage`; goal and last subreddit remembered
- Keyboard (arrows / WASD) and touch swipe
- Classic tile colors and slide / pop animation
- **Subreddit media unlock**
  - Type any `r/` name (or paste a reddit URL) and hit **Play** / **Next**
  - Or **SFW** / **NSFW** for a random live-discovered community
  - Background + side panel unblur from 0% → 100% as your max tile approaches the goal
  - Reddit post link unlocks when you hit the goal tile
  - Prefetches the next few posts so **Next** / **Next game** stay snappy
  - **Session restore**: refresh keeps board, score, media, and reveal progress (same tab)
  - **Gallery**: unlocked posts this session appear under the reveal panel (click to re-view)
  - Media via [Arctic Shift](https://arctic-shift.photon-reddit.com) / [Pullpush](https://pullpush.io) archives (Reddit blocks direct browser API use)

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

The workflow runs `cargo test`, builds with Trunk, and deploys the `dist/` folder.

## Project layout

| Path | Role |
|------|------|
| `src/game/` | Pure board / move logic (unit tested) |
| `src/difficulty.rs` | Goal levels and labels |
| `src/progress.rs` | Reveal / blur progress |
| `src/reddit/` | Media types, normalize, parse, network/probe |
| `src/subreddits.rs` | Live SFW/NSFW discovery |
| `src/components/` | Leptos UI |
| `src/input.rs` | Keyboard + swipe |
| `src/storage.rs` | localStorage / sessionStorage |
| `style.css` | Classic look |
| `.github/workflows/deploy.yml` | Test + Pages CI |

## License

MIT. 2048 is a known game concept; assets and code here are original.
