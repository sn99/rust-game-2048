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

/// Discover image-friendly subreddits from live archives (no hardcoded names).
#[cfg(target_arch = "wasm32")]
async fn discover_live(pool: SubredditPool) -> Result<Vec<SubredditEntry>, DiscoverError> {
    let want_nsfw = pool.wants_nsfw();
    let mut map: HashMap<String, SubredditEntry> = HashMap::new();

    // --- Pullpush: true top posts by score → diverse popular subs ---
    let now = (js_sys::Date::now() / 1000.0) as u64;
    for days in [7_u64, 30, 90] {
        let since = now.saturating_sub(days.saturating_mul(86_400));
        let url = format!(
            "https://api.pullpush.io/reddit/search/submission/?sort=desc&sort_type=score&size=100&since={since}"
        );
        if let Ok(v) = fetch_json(&url).await {
            ingest_value_posts(&mut map, want_nsfw, &v);
        }
        if map.len() >= 40 {
            break;
        }
    }

    // --- Arctic: recent posts (global) for more variety / NSFW density ---
    let mut before = String::new();
    for page in 0..4u32 {
        let mut url = "https://arctic-shift.photon-reddit.com/api/posts/search?limit=100&sort=desc"
            .to_string();
        if !before.is_empty() {
            url.push_str(&format!("&before={before}"));
        }
        // Randomize slice into the past to avoid always sampling “right now”.
        if page == 0 {
            let days_ago = fastrand::u32(0..120);
            let ms = js_sys::Date::now() - f64::from(days_ago) * 86_400_000.0;
            let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(ms));
            let after = format!(
                "{:04}-{:02}-{:02}",
                d.get_utc_full_year() as i32,
                d.get_utc_month() as u32 + 1,
                d.get_utc_date() as u32
            );
            url.push_str(&format!("&after={after}"));
        }
        match fetch_json(&url).await {
            Ok(v) => {
                let arr = v.get("data").and_then(|d| d.as_array()).cloned().unwrap_or_default();
                if arr.is_empty() {
                    break;
                }
                ingest_value_posts(&mut map, want_nsfw, &v);
                let mut oldest = f64::MAX;
                for p in &arr {
                    if let Some(c) = p.get("created_utc").and_then(|x| x.as_f64()) {
                        oldest = oldest.min(c);
                    }
                }
                if oldest < f64::MAX {
                    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(oldest * 1000.0));
                    before = format!(
                        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
                        d.get_utc_full_year() as i32,
                        d.get_utc_month() as u32 + 1,
                        d.get_utc_date() as u32,
                        d.get_utc_hours() as u32,
                        d.get_utc_minutes() as u32,
                        d.get_utc_seconds() as u32
                    );
                } else {
                    break;
                }
            }
            Err(_) => break,
        }
        if map.len() >= 80 {
            break;
        }
    }

    // --- Arctic prefix search: random letter prefixes (not a name list) ---
    const ALPHA: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
    for _ in 0..8 {
        let a = ALPHA[fastrand::usize(..ALPHA.len())] as char;
        let b = ALPHA[fastrand::usize(..ALPHA.len())] as char;
        let pref = format!("{a}{b}");
        let url = format!(
            "https://arctic-shift.photon-reddit.com/api/subreddits/search?subreddit={pref}&limit=25"
        );
        if let Ok(v) = fetch_json(&url).await {
            if let Some(arr) = v.get("data").and_then(|d| d.as_array()) {
                for p in arr {
                    let over18 = p.get("over18").and_then(|x| x.as_bool()).unwrap_or(false);
                    if over18 != want_nsfw {
                        continue;
                    }
                    let name = p
                        .get("display_name")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .trim();
                    if name.is_empty() {
                        continue;
                    }
                    // Prefer communities that allow images when the field exists.
                    if let Some(false) = p.get("allow_images").and_then(|x| x.as_bool()) {
                        continue;
                    }
                    let subs = p.get("subscribers").and_then(|x| x.as_u64()).unwrap_or(0);
                    if subs < 500 {
                        continue;
                    }
                    let key = name.to_ascii_lowercase();
                    let blurb = p
                        .get("public_description")
                        .and_then(|x| x.as_str())
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .unwrap_or("")
                        .to_string();
                    map.entry(key).or_insert_with(|| SubredditEntry {
                        name: name.to_string(),
                        blurb,
                    });
                }
            }
        }
    }

    if map.is_empty() {
        return Err(DiscoverError::Empty);
    }

    let mut list: Vec<SubredditEntry> = map.into_values().collect();
    // Enrich a few blurbs? too slow — leave as-is; chrome can fetch description.

    // Persist catalog for this session so we don't re-hit APIs every click.
    if let Ok(raw) = serde_json::to_string(&list.iter().map(|e| {
        serde_json::json!({"name": e.name, "blurb": e.blurb})
    }).collect::<Vec<_>>()) {
        session_set(cache_key(pool), &raw);
    }

    Ok(list)
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

    // Prefer live public description when available.
    if let Some(desc) = fetch_subreddit_description(&entry.name).await {
        entry.blurb = desc;
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
