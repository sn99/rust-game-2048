//! Map board strength → background reveal / blur relative to the win target.

/// 0.0 at the start (tile 2), 1.0 when `max_tile >= win_tile`.
/// Linear in log2 so each merge tier is an even step toward the selected goal.
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
    ((log - start) / (end - start)).clamp(0.0, 1.0)
}

/// CSS `blur()` radius in px — full clear exactly at the goal for any level.
pub fn blur_px(max_tile: u32, win_tile: u32) -> f32 {
    const MAX_BLUR: f32 = 42.0;
    MAX_BLUR * (1.0 - reveal_progress(max_tile, win_tile))
}

/// Image opacity (readable even when blurred; full at goal).
pub fn image_opacity(max_tile: u32, win_tile: u32) -> f32 {
    0.45 + 0.55 * reveal_progress(max_tile, win_tile)
}

/// Soft page veil over ambient background (less at high progress).
pub fn veil_opacity(max_tile: u32, win_tile: u32) -> f32 {
    0.72 - 0.42 * reveal_progress(max_tile, win_tile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_is_blurry() {
        assert!(blur_px(2, 256) > 35.0);
        assert!(reveal_progress(2, 256) < 0.05);
    }

    #[test]
    fn mid_game_depends_on_goal() {
        let p_easy = reveal_progress(64, 256);
        let p_hard = reveal_progress(64, 2048);
        assert!(p_easy > p_hard);
        // One step before 256 on easy goal is near complete
        assert!(reveal_progress(128, 256) > 0.5);
        assert!(reveal_progress(128, 256) < 1.0);
    }

    #[test]
    fn win_is_fully_clear_any_level() {
        for goal in [64, 128, 256, 512, 1024, 2048, 4096] {
            assert_eq!(reveal_progress(goal, goal), 1.0);
            assert_eq!(blur_px(goal, goal), 0.0);
            assert!((image_opacity(goal, goal) - 1.0).abs() < 0.001);
        }
        assert_eq!(blur_px(512, 256), 0.0);
    }
}
