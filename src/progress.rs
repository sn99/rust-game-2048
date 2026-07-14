//! Map board strength → background reveal / blur relative to the win target.

/// 0.0 at the start (tile 2), 1.0 when `max_tile >= win_tile`.
/// Uses log2 so each merge tier is an even step; squared so mid-game stays blurrier.
pub fn reveal_progress(max_tile: u32, win_tile: u32) -> f32 {
    if max_tile == 0 {
        return 0.0;
    }
    let goal = win_tile.max(2);
    if max_tile >= goal {
        return 1.0;
    }
    let start = 1.0_f32; // log2(2)
    let end = (goal as f32).log2();
    if end <= start {
        return 1.0;
    }
    let log = (max_tile.max(2) as f32).log2();
    let t = ((log - start) / (end - start)).clamp(0.0, 1.0);
    t * t
}

/// CSS `blur()` radius in px (heavy at start, 0 at win).
pub fn blur_px(max_tile: u32, win_tile: u32) -> f32 {
    const MAX_BLUR: f32 = 36.0;
    MAX_BLUR * (1.0 - reveal_progress(max_tile, win_tile))
}

/// Background image opacity (dim while heavily blurred).
pub fn image_opacity(max_tile: u32, win_tile: u32) -> f32 {
    0.2 + 0.75 * reveal_progress(max_tile, win_tile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_is_blurry() {
        assert!(blur_px(2, 256) > 30.0);
        assert!(reveal_progress(2, 256) < 0.05);
    }

    #[test]
    fn mid_game_depends_on_goal() {
        // Halfway to 256 is clearer than halfway to 2048.
        let p_easy = reveal_progress(64, 256);
        let p_hard = reveal_progress(64, 2048);
        assert!(p_easy > p_hard);
        assert!(reveal_progress(128, 256) > 0.2);
        assert!(reveal_progress(128, 256) < 1.0);
    }

    #[test]
    fn win_is_clear() {
        assert_eq!(reveal_progress(64, 64), 1.0);
        assert_eq!(reveal_progress(256, 256), 1.0);
        assert_eq!(blur_px(2048, 2048), 0.0);
        assert_eq!(blur_px(4096, 2048), 0.0);
    }
}
