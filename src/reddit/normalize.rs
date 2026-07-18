//! Subreddit name normalization.

/// Normalize `r/pics`, full reddit URLs, or bare names → `pics`.
pub fn normalize_subreddit(raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }

    let lower = s.to_ascii_lowercase();
    if let Some(idx) = lower.find("/r/") {
        let rest = &s[idx + 3..];
        let name = rest.split(['/', '?', '#']).next().unwrap_or("").trim();
        return validate_sub_name(name);
    }
    if let Some(idx) = lower.find("r/") {
        if idx == 0 || s[..idx].contains('.') || s[..idx].ends_with('/') {
            let rest = &s[idx + 2..];
            let name = rest.split(['/', '?', '#']).next().unwrap_or("").trim();
            if let Some(n) = validate_sub_name(name) {
                return Some(n);
            }
        }
    }

    let s = s.trim_start_matches('/').trim();
    let s = s
        .strip_prefix("r/")
        .or_else(|| s.strip_prefix("R/"))
        .unwrap_or(s)
        .trim()
        .trim_end_matches('/');
    validate_sub_name(s)
}

fn validate_sub_name(s: &str) -> Option<String> {
    if s.is_empty() || s.len() > 32 {
        return None;
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return None;
    }
    Some(s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_names_and_urls() {
        assert_eq!(normalize_subreddit("pics").as_deref(), Some("pics"));
        assert_eq!(normalize_subreddit("r/EarthPorn").as_deref(), Some("EarthPorn"));
        assert_eq!(
            normalize_subreddit("https://www.reddit.com/r/aww/comments/x").as_deref(),
            Some("aww")
        );
        assert_eq!(normalize_subreddit("").as_deref(), None);
        assert_eq!(normalize_subreddit("bad name!").as_deref(), None);
    }
}
