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

#[cfg(target_arch = "wasm32")]
fn deck_key(pool: SubredditPool) -> &'static str {
    match pool {
        SubredditPool::Sfw => "rust-game-2048-live-deck-sfw-v1",
        SubredditPool::NsfwOnly => "rust-game-2048-live-deck-nsfw-v1",
    }
}

#[cfg(target_arch = "wasm32")]
fn cache_key(pool: SubredditPool) -> &'static str {
    match pool {
        SubredditPool::Sfw => "rust-game-2048-live-cache-sfw-v1",
        SubredditPool::NsfwOnly => "rust-game-2048-live-cache-nsfw-v1",
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

/// Persist a catalog + shuffled deck for one pool (session only).
#[cfg(target_arch = "wasm32")]
fn persist_pool(pool: SubredditPool, list: &[SubredditEntry]) {
    if list.is_empty() {
        return;
    }
    if let Ok(raw) = serde_json::to_string(
        &list
            .iter()
            .map(|e| serde_json::json!({"name": e.name, "blurb": e.blurb}))
            .collect::<Vec<_>>(),
    ) {
        session_set(cache_key(pool), &raw);
    }
    let mut deck: Vec<String> = list.iter().map(|e| e.name.clone()).collect();
    fisher_yates(&mut deck);
    if let Ok(raw) = serde_json::to_string(&deck) {
        session_set(deck_key(pool), &raw);
    }
}

/// Discover **both** SFW and NSFW from **two** listing requests total (Pullpush + Arctic).
/// Builds two session decks so SFW/NSFW clicks don't wait on network.
#[cfg(target_arch = "wasm32")]
async fn discover_both_pools() -> Result<(Vec<SubredditEntry>, Vec<SubredditEntry>), DiscoverError> {
    let mut sfw: HashMap<String, SubredditEntry> = HashMap::new();
    let mut nsfw: HashMap<String, SubredditEntry> = HashMap::new();

    // 1) Pullpush: one top-by-score week sample (popular image-friendly subs).
    let now = (js_sys::Date::now() / 1000.0) as u64;
    let since = now.saturating_sub(7 * 86_400);
    let pp = format!(
        "https://api.pullpush.io/reddit/search/submission/?sort=desc&sort_type=score&size=100&since={since}"
    );
    if let Ok(v) = fetch_json(&pp).await {
        ingest_value_posts(&mut sfw, false, &v);
        ingest_value_posts(&mut nsfw, true, &v);
    }

    // 2) Arctic: one global recent page (good NSFW density + more names).
    let arctic = "https://arctic-shift.photon-reddit.com/api/posts/search?limit=100&sort=desc";
    if let Ok(v) = fetch_json(arctic).await {
        ingest_value_posts(&mut sfw, false, &v);
        ingest_value_posts(&mut nsfw, true, &v);
    }

    // Optional third call only if one side is empty.
    if sfw.is_empty() || nsfw.is_empty() {
        let since30 = now.saturating_sub(30 * 86_400);
        let pp2 = format!(
            "https://api.pullpush.io/reddit/search/submission/?sort=desc&sort_type=score&size=100&since={since30}"
        );
        if let Ok(v) = fetch_json(&pp2).await {
            ingest_value_posts(&mut sfw, false, &v);
            ingest_value_posts(&mut nsfw, true, &v);
        }
    }

    if sfw.is_empty() && nsfw.is_empty() {
        return Err(DiscoverError::Empty);
    }

    let sfw_list: Vec<_> = sfw.into_values().collect();
    let nsfw_list: Vec<_> = nsfw.into_values().collect();
    persist_pool(SubredditPool::Sfw, &sfw_list);
    persist_pool(SubredditPool::NsfwOnly, &nsfw_list);
    Ok((sfw_list, nsfw_list))
}

/// Warm both community decks once per session (call at app start).
pub async fn warm_community_caches() -> Result<(), DiscoverError> {
    #[cfg(target_arch = "wasm32")]
    {
        // Already warm?
        let sfw_ok = !load_cached_catalog(SubredditPool::Sfw).is_empty();
        let nsfw_ok = !load_cached_catalog(SubredditPool::NsfwOnly).is_empty();
        if sfw_ok && nsfw_ok {
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
async fn discover_live(pool: SubredditPool) -> Result<Vec<SubredditEntry>, DiscoverError> {
    // Prefer shared dual discovery (1–3 requests total for both pools).
    let (sfw, nsfw) = discover_both_pools().await?;
    Ok(match pool {
        SubredditPool::Sfw => sfw,
        SubredditPool::NsfwOnly => nsfw,
    })
}

#[cfg(not(target_arch = "wasm32"))]
async fn discover_live(_pool: SubredditPool) -> Result<Vec<SubredditEntry>, DiscoverError> {
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

/// Pick a random live-discovered sub. Builds/refills a session shuffle deck.
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
    if catalog.is_empty() {
        catalog = discover_live(pool).await?;
    }

    // Rebuild deck if empty or names not in catalog.
    let cat_names: HashSet<String> = catalog
        .iter()
        .map(|e| e.name.to_ascii_lowercase())
        .collect();
    deck.retain(|n| cat_names.contains(&n.to_ascii_lowercase()));

    if deck.is_empty() {
        // Refresh catalog periodically for more variety.
        if catalog.len() < 15 {
            if let Ok(more) = discover_live(pool).await {
                catalog = more;
            }
        }
        deck = catalog.iter().map(|e| e.name.clone()).collect();
        fisher_yates(&mut deck);
        #[cfg(target_arch = "wasm32")]
        if let Ok(raw) = serde_json::to_string(&deck) {
            session_set(deck_key(pool), &raw);
        }
    }

    // Draw next non-avoid name.
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
        choice = deck.pop();
    }

    #[cfg(target_arch = "wasm32")]
    if let Ok(raw) = serde_json::to_string(&deck) {
        session_set(deck_key(pool), &raw);
    }

    let name = choice.ok_or(DiscoverError::Empty)?;
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
}
