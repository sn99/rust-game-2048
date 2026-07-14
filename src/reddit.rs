//! Fetch random top media (images, galleries, videos) from a subreddit.

use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaKind {
    Image,
    Video,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaItem {
    pub url: String,
    pub kind: MediaKind,
    /// Poster frame for videos (optional).
    pub poster: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedditMedia {
    pub items: Vec<MediaItem>,
    pub title: String,
    pub permalink: String,
    pub subreddit: String,
    /// Reddit base36 post id (e.g. `1uuqpvt`), used to re-verify the post is still public.
    pub id: String,
    /// Score at fetch time (for ranking “top” within a window).
    pub score: i64,
}

impl RedditMedia {
    pub fn primary_url(&self) -> &str {
        self.items.first().map(|m| m.url.as_str()).unwrap_or("")
    }

    pub fn is_multi(&self) -> bool {
        self.items.len() > 1
    }

    pub fn has_video(&self) -> bool {
        self.items.iter().any(|m| m.kind == MediaKind::Video)
    }
}

/// Back-compat alias used in older call sites/docs.
pub type RedditImage = RedditMedia;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RedditError {
    InvalidSubreddit,
    Network(String),
    NoImages,
    Parse(String),
}

impl std::fmt::Display for RedditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RedditError::InvalidSubreddit => {
                write!(f, "Enter a subreddit name or reddit.com/r/… URL")
            }
            RedditError::Network(s) => {
                if s.contains("429") {
                    write!(f, "{s}")
                } else if s.contains("403") {
                    write!(
                        f,
                        "Could not load media (blocked). Wait a moment and try Load again."
                    )
                } else {
                    write!(f, "Could not load media ({s})")
                }
            }
            RedditError::NoImages => {
                write!(
                    f,
                    "No unused public image/video left — try another sub or reload the page"
                )
            }
            RedditError::Parse(s) => write!(f, "Unexpected API response ({s})"),
        }
    }
}

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

#[derive(Debug, Deserialize)]
struct Listing {
    data: ListingData,
}

#[derive(Debug, Deserialize)]
struct ListingData {
    children: Vec<Child>,
}

#[derive(Debug, Deserialize)]
struct Child {
    data: PostData,
}

#[derive(Debug, Deserialize)]
struct PostArrayResponse {
    data: Vec<PostData>,
}

#[derive(Debug, Deserialize)]
struct PostData {
    id: Option<String>,
    title: Option<String>,
    url: Option<String>,
    permalink: Option<String>,
    author: Option<String>,
    selftext: Option<String>,
    score: Option<i64>,
    /// Set when mods/admins/reddit removed the post.
    removed_by_category: Option<String>,
    /// false ≈ removed from public listing / deleted for robots.
    /// Must be true for us to link the post (CDN may still serve deleted posts’ files).
    is_robot_indexable: Option<bool>,
    #[allow(dead_code)]
    over_18: Option<bool>,
    post_hint: Option<String>,
    is_video: Option<bool>,
    is_gallery: Option<bool>,
    #[serde(default)]
    preview: Option<Preview>,
    #[serde(default)]
    gallery_data: Option<GalleryData>,
    #[serde(default)]
    media_metadata: Option<HashMap<String, MediaMeta>>,
    #[serde(default)]
    media: Option<MediaWrapper>,
    #[serde(default)]
    secure_media: Option<MediaWrapper>,
}

#[derive(Debug, Deserialize)]
struct MediaWrapper {
    reddit_video: Option<RedditVideo>,
}

#[derive(Debug, Deserialize)]
struct RedditVideo {
    fallback_url: Option<String>,
    #[allow(dead_code)]
    hls_url: Option<String>,
    #[allow(dead_code)]
    dash_url: Option<String>,
    #[allow(dead_code)]
    is_gif: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct Preview {
    images: Option<Vec<PreviewImage>>,
}

#[derive(Debug, Deserialize)]
struct PreviewImage {
    source: Option<PreviewSource>,
}

#[derive(Debug, Deserialize)]
struct PreviewSource {
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GalleryData {
    items: Option<Vec<GalleryItem>>,
}

#[derive(Debug, Deserialize)]
struct GalleryItem {
    media_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MediaMeta {
    e: Option<String>,
    s: Option<MediaSource>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MediaSource {
    u: Option<String>,
    gif: Option<String>,
    mp4: Option<String>,
}

pub fn extract_images(json: &str, subreddit: &str) -> Result<Vec<RedditMedia>, RedditError> {
    if let Ok(arr) = serde_json::from_str::<PostArrayResponse>(json) {
        return posts_to_media(arr.data, subreddit);
    }
    let listing: Listing =
        serde_json::from_str(json).map_err(|e| RedditError::Parse(e.to_string()))?;
    let posts: Vec<PostData> = listing.data.children.into_iter().map(|c| c.data).collect();
    posts_to_media(posts, subreddit)
}

fn posts_to_media(posts: Vec<PostData>, subreddit: &str) -> Result<Vec<RedditMedia>, RedditError> {
    let mut out = Vec::new();
    let mut seen_keys = HashSet::new();
    for p in posts {
        if post_is_unavailable(&p) {
            continue;
        }
        let items = collect_post_media(&p);
        if items.is_empty() {
            continue;
        }
        let key = items
            .iter()
            .map(|m| m.url.as_str())
            .collect::<Vec<_>>()
            .join("|");
        if !seen_keys.insert(key) {
            continue;
        }

        let id = post_id_from(&p);
        if id.is_empty() {
            // Without an id we cannot re-verify the post page is still public.
            continue;
        }

        let permalink = p.permalink.clone().unwrap_or_default();
        let permalink = if permalink.starts_with("http") {
            permalink
        } else if permalink.is_empty() {
            format!("https://www.reddit.com/r/{subreddit}/comments/{id}/")
        } else {
            format!("https://www.reddit.com{permalink}")
        };
        out.push(RedditMedia {
            items,
            title: p.title.unwrap_or_default(),
            permalink,
            subreddit: subreddit.to_string(),
            id,
            score: p.score.unwrap_or(0),
        });
    }
    // Highest score first so callers can take a true “top” slice.
    out.sort_by(|a, b| b.score.cmp(&a.score));
    if out.is_empty() {
        Err(RedditError::NoImages)
    } else {
        Ok(out)
    }
}

fn post_id_from(p: &PostData) -> String {
    if let Some(id) = p.id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        // Strip t3_ prefix if present.
        return id.strip_prefix("t3_").unwrap_or(id).to_string();
    }
    // Fallback: /r/sub/comments/<id>/...
    if let Some(pl) = p.permalink.as_deref() {
        let parts: Vec<&str> = pl.split('/').filter(|s| !s.is_empty()).collect();
        // ["r", sub, "comments", id, ...]
        if let Some(i) = parts.iter().position(|s| *s == "comments") {
            if let Some(id) = parts.get(i + 1) {
                if !id.is_empty() {
                    return (*id).to_string();
                }
            }
        }
    }
    String::new()
}

/// Title patterns Reddit uses for removed/deleted listings.
fn title_looks_removed(title: &str) -> bool {
    let tl = title.trim().to_ascii_lowercase();
    if tl.is_empty() {
        return false;
    }
    matches!(
        tl.as_str(),
        "[deleted]" | "[removed]" | "deleted" | "removed" | "[removed by reddit]"
    ) || tl.starts_with("[deleted")
        || tl.starts_with("[removed")
        || tl.contains("removed by moderator")
        || tl.contains("removed by reddit")
        || tl.contains("removed by a moderator")
        || tl == "[unavailable]"
}

/// True if archive metadata says the Reddit post page is gone (CDN may still serve media).
fn post_is_unavailable(p: &PostData) -> bool {
    // Arctic/Reddit mark removed posts this way (media CDN may still serve files).
    if p.removed_by_category.is_some() {
        return true;
    }
    // STRICT: must be explicitly still public. Missing field = treat as unknown/unavailable
    // until re-verified (Pullpush often omits it; those posts are re-checked via Arctic ids).
    if p.is_robot_indexable != Some(true) {
        return true;
    }
    if let Some(title) = p.title.as_deref() {
        if title_looks_removed(title) {
            return true;
        }
    }
    if let Some(st) = p.selftext.as_deref().map(str::trim) {
        if matches!(st, "[deleted]" | "[removed]") || title_looks_removed(st) {
            return true;
        }
    }
    if let Some(author) = p.author.as_deref() {
        if author.eq_ignore_ascii_case("[deleted]") || author.eq_ignore_ascii_case("[removed]") {
            return true;
        }
    }
    if let Some(url) = p.url.as_deref() {
        if url_looks_deleted(url) {
            return true;
        }
    }
    false
}

/// Known-dead from raw JSON (safe to drop without re-fetch).
fn json_post_is_known_dead(p: &serde_json::Value) -> bool {
    match p.get("removed_by_category") {
        Some(v) if !v.is_null() => return true,
        _ => {}
    }
    if p.get("is_robot_indexable").and_then(|v| v.as_bool()) == Some(false) {
        return true;
    }
    if let Some(title) = p.get("title").and_then(|t| t.as_str()) {
        if title_looks_removed(title) {
            return true;
        }
    }
    if let Some(author) = p.get("author").and_then(|a| a.as_str()) {
        if author.eq_ignore_ascii_case("[deleted]") || author.eq_ignore_ascii_case("[removed]") {
            return true;
        }
    }
    false
}

/// Live-check fields on a raw JSON post (pre-serde). Requires explicit robot=true.
fn json_post_is_public(p: &serde_json::Value) -> bool {
    if json_post_is_known_dead(p) {
        return false;
    }
    // Must be explicitly still indexable (CDN can outlive the post page).
    p.get("is_robot_indexable").and_then(|v| v.as_bool()) == Some(true)
}

fn json_post_id(p: &serde_json::Value) -> Option<String> {
    if let Some(id) = p.get("id").and_then(|v| v.as_str()).map(str::trim).filter(|s| !s.is_empty()) {
        return Some(id.strip_prefix("t3_").unwrap_or(id).to_string());
    }
    if let Some(pl) = p.get("permalink").and_then(|v| v.as_str()) {
        let parts: Vec<&str> = pl.split('/').filter(|s| !s.is_empty()).collect();
        if let Some(i) = parts.iter().position(|s| *s == "comments") {
            if let Some(id) = parts.get(i + 1).copied().filter(|s| !s.is_empty()) {
                return Some(id.to_string());
            }
        }
    }
    None
}

fn json_post_score(p: &serde_json::Value) -> i64 {
    p.get("score")
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
        .unwrap_or(0)
}

fn url_looks_deleted(url: &str) -> bool {
    let u = url.to_ascii_lowercase();
    u.contains("redditstatic.com")
        || u.contains("default_avatar")
        || u.contains("/removed")
        || u.contains("emoji.")
        || u.ends_with("/null")
        || u.contains("style_emote")
}

fn is_video_post(p: &PostData) -> bool {
    if p.is_video.unwrap_or(false) {
        return true;
    }
    match p.post_hint.as_deref() {
        Some("hosted:video") | Some("rich:video") => return true,
        _ => {}
    }
    p.url
        .as_deref()
        .map(|u| {
            let u = u.to_ascii_lowercase();
            u.contains("v.redd.it") || u.contains("youtube.com") || u.contains("youtu.be")
        })
        .unwrap_or(false)
}

fn collect_post_media(p: &PostData) -> Vec<MediaItem> {
    // Video posts: ONLY if we have a progressive MP4 we can actually play.
    // Never fall back to a still/poster — user asked not to fetch unusable video stills.
    if is_video_post(p) {
        return match playable_reddit_video(p) {
            Some(item) => vec![item],
            None => Vec::new(), // skip gfycat/youtube/broken video entirely
        };
    }

    // Image galleries (skip pure video entries inside)
    if p.is_gallery.unwrap_or(false) || p.gallery_data.is_some() {
        if let Some(items) = gallery_items(p) {
            if !items.is_empty() {
                return items;
            }
        }
        return Vec::new();
    }

    // Direct still images only (or easily playable imgur mp4 from gifv)
    if let Some(url) = &p.url {
        if let Some(item) = media_from_url(url) {
            return vec![item];
        }
    }

    // Preview still only for non-video image posts
    if let Some(url) = preview_source_url(p) {
        if is_image_url(&url) && !url_looks_deleted(&url) {
            return vec![MediaItem {
                url: decode_url(&url),
                kind: MediaKind::Image,
                poster: None,
            }];
        }
    }

    Vec::new()
}

/// Only progressive MP4 fallbacks from Reddit-hosted video (autoplay-friendly).
fn playable_reddit_video(p: &PostData) -> Option<MediaItem> {
    let rv = p
        .media
        .as_ref()
        .and_then(|m| m.reddit_video.as_ref())
        .or_else(|| p.secure_media.as_ref().and_then(|m| m.reddit_video.as_ref()))?;
    let url = rv.fallback_url.as_ref()?;
    let url = decode_url(url);
    if !url_is_playable_mp4(&url) {
        return None;
    }
    Some(MediaItem {
        url,
        kind: MediaKind::Video,
        // Poster optional for <video poster=> only — never used as standalone image.
        poster: poster_from_preview(p).filter(|u| is_image_url(u)),
    })
}

fn url_is_playable_mp4(url: &str) -> bool {
    let u = url.to_ascii_lowercase();
    // Reddit DASH progressive fallbacks and imgur mp4
    if u.contains("youtube") || u.contains("youtu.be") || u.contains("gfycat") || u.contains("redgifs")
    {
        return false;
    }
    u.contains(".mp4")
        || u.contains("dash_")
        || u.contains("/dash")
        || u.ends_with("mp4")
        || (u.contains("v.redd.it") && u.contains("dash"))
}

fn gallery_items(p: &PostData) -> Option<Vec<MediaItem>> {
    let items = p.gallery_data.as_ref()?.items.as_ref()?;
    let meta = p.media_metadata.as_ref()?;
    let mut out = Vec::new();
    for item in items {
        let id = item.media_id.as_ref()?;
        let m = meta.get(id)?;
        // Reddit marks deleted gallery files as failed / invalid.
        if let Some(status) = m.status.as_deref() {
            if status != "valid" {
                continue;
            }
        }
        if m.e.as_deref() == Some("RedditVideo") {
            // Rare in galleries; skip without a clean mp4 field
            continue;
        }
        let src = m.s.as_ref()?;
        if let Some(mp4) = &src.mp4 {
            let mp4 = decode_url(mp4);
            if url_is_playable_mp4(&mp4) {
                out.push(MediaItem {
                    url: mp4,
                    kind: MediaKind::Video,
                    poster: None,
                });
            }
            continue;
        }
        let u = src.u.as_ref().or(src.gif.as_ref())?;
        let u = decode_url(u);
        if is_image_url(&u) || u.contains("preview.redd.it") || u.contains("i.redd.it") {
            out.push(MediaItem {
                url: u,
                kind: MediaKind::Image,
                poster: None,
            });
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn media_from_url(url: &str) -> Option<MediaItem> {
    let url = decode_url(url);
    if url_looks_deleted(&url) {
        return None;
    }
    let lower = url.to_ascii_lowercase();

    // Imgur gifv/gif → progressive mp4 (easy to play muted/loop)
    if lower.contains("imgur.com/") {
        if let Some(base) = url
            .strip_suffix(".gifv")
            .or_else(|| url.strip_suffix(".GIFV"))
            .or_else(|| url.strip_suffix(".gif"))
            .or_else(|| url.strip_suffix(".GIF"))
        {
            let mp4 = format!("{base}.mp4");
            if url_is_playable_mp4(&mp4) {
                return Some(MediaItem {
                    url: mp4,
                    kind: MediaKind::Video,
                    poster: None,
                });
            }
            return None;
        }
    }

    // Never treat bare v.redd.it as an image; only playable via reddit_video fallback.
    if lower.contains("v.redd.it") {
        return None;
    }

    if is_image_url(&url) {
        return Some(MediaItem {
            url,
            kind: MediaKind::Image,
            poster: None,
        });
    }

    None
}

fn poster_from_preview(p: &PostData) -> Option<String> {
    preview_source_url(p).map(|u| decode_url(&u))
}

fn preview_source_url(p: &PostData) -> Option<String> {
    p.preview
        .as_ref()
        .and_then(|pr| pr.images.as_ref())
        .and_then(|imgs| imgs.first())
        .and_then(|img| img.source.as_ref())
        .and_then(|s| s.url.clone())
}

fn is_image_url(url: &str) -> bool {
    let u = url.split('?').next().unwrap_or(url).to_ascii_lowercase();
    if u.contains("v.redd.it") || u.contains("youtube.com") || u.contains("youtu.be") {
        return false;
    }
    u.contains("i.redd.it")
        || u.contains("preview.redd.it")
        || u.contains("i.imgur.com")
        || (u.contains("imgur.com/")
            && (u.ends_with(".jpg")
                || u.ends_with(".jpeg")
                || u.ends_with(".png")
                || u.ends_with(".webp")
                || u.ends_with(".gif")))
        || u.ends_with(".jpg")
        || u.ends_with(".jpeg")
        || u.ends_with(".png")
        || u.ends_with(".webp")
        || u.ends_with(".gif")
}

fn decode_url(url: &str) -> String {
    url.replace("&amp;", "&")
}

#[allow(dead_code)]
pub fn pick_random_image(images: &[RedditMedia], avoid_urls: &[String]) -> Option<RedditMedia> {
    if images.is_empty() {
        return None;
    }
    let fresh: Vec<&RedditMedia> = images
        .iter()
        .filter(|img| !avoid_urls.iter().any(|u| u == img.primary_url()))
        .collect();
    let pool = if fresh.is_empty() {
        images.iter().collect::<Vec<_>>()
    } else {
        fresh
    };
    Some(pool[fastrand::usize(..pool.len())].clone())
}

/// Warm browser cache for images (best-effort).
#[cfg(target_arch = "wasm32")]
pub fn warm_media_cache(media: &RedditMedia) {
    use wasm_bindgen::JsCast;
    for item in &media.items {
        match item.kind {
            MediaKind::Image => {
                if let Ok(img) = web_sys::HtmlImageElement::new() {
                    img.set_src(&item.url);
                }
            }
            MediaKind::Video => {
                // Create a video element with preload=auto (not attached).
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    if let Ok(el) = doc.create_element("video") {
                        if let Ok(vid) = el.dyn_into::<web_sys::HtmlVideoElement>() {
                            vid.set_preload("auto");
                            vid.set_muted(true);
                            vid.set_src(&item.url);
                            let _ = vid.load();
                        }
                    }
                }
                if let Some(poster) = &item.poster {
                    if let Ok(img) = web_sys::HtmlImageElement::new() {
                        img.set_src(poster);
                    }
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn warm_media_cache(_media: &RedditMedia) {}

#[cfg(target_arch = "wasm32")]
fn now_secs() -> u64 {
    (js_sys::Date::now() / 1000.0) as u64
}

#[cfg(target_arch = "wasm32")]
fn now_ms() -> f64 {
    js_sys::Date::now()
}

#[cfg(target_arch = "wasm32")]
fn iso_utc_days_ago(days: i64) -> String {
    let ms = js_sys::Date::now() - (days as f64) * 86_400_000.0;
    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(ms));
    format!(
        "{:04}-{:02}-{:02}",
        d.get_utc_full_year() as i32,
        d.get_utc_month() as u32 + 1,
        d.get_utc_date() as u32
    )
}

#[cfg(target_arch = "wasm32")]
fn iso_utc_from_unix(secs: f64) -> String {
    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(secs * 1000.0));
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        d.get_utc_full_year() as i32,
        d.get_utc_month() as u32 + 1,
        d.get_utc_date() as u32,
        d.get_utc_hours() as u32,
        d.get_utc_minutes() as u32,
        d.get_utc_seconds() as u32
    )
}

/// How many highest-scoring public posts we randomize among (true “top” pool).
const TOP_POOL_SIZE: usize = 40;

#[cfg(target_arch = "wasm32")]
thread_local! {
    static LAST_API_MS: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
    static BACKOFF_UNTIL_MS: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
    /// Active AbortController — cancelled when the user switches sub mid-fetch.
    static ACTIVE_ABORT: std::cell::RefCell<Option<web_sys::AbortController>> =
        const { std::cell::RefCell::new(None) };
}

/// Cancel in-flight archive HTTP requests (Network panel + work stops).
#[cfg(target_arch = "wasm32")]
pub fn abort_active_fetches() {
    ACTIVE_ABORT.with(|c| {
        if let Some(ctrl) = c.borrow_mut().take() {
            ctrl.abort();
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn abort_active_fetches() {}

/// Replace the active abort controller and return its signal for a new request wave.
#[cfg(target_arch = "wasm32")]
fn take_abort_signal() -> Option<web_sys::AbortSignal> {
    let ctrl = web_sys::AbortController::new().ok()?;
    let signal = ctrl.signal();
    ACTIVE_ABORT.with(|c| {
        if let Some(old) = c.borrow_mut().replace(ctrl) {
            old.abort();
        }
    });
    Some(signal)
}

#[cfg(target_arch = "wasm32")]
async fn sleep_ms(ms: i32) {
    use js_sys::Promise;
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;

    let promise = Promise::new(&mut |resolve, _reject| {
        let cb = Closure::once(move || {
            let _ = resolve.call0(&JsValue::NULL);
        });
        let _ = web_sys::window().map(|w| {
            w.set_timeout_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), ms)
        });
        cb.forget();
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

#[cfg(target_arch = "wasm32")]
async fn wait_rate_limit(min_gap_ms: f64) {
    let now = now_ms();
    let backoff = BACKOFF_UNTIL_MS.with(|c| c.get());
    if now < backoff {
        sleep_ms((backoff - now).ceil() as i32).await;
    }
    let last = LAST_API_MS.with(|c| c.get());
    let wait = min_gap_ms - (now_ms() - last);
    if wait > 0.0 {
        sleep_ms(wait.ceil() as i32).await;
    }
    LAST_API_MS.with(|c| c.set(now_ms()));
}

#[cfg(target_arch = "wasm32")]
async fn fetch_text_with_opts(
    url: &str,
    min_gap_ms: f64,
    signal: Option<&web_sys::AbortSignal>,
) -> Result<String, RedditError> {
    wait_rate_limit(min_gap_ms).await;

    let mut req = gloo_net::http::Request::get(url).header("Accept", "application/json");
    if let Some(sig) = signal {
        req = req.abort_signal(Some(sig));
    }
    let resp = req
        .send()
        .await
        .map_err(|e| RedditError::Network(e.to_string()))?;

    if resp.status() == 429 {
        BACKOFF_UNTIL_MS.with(|c| c.set(now_ms() + 25_000.0));
        return Err(RedditError::Network(
            "Rate limited (HTTP 429). Wait ~25s, then try Load again."
                .into(),
        ));
    }
    if resp.status() == 403 {
        return Err(RedditError::Network(
            "HTTP 403 forbidden by source — trying another API…".into(),
        ));
    }
    if !resp.ok() {
        return Err(RedditError::Network(format!("HTTP {}", resp.status())));
    }
    resp.text()
        .await
        .map_err(|e| RedditError::Network(e.to_string()))
}

/// Fetch posts from Arctic Shift (CORS-friendly). Ranks by score client-side
/// (API only supports time sort asc/desc).
#[cfg(target_arch = "wasm32")]
async fn fetch_arctic_window(
    subreddit: &str,
    after_days: Option<i64>,
    max_pages: usize,
    soft_min_score: i64,
) -> Result<Vec<serde_json::Value>, RedditError> {
    let after = after_days.map(iso_utc_days_ago);
    let mut before = iso_utc_days_ago(-1); // tomorrow (inclusive upper bound)
    let mut all: Vec<serde_json::Value> = Vec::new();
    let mut seen_ids = HashSet::new();

    for page in 0..max_pages {
        let mut url = format!(
            "https://arctic-shift.photon-reddit.com/api/posts/search?subreddit={subreddit}&limit=100&sort=desc"
        );
        if let Some(ref a) = after {
            url.push_str(&format!("&after={a}"));
        }
        url.push_str(&format!("&before={before}"));

        let text = fetch_text_with_opts(&url, 700.0, None).await?;
        let v: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| RedditError::Parse(e.to_string()))?;
        let Some(arr) = v.get("data").and_then(|d| d.as_array()) else {
            return Err(RedditError::Parse("arctic missing data".into()));
        };
        if arr.is_empty() {
            break;
        }
        let mut oldest = f64::MAX;
        for p in arr {
            if let Some(c) = p.get("created_utc").and_then(|x| x.as_f64()) {
                oldest = oldest.min(c);
            }
            if let Some(id) = json_post_id(p) {
                if !seen_ids.insert(id) {
                    continue;
                }
            }
            // Drop known-removed early to keep memory/scoring clean.
            if json_post_is_public(p) {
                all.push(p.clone());
            }
        }
        // Single-page mode is the common path (max_pages == 1).
        if max_pages <= 1 {
            break;
        }
        // Early stop once we have a solid high-score public pool.
        let high = all
            .iter()
            .filter(|p| json_post_score(p) >= soft_min_score)
            .count();
        if high >= TOP_POOL_SIZE && page + 1 >= 2 {
            break;
        }
        if oldest == f64::MAX || arr.len() < 50 {
            break;
        }
        let next_before = iso_utc_from_unix(oldest);
        if next_before == before {
            break;
        }
        before = next_before;
    }

    // Rank by score descending (client-side true “top of sampled window”)
    all.sort_by(|a, b| json_post_score(b).cmp(&json_post_score(a)));
    Ok(all)
}

/// Pullpush score-sorted (true top-of-window). Often 429 — use sparingly.
#[cfg(target_arch = "wasm32")]
async fn fetch_pullpush_window(
    subreddit: &str,
    since_days: Option<i64>,
) -> Result<Vec<serde_json::Value>, RedditError> {
    let now = now_secs();
    let mut url = format!(
        "https://api.pullpush.io/reddit/search/submission/?subreddit={subreddit}&sort=desc&sort_type=score&size=100"
    );
    if let Some(days) = since_days {
        let since = now.saturating_sub((days as u64).saturating_mul(86_400));
        url.push_str(&format!("&since={since}"));
    }
    let text = fetch_text_with_opts(&url, 3_500.0, None).await?;
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| RedditError::Parse(e.to_string()))?;
    let mut arr = v
        .get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();
    // Drop known-dead; missing robot field is OK until Arctic ids re-verify.
    arr.retain(|p| !json_post_is_known_dead(p));
    Ok(arr)
}

fn posts_json_to_media(posts: Vec<serde_json::Value>, subreddit: &str) -> Result<Vec<RedditMedia>, RedditError> {
    // Reuse extract_images by wrapping as { "data": posts }
    let wrapped = serde_json::json!({ "data": posts });
    extract_images(&wrapped.to_string(), subreddit)
}

/// True if any URL in this post was already used this session.
pub fn media_seen_in_session(media: &RedditMedia, seen: &[String]) -> bool {
    media
        .items
        .iter()
        .any(|item| seen.iter().any(|s| s == &item.url))
        || (!media.id.is_empty() && seen.iter().any(|s| s == &media.id || s.ends_with(&format!("/{}", media.id))))
}

pub async fn load_random_image(
    raw_sub: &str,
    avoid_urls: &[String],
) -> Result<(RedditMedia, &'static str), RedditError> {
    let mut batch = load_media_batch(raw_sub, avoid_urls, 1).await?;
    batch
        .pop()
        .ok_or(RedditError::NoImages)
}

/// Fetch up to `count` ready posts with **minimal API calls**:
/// ideally 1 listing request + 1 batch id re-verify, then local media probes.
/// Used to fill the prefetch queue without N separate full searches.
pub async fn load_media_batch(
    raw_sub: &str,
    avoid_urls: &[String],
    count: usize,
) -> Result<Vec<(RedditMedia, &'static str)>, RedditError> {
    let sub = normalize_subreddit(raw_sub).ok_or(RedditError::InvalidSubreddit)?;
    if count == 0 {
        return Ok(Vec::new());
    }
    load_media_batch_for_sub(&sub, avoid_urls, count).await
}

#[cfg(target_arch = "wasm32")]
async fn load_media_batch_for_sub(
    sub: &str,
    avoid_urls: &[String],
    count: usize,
) -> Result<Vec<(RedditMedia, &'static str)>, RedditError> {
    // New wave of requests — abort anything still running from a skipped sub.
    let _signal = take_abort_signal();
    let mut last_err = RedditError::NoImages;

    // Fast path: one Arctic page for the last week (soft score floor 0 = take anything usable).
    match fetch_arctic_window(sub, Some(7), 1, 0).await {
        Ok(posts) => {
            match try_pick_batch_from_posts(posts, sub, avoid_urls, 0, count).await {
                Ok(media) if !media.is_empty() => {
                    return Ok(media.into_iter().map(|m| (m, "top week")).collect());
                }
                Ok(_) => last_err = RedditError::NoImages,
                Err(e) => last_err = e,
            }
        }
        Err(e) => last_err = e,
    }

    // One fallback only if week listing empty/unusable.
    match fetch_arctic_window(sub, None, 1, 0).await {
        Ok(posts) => {
            match try_pick_batch_from_posts(posts, sub, avoid_urls, 0, count).await {
                Ok(media) if !media.is_empty() => {
                    return Ok(media.into_iter().map(|m| (m, "recent")).collect());
                }
                Ok(_) => last_err = RedditError::NoImages,
                Err(e) => last_err = e,
            }
        }
        Err(e) => last_err = e,
    }

    Err(last_err)
}

#[cfg(not(target_arch = "wasm32"))]
async fn load_media_batch_for_sub(
    _sub: &str,
    _avoid_urls: &[String],
    _count: usize,
) -> Result<Vec<(RedditMedia, &'static str)>, RedditError> {
    Err(RedditError::Network("browser only".into()))
}

/// Fast pick from one listing: trust listing metadata + non-tombstone URLs.
/// **No CDN probes, no second id re-verify** — prioritizes idle time over perfect accuracy.
async fn try_pick_batch_from_posts(
    posts: Vec<serde_json::Value>,
    sub: &str,
    avoid_urls: &[String],
    soft_min_score: i64,
    max_n: usize,
) -> Result<Vec<RedditMedia>, RedditError> {
    // Prefer explicitly public posts; fall back to “not known dead” if needed.
    let mut public: Vec<serde_json::Value> = posts
        .iter()
        .filter(|p| json_post_is_public(p))
        .cloned()
        .collect();
    if public.len() < max_n {
        for p in &posts {
            if json_post_is_known_dead(p) || json_post_id(p).is_none() {
                continue;
            }
            if public.iter().any(|q| json_post_id(q) == json_post_id(p)) {
                continue;
            }
            public.push(p.clone());
        }
    }
    if public.is_empty() {
        return Err(RedditError::NoImages);
    }
    public.sort_by(|a, b| json_post_score(b).cmp(&json_post_score(a)));

    let high: Vec<serde_json::Value> = public
        .iter()
        .filter(|p| json_post_score(p) >= soft_min_score)
        .cloned()
        .collect();
    let ranked = if high.len() >= max_n {
        high
    } else {
        public
    };

    let top: Vec<serde_json::Value> = ranked.into_iter().take(TOP_POOL_SIZE).collect();
    let mut media_list = posts_json_to_media(top, sub)?;
    media_list.retain(|m| !media_seen_in_session(m, avoid_urls) && !m.id.is_empty());
    for m in &mut media_list {
        m.items.retain(|it| !url_looks_deleted(&it.url));
    }
    media_list.retain(|m| !m.items.is_empty());
    if media_list.is_empty() {
        return Err(RedditError::NoImages);
    }

    // Light shuffle among the first few for variety (still score-leaning).
    let shuffle_n = media_list.len().min(max_n.saturating_mul(2).max(4));
    for i in (1..shuffle_n).rev() {
        let j = fastrand::usize(..=i);
        media_list.swap(i, j);
    }

    // Take up to max_n immediately — zero extra network.
    let out: Vec<RedditMedia> = media_list.into_iter().take(max_n).collect();
    if out.is_empty() {
        Err(RedditError::NoImages)
    } else {
        Ok(out)
    }
}

/// Batch re-fetch posts by id; return only ids that are still public on Reddit.
#[cfg(target_arch = "wasm32")]
async fn reverify_public_ids(ids: &[String]) -> HashSet<String> {
    let mut out = HashSet::new();
    if ids.is_empty() {
        return out;
    }
    // One signal for this re-verify wave (shares ACTIVE_ABORT with listing if same wave).
    let signal = ACTIVE_ABORT.with(|c| c.borrow().as_ref().map(|ctrl| ctrl.signal()));
    for chunk in ids.chunks(25) {
        let joined = chunk.join(",");
        let url = format!("https://arctic-shift.photon-reddit.com/api/posts/ids?ids={joined}");
        let Ok(text) = fetch_text_with_opts(&url, 500.0, signal.as_ref()).await else {
            // Conservative: if re-check fails, accept none of this chunk.
            continue;
        };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        let Some(arr) = v.get("data").and_then(|d| d.as_array()) else {
            continue;
        };
        for p in arr {
            if json_post_is_public(p) {
                if let Some(id) = json_post_id(p) {
                    out.insert(id);
                }
            }
        }
    }
    out
}

#[cfg(not(target_arch = "wasm32"))]
async fn reverify_public_ids(ids: &[String]) -> HashSet<String> {
    ids.iter().cloned().collect()
}

/// Re-fetch one post by id; true only if still public on the archive.
/// Used before accepting media so we don't link deleted Reddit posts (CDN may still serve files).
pub async fn post_is_still_public(id: &str) -> bool {
    post_still_public(id).await
}

#[cfg(target_arch = "wasm32")]
async fn post_still_public(id: &str) -> bool {
    if id.is_empty() {
        return false;
    }
    let set = reverify_public_ids(&[id.to_string()]).await;
    set.contains(id)
}

#[cfg(not(target_arch = "wasm32"))]
async fn post_still_public(id: &str) -> bool {
    !id.is_empty()
}

/// Keep only media that still loads (filters Reddit-deleted files).
pub async fn filter_live_media(media: RedditMedia) -> Option<RedditMedia> {
    let mut live = Vec::with_capacity(media.items.len());
    for item in media.items {
        if url_looks_deleted(&item.url) {
            continue;
        }
        if media_item_is_available(&item).await {
            live.push(item);
        }
    }
    if live.is_empty() {
        None
    } else {
        Some(RedditMedia {
            items: live,
            title: media.title,
            permalink: media.permalink,
            subreddit: media.subreddit,
            id: media.id,
            score: media.score,
        })
    }
}

#[cfg(target_arch = "wasm32")]
async fn media_item_is_available(item: &MediaItem) -> bool {
    // One CDN load only (element probe). A separate HTTP GET would double traffic.
    match item.kind {
        MediaKind::Image => probe_image(&item.url).await,
        MediaKind::Video => probe_video(&item.url).await,
    }
}

/// Returns Some(true/false) if we got a readable HTTP status; None if CORS blocked.
#[cfg(target_arch = "wasm32")]
async fn http_url_ok(url: &str) -> Option<bool> {
    match gloo_net::http::Request::get(url).send().await {
        Ok(resp) => {
            let status = resp.status();
            // 2xx/3xx ok; 404/410 gone; 403 often deleted/forbidden media
            if status == 404 || status == 410 || status == 451 {
                return Some(false);
            }
            if (200..400).contains(&status) {
                return Some(true);
            }
            // Other statuses (403 etc.) — still try element probe (CDN quirks)
            None
        }
        Err(_) => None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn media_item_is_available(_item: &MediaItem) -> bool {
    true
}

#[cfg(target_arch = "wasm32")]
async fn probe_image(url: &str) -> bool {
    use js_sys::Promise;
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;

    let Ok(img) = web_sys::HtmlImageElement::new() else {
        return false;
    };
    // Don't set crossOrigin: many CDNs fail CORS on decode check; load/error still works.
    let url = url.to_string();
    let promise = Promise::new(&mut |resolve, _reject| {
        let img_ok = img.clone();
        let resolve_ok = resolve.clone();
        let onload = Closure::once(move || {
            let w = img_ok.natural_width();
            let h = img_ok.natural_height();
            // Reject broken/tiny placeholders; real posts are much larger.
            let ok = w >= 64 && h >= 64;
            let _ = resolve_ok.call1(&JsValue::NULL, &JsValue::from_bool(ok));
        });
        let resolve_err = resolve.clone();
        let onerror = Closure::once(move || {
            let _ = resolve_err.call1(&JsValue::NULL, &JsValue::from_bool(false));
        });
        img.set_onload(Some(onload.as_ref().unchecked_ref()));
        img.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        img.set_src(&url);
        onload.forget();
        onerror.forget();
    });
    // Race with timeout so we don't hang forever
    let raced = promise_race_timeout(promise, 8_000);
    match wasm_bindgen_futures::JsFuture::from(raced).await {
        Ok(v) => v.as_bool().unwrap_or(false),
        Err(_) => false,
    }
}

#[cfg(target_arch = "wasm32")]
async fn probe_video(url: &str) -> bool {
    use js_sys::Promise;
    use std::cell::Cell;
    use std::rc::Rc;
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;

    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return false;
    };
    let Ok(el) = doc.create_element("video") else {
        return false;
    };
    let Ok(vid) = el.dyn_into::<web_sys::HtmlVideoElement>() else {
        return false;
    };
    vid.set_preload("metadata");
    vid.set_muted(true);
    let url = url.to_string();
    let promise = Promise::new(&mut |resolve, _reject| {
        let done = Rc::new(Cell::new(false));
        let resolve = Rc::new(resolve);
        let finish = {
            let done = done.clone();
            let resolve = resolve.clone();
            Rc::new(move |ok: bool| {
                if done.replace(true) {
                    return;
                }
                let _ = resolve.call1(&JsValue::NULL, &JsValue::from_bool(ok));
            }) as Rc<dyn Fn(bool)>
        };
        let f1 = finish.clone();
        let vid_ok = vid.clone();
        let onok = Closure::wrap(Box::new(move || {
            let ok = vid_ok.video_width() >= 32 && vid_ok.video_height() >= 32;
            f1(ok);
        }) as Box<dyn FnMut()>);
        let f2 = finish.clone();
        let onerror = Closure::wrap(Box::new(move || f2(false)) as Box<dyn FnMut()>);
        let _ = vid.add_event_listener_with_callback("loadedmetadata", onok.as_ref().unchecked_ref());
        let _ = vid.add_event_listener_with_callback("error", onerror.as_ref().unchecked_ref());
        vid.set_src(&url);
        let _ = vid.load();
        onok.forget();
        onerror.forget();
    });
    let raced = promise_race_timeout(promise, 10_000);
    match wasm_bindgen_futures::JsFuture::from(raced).await {
        Ok(v) => v.as_bool().unwrap_or(false),
        Err(_) => false,
    }
}

#[cfg(target_arch = "wasm32")]
fn promise_race_timeout(promise: js_sys::Promise, ms: i32) -> js_sys::Promise {
    use js_sys::Promise;
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;

    let timeout = Promise::new(&mut |resolve, _reject| {
        let resolve = resolve.clone();
        let cb = Closure::once(move || {
            let _ = resolve.call1(&JsValue::NULL, &JsValue::from_bool(false));
        });
        let _ = web_sys::window().map(|w| {
            w.set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                ms,
            )
        });
        cb.forget();
    });
    // Promise.race([probe, timeout])
    let arr = js_sys::Array::new();
    arr.push(&promise);
    arr.push(&timeout);
    Promise::race(&arr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_names_and_urls() {
        assert_eq!(normalize_subreddit("pics").as_deref(), Some("pics"));
        assert_eq!(
            normalize_subreddit("https://www.reddit.com/r/FiftyFifty/").as_deref(),
            Some("FiftyFifty")
        );
    }

    #[test]
    fn extract_image_gallery_video() {
        let json = r#"{
          "data": [
            {
              "id": "aaa111",
              "title": "Single",
              "url": "https://i.redd.it/top.jpg",
              "permalink": "/r/pics/comments/aaa111/top/",
              "score": 500,
              "is_robot_indexable": true,
              "is_video": false,
              "post_hint": "image"
            },
            {
              "id": "bbb222",
              "title": "Vid",
              "url": "https://v.redd.it/abc",
              "permalink": "/r/pics/comments/bbb222/v/",
              "score": 400,
              "is_robot_indexable": true,
              "is_video": true,
              "media": {
                "reddit_video": {
                  "fallback_url": "https://v.redd.it/abc/DASH_480.mp4?source=fallback",
                  "is_gif": true
                }
              },
              "preview": {
                "images": [{ "source": { "url": "https://preview.redd.it/poster.jpg" } }]
              }
            },
            {
              "id": "ccc333",
              "title": "Gallery",
              "url": "https://www.reddit.com/gallery/xyz",
              "permalink": "/r/pics/comments/ccc333/g/",
              "score": 300,
              "is_robot_indexable": true,
              "is_gallery": true,
              "gallery_data": { "items": [ { "media_id": "aaa" }, { "media_id": "bbb" } ] },
              "media_metadata": {
                "aaa": { "e": "Image", "s": { "u": "https://preview.redd.it/aaa.jpg?width=100" } },
                "bbb": { "e": "Image", "s": { "u": "https://preview.redd.it/bbb.jpg?width=100" } }
              }
            }
          ]
        }"#;
        let items = extract_images(json, "pics").unwrap();
        assert_eq!(items.len(), 3);
        // Score-ranked: 500, 400, 300
        assert_eq!(items[0].items[0].kind, MediaKind::Image);
        assert_eq!(items[0].id, "aaa111");
        assert_eq!(items[0].score, 500);
        assert_eq!(items[1].items[0].kind, MediaKind::Video);
        assert!(items[1].items[0].url.contains("DASH_480"));
        assert_eq!(items[2].items.len(), 2);
    }

    #[test]
    fn imgur_gifv_to_mp4() {
        let item = media_from_url("https://i.imgur.com/ZenTaxR.gifv").unwrap();
        assert_eq!(item.kind, MediaKind::Video);
        assert_eq!(item.url, "https://i.imgur.com/ZenTaxR.mp4");
    }

    #[test]
    fn skips_removed_posts() {
        let json = r#"{
          "data": [
            {
              "id": "dead1",
              "title": "[deleted]",
              "url": "https://i.redd.it/x.jpg",
              "permalink": "/r/pics/comments/dead1/",
              "author": "[deleted]",
              "removed_by_category": "moderator",
              "is_robot_indexable": false,
              "score": 9999
            },
            {
              "id": "live2",
              "title": "Alive",
              "url": "https://i.redd.it/ok.jpg",
              "permalink": "/r/pics/comments/live2/",
              "author": "someone",
              "post_hint": "image",
              "is_robot_indexable": true,
              "score": 10
            }
          ]
        }"#;
        let items = extract_images(json, "pics").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].primary_url(), "https://i.redd.it/ok.jpg");
        assert_eq!(items[0].id, "live2");
    }

    #[test]
    fn skips_not_robot_indexable() {
        let json = r#"{
          "data": [
            {
              "id": "gone",
              "title": "Removed but CDN has file",
              "url": "https://i.redd.it/x.jpg",
              "permalink": "/r/pics/comments/gone/",
              "author": "someone",
              "removed_by_category": "moderator",
              "is_robot_indexable": false,
              "post_hint": "image",
              "score": 5000
            },
            {
              "id": "live",
              "title": "Live",
              "url": "https://i.redd.it/ok.jpg",
              "permalink": "/r/pics/comments/live/",
              "author": "someone",
              "is_robot_indexable": true,
              "post_hint": "image",
              "score": 100
            }
          ]
        }"#;
        let items = extract_images(json, "pics").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].primary_url(), "https://i.redd.it/ok.jpg");
    }

    #[test]
    fn skips_video_without_playable_mp4() {
        let json = r#"{
          "data": [
            {
              "id": "yt1",
              "title": "YT",
              "url": "https://youtube.com/watch?v=abc",
              "permalink": "/r/pics/comments/yt1/",
              "is_video": false,
              "post_hint": "rich:video",
              "is_robot_indexable": true
            },
            {
              "id": "v1",
              "title": "v.redd.it no media object",
              "url": "https://v.redd.it/abc",
              "permalink": "/r/pics/comments/v1/",
              "is_video": true,
              "is_robot_indexable": true
            }
          ]
        }"#;
        let err = extract_images(json, "pics").unwrap_err();
        assert!(matches!(err, RedditError::NoImages));
    }

    #[test]
    fn ranks_by_score_descending() {
        let json = r#"{
          "data": [
            {
              "id": "low",
              "title": "Low",
              "url": "https://i.redd.it/low.jpg",
              "permalink": "/r/pics/comments/low/",
              "is_robot_indexable": true,
              "score": 3
            },
            {
              "id": "high",
              "title": "High",
              "url": "https://i.redd.it/high.jpg",
              "permalink": "/r/pics/comments/high/",
              "is_robot_indexable": true,
              "score": 9000
            }
          ]
        }"#;
        let items = extract_images(json, "pics").unwrap();
        assert_eq!(items[0].id, "high");
        assert_eq!(items[1].id, "low");
    }

    #[test]
    fn json_public_requires_robot_true() {
        let live = serde_json::json!({
            "id": "x",
            "title": "Ok",
            "is_robot_indexable": true,
            "removed_by_category": null
        });
        let dead = serde_json::json!({
            "id": "y",
            "title": "Nope",
            "is_robot_indexable": false,
            "removed_by_category": null
        });
        let removed = serde_json::json!({
            "id": "z",
            "title": "Nope",
            "is_robot_indexable": true,
            "removed_by_category": "moderator"
        });
        let missing_robot = serde_json::json!({
            "id": "m",
            "title": "Maybe",
            "removed_by_category": null
        });
        assert!(json_post_is_public(&live));
        assert!(!json_post_is_public(&dead));
        assert!(!json_post_is_public(&removed));
        assert!(!json_post_is_public(&missing_robot));
        assert!(!json_post_is_known_dead(&missing_robot)); // unknown, not known-dead
    }

    #[test]
    fn title_patterns_catch_mod_removed() {
        assert!(title_looks_removed("[ Removed by moderator ]"));
        assert!(title_looks_removed("[removed]"));
        assert!(title_looks_removed("[deleted by user]"));
        assert!(!title_looks_removed("Nice photo of a cat"));
    }

    #[test]
    fn skips_mod_removed_title_even_if_robot_missing_elsewhere() {
        let json = r#"{
          "data": [
            {
              "id": "gone",
              "title": "[ Removed by moderator ]",
              "url": "https://i.redd.it/x.jpg",
              "permalink": "/r/pics/comments/gone/",
              "is_robot_indexable": true,
              "score": 999
            }
          ]
        }"#;
        let err = extract_images(json, "pics").unwrap_err();
        assert!(matches!(err, RedditError::NoImages));
    }

}
