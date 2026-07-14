//! Map board strength → background reveal / blur.

/// Target tile that fully reveals the image.
pub const WIN_TILE: u32 = 2048;

/// 0.0 at the start (tile 2), 1.0 at 2048+.
/// Uses log2 so each merge tier is an even step; squared so mid-game stays blurrier.
pub fn reveal_progress(max_tile: u32) -> f32 {
    if max_tile == 0 {
        return 0.0;
    }
    if max_tile >= WIN_TILE {
        return 1.0;
    }
    let log = (max_tile.max(2) as f32).log2();
    // tile 2 → 0, tile 2048 (2^11) → 1
    let t = ((log - 1.0) / 10.0).clamp(0.0, 1.0);
    t * t
}

/// CSS `blur()` radius in px (heavy at start, 0 at win).
pub fn blur_px(max_tile: u32) -> f32 {
    const MAX_BLUR: f32 = 36.0;
    MAX_BLUR * (1.0 - reveal_progress(max_tile))
}

/// Background image opacity (dim while heavily blurred).
pub fn image_opacity(max_tile: u32) -> f32 {
    0.2 + 0.75 * reveal_progress(max_tile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_is_blurry() {
        assert!(blur_px(2) > 30.0);
        assert!(reveal_progress(2) < 0.05);
    }

    #[test]
    fn mid_game_partial() {
        let p = reveal_progress(256);
        assert!(p > 0.2 && p < 0.8);
        assert!(blur_px(256) > 5.0 && blur_px(256) < 30.0);
    }

    #[test]
    fn win_is_clear() {
        assert_eq!(reveal_progress(2048), 1.0);
        assert_eq!(blur_px(2048), 0.0);
        assert_eq!(blur_px(4096), 0.0);
    }
}
