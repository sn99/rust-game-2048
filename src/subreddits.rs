//! Live subreddit discovery (no hardcoded catalog).
//! Sources: Pullpush top posts + Arctic Shift posts, with session cache.

use std::collections::{HashMap, HashSet};

/// Which pool the random finder draws from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubredditPool {
    Sfw,
    NsfwOnly,
}

impl SubredditPool {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sfw => "sfw",
            Self::NsfwOnly => "nsfw",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "nsfw" | "nsfw_only" | "only_nsfw" | "18" => Self::NsfwOnly,
            _ => Self::Sfw,
        }
    }

    pub fn wants_nsfw(self) -> bool {
        matches!(self, Self::NsfwOnly)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubredditEntry {
    pub name: String,
    pub blurb: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiscoverError {
    Network(String),
    Empty,
}

impl std::fmt::Display for DiscoverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoverError::Network(s) => write!(f, "Could not discover subreddits ({s})"),
            DiscoverError::Empty => write!(
                f,
                "No matching subreddits found — try again or type a name"
            ),
        }
    }
}

/// Minimum names we want in a pool before we stop expanding discovery.
const MIN_CATALOG: usize = 60;
/// Arctic random time windows per discovery pass (variety >> one "recent" page).
const DISCOVER_WINDOWS: usize = 4;
/// How far back we sample posts (days).
const DISCOVER_LOOKBACK_DAYS: u64 = 180;

#[cfg(target_arch = "wasm32")]
fn deck_key(pool: SubredditPool) -> &'static str {
    // v2: multi-window discovery + used-set (old v1 caches were tiny + repetitive).
    match pool {
        SubredditPool::Sfw => "rust-game-2048-live-deck-sfw-v2",
        SubredditPool::NsfwOnly => "rust-game-2048-live-deck-nsfw-v2",
    }
}

#[cfg(target_arch = "wasm32")]
fn cache_key(pool: SubredditPool) -> &'static str {
    match pool {
        SubredditPool::Sfw => "rust-game-2048-live-cache-sfw-v2",
        SubredditPool::NsfwOnly => "rust-game-2048-live-cache-nsfw-v2",
    }
}

#[cfg(target_arch = "wasm32")]
fn used_key(pool: SubredditPool) -> &'static str {
    match pool {
        SubredditPool::Sfw => "rust-game-2048-live-used-sfw-v2",
        SubredditPool::NsfwOnly => "rust-game-2048-live-used-nsfw-v2",
    }
}

#[cfg(target_arch = "wasm32")]
fn session_get(key: &str) -> Option<String> {
    web_sys::window()
        .and_then(|w| w.session_storage().ok().flatten())
        .and_then(|s| s.get_item(key).ok().flatten())
}

#[cfg(target_arch = "wasm32")]
fn session_set(key: &str, val: &str) {
    if let Some(storage) = web_sys::window().and_then(|w| w.session_storage().ok().flatten()) {
        let _ = storage.set_item(key, val);
    }
}

fn looks_like_image_post(url: &str, post_hint: Option<&str>, is_gallery: bool, is_video: bool) -> bool {
    if is_gallery || is_video {
        return true;
    }
    if matches!(
        post_hint,
        Some("image") | Some("hosted:video") | Some("rich:video")
    ) {
        return true;
    }
    let u = url.to_ascii_lowercase();
    u.contains("i.redd.it")
        || u.contains("preview.redd.it")
        || u.contains("i.imgur.com")
        || u.contains("imgur.com/")
        || u.contains("v.redd.it")
        || u.ends_with(".jpg")
        || u.ends_with(".jpeg")
        || u.ends_with(".png")
        || u.ends_with(".webp")
        || u.ends_with(".gif")
        || u.ends_with(".gifv")
        || u.contains("/gallery/")
}

fn ingest_post(
    map: &mut HashMap<String, SubredditEntry>,
    want_nsfw: bool,
    subreddit: Option<&str>,
    over_18: Option<bool>,
    url: Option<&str>,
    post_hint: Option<&str>,
    is_gallery: bool,
    is_video: bool,
    title: Option<&str>,
) {
    let Some(name) = subreddit.map(str::trim).filter(|s| !s.is_empty()) else {
        return;
    };
    if name.len() > 32
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return;
    }
    let nsfw = over_18.unwrap_or(false);
    if want_nsfw != nsfw {
        return;
    }
    let url = url.unwrap_or("");
    if !looks_like_image_post(url, post_hint, is_gallery, is_video) {
        return;
    }
    let key = name.to_ascii_lowercase();
    map.entry(key).or_insert_with(|| SubredditEntry {
        name: name.to_string(),
        blurb: title
            .map(|t| t.trim())
            .filter(|t| !t.is_empty() && !t.starts_with('['))
            .map(|t| {
                if t.len() > 120 {
                    format!("{}…", &t[..117])
                } else {
                    t.to_string()
                }
            })
            .unwrap_or_default(),
    });
}

fn fisher_yates(names: &mut [String]) {
    for i in (1..names.len()).rev() {
        let j = fastrand::usize(..=i);
        names.swap(i, j);
    }
}

/// Fetch a short description for a sub (best-effort).
#[cfg(target_arch = "wasm32")]
pub async fn fetch_subreddit_description(name: &str) -> Option<String> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    let url = format!(
        "https://arctic-shift.photon-reddit.com/api/subreddits/search?subreddit={name}&limit=1"
    );
    let resp = gloo_net::http::Request::get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .ok()?;
    if !resp.ok() {
        return None;
    }
    let text = resp.text().await.ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let p = v.get("data")?.as_array()?.first()?;
    // Ensure exact name match when possible
    let display = p
        .get("display_name")
        .and_then(|x| x.as_str())
        .unwrap_or(name);
    if !display.eq_ignore_ascii_case(name) && p.get("display_name").is_some() {
        // prefix search can return wrong sub — still OK if only result
    }
    let public = p
        .get("public_description")
        .and_then(|x| x.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let title = p
        .get("title")
        .and_then(|x| x.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let mut desc = public.or(title).unwrap_or("Reddit community").to_string();
    desc = desc.split_whitespace().collect::<Vec<_>>().join(" ");
    if desc.len() > 160 {
        desc.truncate(157);
        desc.push_str("…");
    }
    Some(desc)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_subreddit_description(_name: &str) -> Option<String> {
    None
}

/// Curated blurb only when we already know it from discovery (no static table).
pub fn curated_blurb(_name: &str) -> Option<&'static str> {
    None
}

#[cfg(target_arch = "wasm32")]
async fn fetch_json(url: &str) -> Result<serde_json::Value, DiscoverError> {
    let resp = gloo_net::http::Request::get(url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| DiscoverError::Network(e.to_string()))?;
    if resp.status() == 429 {
        return Err(DiscoverError::Network("rate limited".into()));
    }
    if !resp.ok() {
        return Err(DiscoverError::Network(format!("HTTP {}", resp.status())));
    }
    let text = resp
        .text()
        .await
        .map_err(|e| DiscoverError::Network(e.to_string()))?;
    serde_json::from_str(&text).map_err(|e| DiscoverError::Network(e.to_string()))
}

#[cfg(target_arch = "wasm32")]
fn ingest_value_posts(map: &mut HashMap<String, SubredditEntry>, want_nsfw: bool, v: &serde_json::Value) {
    let Some(arr) = v.get("data").and_then(|d| d.as_array()) else {
        return;
    };
    for p in arr {
        ingest_post(
            map,
            want_nsfw,
            p.get("subreddit").and_then(|x| x.as_str()),
            p.get("over_18").and_then(|x| x.as_bool()),
            p.get("url").and_then(|x| x.as_str()),
            p.get("post_hint").and_then(|x| x.as_str()),
            p.get("is_gallery").and_then(|x| x.as_bool()).unwrap_or(false),
            p.get("is_video").and_then(|x| x.as_bool()).unwrap_or(false),
            p.get("title").and_then(|x| x.as_str()),
        );
    }
}

/// Merge `incoming` into existing session catalog (grow, never shrink variety).
#[cfg(target_arch = "wasm32")]
fn merge_and_persist_catalog(pool: SubredditPool, incoming: &[SubredditEntry]) -> Vec<SubredditEntry> {
    let mut map: HashMap<String, SubredditEntry> = HashMap::new();
    for e in load_cached_catalog(pool) {
        map.insert(e.name.to_ascii_lowercase(), e);
    }
    for e in incoming {
        let key = e.name.to_ascii_lowercase();
        map.entry(key).and_modify(|old| {
            if old.blurb.is_empty() && !e.blurb.is_empty() {
                old.blurb = e.blurb.clone();
            }
        }).or_insert_with(|| e.clone());
    }
    let list: Vec<_> = map.into_values().collect();
    if list.is_empty() {
        return list;
    }
    if let Ok(raw) = serde_json::to_string(
        &list
            .iter()
            .map(|e| serde_json::json!({"name": e.name, "blurb": e.blurb}))
            .collect::<Vec<_>>(),
    ) {
        session_set(cache_key(pool), &raw);
    }
    list
}

#[cfg(target_arch = "wasm32")]
fn load_used(pool: SubredditPool) -> HashSet<String> {
    session_get(used_key(pool))
        .and_then(|raw| serde_json::from_str::<Vec<String>>(&raw).ok())
        .map(|v| v.into_iter().map(|s| s.to_ascii_lowercase()).collect())
        .unwrap_or_default()
}

#[cfg(target_arch = "wasm32")]
fn save_used(pool: SubredditPool, used: &HashSet<String>) {
    let mut list: Vec<String> = used.iter().cloned().collect();
    list.sort();
    // Cap so storage stays reasonable in a very long session.
    if list.len() > 800 {
        list.truncate(800);
    }
    if let Ok(raw) = serde_json::to_string(&list) {
        session_set(used_key(pool), &raw);
    }
}

#[cfg(target_arch = "wasm32")]
fn mark_used(pool: SubredditPool, name: &str) {
    let mut used = load_used(pool);
    used.insert(name.to_ascii_lowercase());
    save_used(pool, &used);
}

#[cfg(target_arch = "wasm32")]
fn clear_used(pool: SubredditPool) {
    session_set(used_key(pool), "[]");
}

#[cfg(target_arch = "wasm32")]
fn iso_utc(secs: u64) -> String {
    // Arctic accepts ISO-8601; build without chrono.
    let day = secs / 86_400;
    let rem = secs % 86_400;
    let hour = rem / 3600;
    let min = (rem % 3600) / 60;
    let sec = rem % 60;
    // Days since Unix epoch → year/month/day (civil from days algorithm).
    let (y, m, d) = civil_from_days(day as i64);
    format!("{y:04}-{m:02}-{d:02}T{hour:02}:{min:02}:{sec:02}")
}

/// Howard Hinnant civil_from_days (UTC).
#[cfg(target_arch = "wasm32")]
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

/// Discover communities by sampling **random time windows** of global posts.
/// One "top week" + one "recent" page always returns the same mega-subs — that was the
/// repeat bug. Random windows yield hundreds of distinct image-friendly names.
#[cfg(target_arch = "wasm32")]
async fn discover_both_pools() -> Result<(Vec<SubredditEntry>, Vec<SubredditEntry>), DiscoverError> {
    let mut sfw: HashMap<String, SubredditEntry> = HashMap::new();
    let mut nsfw: HashMap<String, SubredditEntry> = HashMap::new();

    // Seed maps from existing catalog so we grow across discovery passes.
    for e in load_cached_catalog(SubredditPool::Sfw) {
        sfw.insert(e.name.to_ascii_lowercase(), e);
    }
    for e in load_cached_catalog(SubredditPool::NsfwOnly) {
        nsfw.insert(e.name.to_ascii_lowercase(), e);
    }

    let now = (js_sys::Date::now() / 1000.0) as u64;
    let lookback = DISCOVER_LOOKBACK_DAYS * 86_400;

    // 1) Several random Arctic windows across the lookback range.
    for _ in 0..DISCOVER_WINDOWS {
        // Random end in [now-lookback+2h, now-1h], 2-hour slice.
        let span = lookback.saturating_sub(3 * 3600).max(3600);
        let end = now.saturating_sub(3600 + fastrand::u64(0..=span));
        let start = end.saturating_sub(2 * 3600);
        let after = iso_utc(start);
        let before = iso_utc(end);
        let url = format!(
            "https://arctic-shift.photon-reddit.com/api/posts/search?limit=100&sort=desc&after={after}&before={before}"
        );
        if let Ok(v) = fetch_json(&url).await {
            ingest_value_posts(&mut sfw, false, &v);
            ingest_value_posts(&mut nsfw, true, &v);
        }
    }

    // 2) One "latest" page for fresh names (still merges, doesn't replace).
    let arctic = "https://arctic-shift.photon-reddit.com/api/posts/search?limit=100&sort=desc";
    if let Ok(v) = fetch_json(arctic).await {
        ingest_value_posts(&mut sfw, false, &v);
        ingest_value_posts(&mut nsfw, true, &v);
    }

    // 3) Pullpush only if a pool is still thin (top-score is repetitive — avoid as primary).
    if sfw.len() < MIN_CATALOG / 2 || nsfw.len() < MIN_CATALOG / 2 {
        let since = now.saturating_sub(30 * 86_400);
        let pp = format!(
            "https://api.pullpush.io/reddit/search/submission/?sort=desc&sort_type=score&size=100&since={since}"
        );
        if let Ok(v) = fetch_json(&pp).await {
            ingest_value_posts(&mut sfw, false, &v);
            ingest_value_posts(&mut nsfw, true, &v);
        }
    }

    if sfw.is_empty() && nsfw.is_empty() {
        return Err(DiscoverError::Empty);
    }

    let sfw_list: Vec<_> = sfw.into_values().collect();
    let nsfw_list: Vec<_> = nsfw.into_values().collect();
    // Persist catalogs; rebuild decks only from **unused** names so we don't
    // immediately reshuffle names the user already saw this session.
    let sfw_list = merge_and_persist_catalog(SubredditPool::Sfw, &sfw_list);
    let nsfw_list = merge_and_persist_catalog(SubredditPool::NsfwOnly, &nsfw_list);
    rebuild_deck_from_unused(SubredditPool::Sfw, &sfw_list);
    rebuild_deck_from_unused(SubredditPool::NsfwOnly, &nsfw_list);
    Ok((sfw_list, nsfw_list))
}

/// Build a shuffled deck of catalog names not yet used this session.
/// If every name was used, clear the used-set and reshuffle the full catalog.
#[cfg(target_arch = "wasm32")]
fn rebuild_deck_from_unused(pool: SubredditPool, catalog: &[SubredditEntry]) {
    if catalog.is_empty() {
        return;
    }
    let used = load_used(pool);
    let mut fresh: Vec<String> = catalog
        .iter()
        .filter(|e| !used.contains(&e.name.to_ascii_lowercase()))
        .map(|e| e.name.clone())
        .collect();
    if fresh.is_empty() {
        clear_used(pool);
        fresh = catalog.iter().map(|e| e.name.clone()).collect();
    }
    fisher_yates(&mut fresh);
    if let Ok(raw) = serde_json::to_string(&fresh) {
        session_set(deck_key(pool), &raw);
    }
}

/// Warm both community decks once per session (call at app start).
pub async fn warm_community_caches() -> Result<(), DiscoverError> {
    #[cfg(target_arch = "wasm32")]
    {
        let sfw_n = load_cached_catalog(SubredditPool::Sfw).len();
        let nsfw_n = load_cached_catalog(SubredditPool::NsfwOnly).len();
        // Re-expand if either pool is still small (old sessions / thin first pass).
        if sfw_n >= MIN_CATALOG && nsfw_n >= MIN_CATALOG {
            return Ok(());
        }
        discover_both_pools().await?;
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
async fn expand_catalog(pool: SubredditPool) -> Result<Vec<SubredditEntry>, DiscoverError> {
    let (sfw, nsfw) = discover_both_pools().await?;
    Ok(match pool {
        SubredditPool::Sfw => sfw,
        SubredditPool::NsfwOnly => nsfw,
    })
}

#[cfg(not(target_arch = "wasm32"))]
async fn expand_catalog(_pool: SubredditPool) -> Result<Vec<SubredditEntry>, DiscoverError> {
    Err(DiscoverError::Empty)
}

#[cfg(target_arch = "wasm32")]
fn load_cached_catalog(pool: SubredditPool) -> Vec<SubredditEntry> {
    let Some(raw) = session_get(cache_key(pool)) else {
        return Vec::new();
    };
    let Ok(v) = serde_json::from_str::<Vec<serde_json::Value>>(&raw) else {
        return Vec::new();
    };
    v.into_iter()
        .filter_map(|o| {
            let name = o.get("name")?.as_str()?.to_string();
            let blurb = o
                .get("blurb")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                None
            } else {
                Some(SubredditEntry { name, blurb })
            }
        })
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
fn load_cached_catalog(_pool: SubredditPool) -> Vec<SubredditEntry> {
    Vec::new()
}

/// Pick a random live-discovered sub. Session shuffle deck + used-set (no immediate repeats).
pub async fn pick_random_subreddit_live(
    pool: SubredditPool,
    avoid: Option<&str>,
) -> Result<SubredditEntry, DiscoverError> {
    let avoid_l = avoid
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty());

    #[cfg(target_arch = "wasm32")]
    let mut deck: Vec<String> = session_get(deck_key(pool))
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();

    #[cfg(not(target_arch = "wasm32"))]
    let mut deck: Vec<String> = Vec::new();

    let mut catalog = load_cached_catalog(pool);
    if catalog.len() < MIN_CATALOG / 3 {
        // First pick / thin cache: expand with multi-window discovery.
        if let Ok(more) = expand_catalog(pool).await {
            catalog = more;
            #[cfg(target_arch = "wasm32")]
            {
                deck = session_get(deck_key(pool))
                    .and_then(|raw| serde_json::from_str(&raw).ok())
                    .unwrap_or_default();
            }
        } else if catalog.is_empty() {
            return Err(DiscoverError::Empty);
        }
    }

    let cat_names: HashSet<String> = catalog
        .iter()
        .map(|e| e.name.to_ascii_lowercase())
        .collect();
    deck.retain(|n| cat_names.contains(&n.to_ascii_lowercase()));

    // Drop names already used this session (deck may be stale after used-set updates).
    #[cfg(target_arch = "wasm32")]
    {
        let used = load_used(pool);
        deck.retain(|n| !used.contains(&n.to_ascii_lowercase()));
    }

    if deck.is_empty() {
        // Expand catalog for more variety, then rebuild from unused names.
        if let Ok(more) = expand_catalog(pool).await {
            catalog = more;
        }
        #[cfg(target_arch = "wasm32")]
        {
            rebuild_deck_from_unused(pool, &catalog);
            deck = session_get(deck_key(pool))
                .and_then(|raw| serde_json::from_str(&raw).ok())
                .unwrap_or_default();
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            deck = catalog.iter().map(|e| e.name.clone()).collect();
            fisher_yates(&mut deck);
        }
    }

    // Draw next non-avoid name (deck front is already shuffled).
    let mut choice: Option<String> = None;
    let mut i = 0;
    while i < deck.len() {
        let c = deck[i].clone();
        if avoid_l
            .as_ref()
            .is_some_and(|a| a == &c.to_ascii_lowercase())
        {
            i += 1;
            continue;
        }
        choice = Some(c);
        deck.remove(i);
        break;
    }
    if choice.is_none() {
        // Only the avoided current sub left — allow it as last resort after used clear.
        #[cfg(target_arch = "wasm32")]
        {
            clear_used(pool);
            rebuild_deck_from_unused(pool, &catalog);
            deck = session_get(deck_key(pool))
                .and_then(|raw| serde_json::from_str(&raw).ok())
                .unwrap_or_default();
            if let Some(a) = &avoid_l {
                deck.retain(|n| n.to_ascii_lowercase() != *a);
            }
            choice = deck.pop();
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            choice = deck.pop();
        }
    }

    #[cfg(target_arch = "wasm32")]
    if let Ok(raw) = serde_json::to_string(&deck) {
        session_set(deck_key(pool), &raw);
    }

    let name = choice.ok_or(DiscoverError::Empty)?;

    #[cfg(target_arch = "wasm32")]
    mark_used(pool, &name);

    let mut entry = catalog
        .into_iter()
        .find(|e| e.name.eq_ignore_ascii_case(&name))
        .unwrap_or(SubredditEntry {
            name: name.clone(),
            blurb: String::new(),
        });

    // Only one optional description call when discovery left blurb empty.
    if entry.blurb.is_empty() {
        if let Some(desc) = fetch_subreddit_description(&entry.name).await {
            entry.blurb = desc;
        }
    }

    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_parse() {
        assert!(!SubredditPool::Sfw.wants_nsfw());
        assert!(SubredditPool::NsfwOnly.wants_nsfw());
        assert_eq!(SubredditPool::from_str("nsfw"), SubredditPool::NsfwOnly);
    }

    #[test]
    fn image_post_heuristic() {
        assert!(looks_like_image_post(
            "https://i.redd.it/x.jpg",
            Some("image"),
            false,
            false
        ));
        assert!(!looks_like_image_post(
            "https://www.reddit.com/r/news/comments/x",
            None,
            false,
            false
        ));
    }

    #[test]
    fn fisher_yates_preserves_membership() {
        let mut names: Vec<String> = (0..50).map(|i| format!("sub{i}")).collect();
        let set: HashSet<_> = names.iter().cloned().collect();
        fisher_yates(&mut names);
        let after: HashSet<_> = names.iter().cloned().collect();
        assert_eq!(set, after);
        assert_eq!(names.len(), 50);
    }
}
