//! Win-target difficulty levels.

/// Selectable goal tiles (powers of two).
pub const TARGETS: &[u32] = &[16, 32, 64, 128, 256, 512, 1024, 2048, 4096];

pub const DEFAULT_TARGET: u32 = 256;

/// Short label for the goal button.
pub fn label(target: u32) -> &'static str {
    match target {
        16 => "Baby",
        32 => "Super easy",
        64 => "Easy",
        128 => "Normal",
        256 => "Medium",
        512 => "Hard",
        1024 => "Expert",
        2048 => "Classic",
        4096 => "Insane",
        _ => "Custom",
    }
}

pub fn is_valid_target(n: u32) -> bool {
    TARGETS.contains(&n)
}

pub fn clamp_target(n: u32) -> u32 {
    if is_valid_target(n) {
        n
    } else {
        DEFAULT_TARGET
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_default() {
        assert_eq!(label(16), "Baby");
        assert_eq!(label(32), "Super easy");
        assert_eq!(label(64), "Easy");
        assert_eq!(label(2048), "Classic");
        assert!(is_valid_target(16));
        assert!(is_valid_target(32));
        assert!(!is_valid_target(100));
        assert_eq!(clamp_target(99), DEFAULT_TARGET);
    }
}
