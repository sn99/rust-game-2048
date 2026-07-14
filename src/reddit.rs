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
            RedditError::Network(s) => write!(f, "Could not load media ({s})"),
            RedditError::NoImages => {
                write!(
                    f,
                    "No unused image/video left (week→day→month→year→all-time) — try another sub or reload the page"
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
    title: Option<String>,
    url: Option<String>,
    permalink: Option<String>,
    author: Option<String>,
    /// Set when mods/admins removed the post (or spam/etc.).
    removed_by_category: Option<String>,
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

        let permalink = p.permalink.unwrap_or_default();
        let permalink = if permalink.starts_with("http") {
            permalink
        } else {
            format!("https://www.reddit.com{permalink}")
        };
        out.push(RedditMedia {
            items,
            title: p.title.unwrap_or_default(),
            permalink,
            subreddit: subreddit.to_string(),
        });
    }
    if out.is_empty() {
        Err(RedditError::NoImages)
    } else {
        Ok(out)
    }
}


fn post_is_unavailable(p: &PostData) -> bool {
    if p.removed_by_category.is_some() {
        return true;
    }
    // Common tombstone titles from deleted/removed posts.
    if let Some(title) = p.title.as_deref().map(str::trim) {
        let tl = title.to_ascii_lowercase();
        if matches!(
            tl.as_str(),
            "[deleted]" | "[removed]" | "deleted" | "removed" | "[removed by reddit]"
        ) {
            return true;
        }
    }
    if let Some(author) = p.author.as_deref() {
        if author.eq_ignore_ascii_case("[deleted]") || author.eq_ignore_ascii_case("[removed]") {
            // Keep only if hosted media metadata still looks intact; otherwise skip.
            let has_video = p
                .media
                .as_ref()
                .and_then(|m| m.reddit_video.as_ref())
                .or_else(|| p.secure_media.as_ref().and_then(|m| m.reddit_video.as_ref()))
                .and_then(|v| v.fallback_url.as_ref())
                .is_some();
            let has_gallery = p
                .gallery_data
                .as_ref()
                .and_then(|g| g.items.as_ref())
                .map(|i| !i.is_empty())
                .unwrap_or(false);
            let url = p.url.as_deref().unwrap_or("");
            let has_direct = is_image_url(url) || url.contains("v.redd.it");
            if !has_video && !has_gallery && !has_direct {
                return true;
            }
        }
    }
    // Dead link patterns Reddit leaves behind.
    if let Some(url) = p.url.as_deref() {
        if url_looks_deleted(url) {
            return true;
        }
    }
    false
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

fn collect_post_media(p: &PostData) -> Vec<MediaItem> {
    // Gallery (images / animated)
    if p.is_gallery.unwrap_or(false) || p.gallery_data.is_some() {
        if let Some(items) = gallery_items(p) {
            if !items.is_empty() {
                return items;
            }
        }
    }

    // Hosted Reddit video
    if let Some(item) = reddit_video_item(p) {
        return vec![item];
    }

    // Direct image / gif / gifv→mp4
    if let Some(url) = &p.url {
        if let Some(item) = media_from_url(url, poster_from_preview(p)) {
            return vec![item];
        }
    }

    // Preview still as last resort
    if let Some(url) = preview_source_url(p) {
        if let Some(item) = media_from_url(&url, None) {
            return vec![item];
        }
    }

    Vec::new()
}

fn reddit_video_item(p: &PostData) -> Option<MediaItem> {
    let rv = p
        .media
        .as_ref()
        .and_then(|m| m.reddit_video.as_ref())
        .or_else(|| p.secure_media.as_ref().and_then(|m| m.reddit_video.as_ref()))?;
    let url = rv.fallback_url.as_ref()?;
    let url = decode_url(url);
    // Strip query for cleaner playback when possible; keep if needed
    Some(MediaItem {
        url,
        kind: MediaKind::Video,
        poster: poster_from_preview(p),
    })
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
            out.push(MediaItem {
                url: decode_url(mp4),
                kind: MediaKind::Video,
                poster: src.u.as_ref().map(|u| decode_url(u)),
            });
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

fn media_from_url(url: &str, poster: Option<String>) -> Option<MediaItem> {
    let url = decode_url(url);
    let lower = url.to_ascii_lowercase();

    // Imgur gifv/gif → mp4 when possible
    if lower.contains("imgur.com/") {
        if let Some(base) = url
            .strip_suffix(".gifv")
            .or_else(|| url.strip_suffix(".GIFV"))
            .or_else(|| url.strip_suffix(".gif"))
            .or_else(|| url.strip_suffix(".GIF"))
        {
            return Some(MediaItem {
                url: format!("{base}.mp4"),
                kind: MediaKind::Video,
                poster,
            });
        }
    }

    if lower.contains("v.redd.it") {
        // bare v.redd.it without media.reddit_video — skip
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
fn listing_endpoints(subreddit: &str) -> Vec<(String, &'static str)> {
    let now = now_secs();
    let day = now.saturating_sub(86_400);
    let week = now.saturating_sub(604_800);
    let month = now.saturating_sub(2_592_000);
    let year = now.saturating_sub(31_536_000);
    // Order: past week → past day → past month → past year → all-time.
    vec![
        (
            format!(
                "https://api.pullpush.io/reddit/search/submission/?subreddit={subreddit}&sort=desc&sort_type=score&size=100&since={week}"
            ),
            "top week",
        ),
        (
            format!(
                "https://api.pullpush.io/reddit/search/submission/?subreddit={subreddit}&sort=desc&sort_type=score&size=100&since={day}"
            ),
            "top day",
        ),
        (
            format!(
                "https://api.pullpush.io/reddit/search/submission/?subreddit={subreddit}&sort=desc&sort_type=score&size=100&since={month}"
            ),
            "top month",
        ),
        (
            format!(
                "https://api.pullpush.io/reddit/search/submission/?subreddit={subreddit}&sort=desc&sort_type=score&size=100&since={year}"
            ),
            "top year",
        ),
        (
            format!(
                "https://api.pullpush.io/reddit/search/submission/?subreddit={subreddit}&sort=desc&sort_type=score&size=100"
            ),
            "top all-time",
        ),
    ]
}

#[cfg(target_arch = "wasm32")]
async fn fetch_text(url: &str) -> Result<String, RedditError> {
    let resp = gloo_net::http::Request::get(url)
        .send()
        .await
        .map_err(|e| RedditError::Network(e.to_string()))?;
    if !resp.ok() {
        return Err(RedditError::Network(format!("HTTP {}", resp.status())));
    }
    resp.text()
        .await
        .map_err(|e| RedditError::Network(e.to_string()))
}

/// True if any URL in this post was already used this session.
pub fn media_seen_in_session(media: &RedditMedia, seen: &[String]) -> bool {
    media
        .items
        .iter()
        .any(|item| seen.iter().any(|s| s == &item.url))
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_images_with_fallback(
    subreddit: &str,
    avoid_urls: &[String],
) -> Result<(Vec<RedditMedia>, &'static str), RedditError> {
    let mut last_err = RedditError::NoImages;
    for (url, label) in listing_endpoints(subreddit) {
        match fetch_text(&url).await {
            Ok(text) if text.trim_start().starts_with('{') => match extract_images(&text, subreddit)
            {
                Ok(imgs) => {
                    // Only posts not already shown this session.
                    let fresh: Vec<RedditMedia> = imgs
                        .into_iter()
                        .filter(|m| !media_seen_in_session(m, avoid_urls))
                        .collect();
                    if !fresh.is_empty() {
                        return Ok((fresh, label));
                    }
                    last_err = RedditError::NoImages;
                }
                Err(e) => last_err = e,
            },
            Ok(_) => last_err = RedditError::Parse("non-JSON body".into()),
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_images_with_fallback(
    _subreddit: &str,
    _avoid_urls: &[String],
) -> Result<(Vec<RedditMedia>, &'static str), RedditError> {
    Err(RedditError::Network("browser only".into()))
}

pub async fn load_random_image(
    raw_sub: &str,
    avoid_urls: &[String],
) -> Result<(RedditMedia, &'static str), RedditError> {
    let sub = normalize_subreddit(raw_sub).ok_or(RedditError::InvalidSubreddit)?;
    load_random_image_for_sub(&sub, avoid_urls).await
}

/// Try each time window; within a window try many random candidates with live URL checks.
pub async fn load_random_image_for_sub(
    sub: &str,
    avoid_urls: &[String],
) -> Result<(RedditMedia, &'static str), RedditError> {
    #[cfg(target_arch = "wasm32")]
    {
        let mut last_err = RedditError::NoImages;
        for (url, label) in listing_endpoints(sub) {
            let text = match fetch_text(&url).await {
                Ok(t) if t.trim_start().starts_with('{') => t,
                Ok(_) => {
                    last_err = RedditError::Parse("non-JSON body".into());
                    continue;
                }
                Err(e) => {
                    last_err = e;
                    continue;
                }
            };
            let imgs = match extract_images(&text, sub) {
                Ok(v) => v,
                Err(e) => {
                    last_err = e;
                    continue;
                }
            };
            let mut fresh: Vec<RedditMedia> = imgs
                .into_iter()
                .filter(|m| !media_seen_in_session(m, avoid_urls))
                .collect();
            if fresh.is_empty() {
                last_err = RedditError::NoImages;
                continue;
            }
            // Shuffle
            for i in (1..fresh.len()).rev() {
                let j = fastrand::usize(..=i);
                fresh.swap(i, j);
            }
            let mut attempts = 0usize;
            for candidate in fresh {
                if attempts >= 18 {
                    break;
                }
                attempts += 1;
                if media_seen_in_session(&candidate, avoid_urls) {
                    continue;
                }
                if let Some(live) = filter_live_media(candidate).await {
                    if !media_seen_in_session(&live, avoid_urls) && !live.items.is_empty() {
                        return Ok((live, label));
                    }
                }
            }
            last_err = RedditError::NoImages;
        }
        Err(last_err)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (sub, avoid_urls);
        Err(RedditError::Network("browser only".into()))
    }
}

/// Keep only media that still loads (filters Reddit-deleted files).
pub async fn filter_live_media(media: RedditMedia) -> Option<RedditMedia> {
    if media.items.iter().any(|i| url_looks_deleted(&i.url)) {
        // Any tombstone URL → reject whole post (partial galleries are rebuilt below).
    }
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
        })
    }
}

#[cfg(target_arch = "wasm32")]
async fn media_item_is_available(item: &MediaItem) -> bool {
    // Prefer HTTP status when CORS allows; always fall back to element load probe.
    if let Some(ok) = http_url_ok(&item.url).await {
        if !ok {
            return false;
        }
    }
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
              "title": "Single",
              "url": "https://i.redd.it/top.jpg",
              "permalink": "/r/pics/comments/1/top/",
              "is_video": false,
              "post_hint": "image"
            },
            {
              "title": "Vid",
              "url": "https://v.redd.it/abc",
              "permalink": "/r/pics/comments/2/v/",
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
              "title": "Gallery",
              "url": "https://www.reddit.com/gallery/xyz",
              "permalink": "/r/pics/comments/3/g/",
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
        assert_eq!(items[0].items[0].kind, MediaKind::Image);
        assert_eq!(items[1].items[0].kind, MediaKind::Video);
        assert!(items[1].items[0].url.contains("DASH_480"));
        assert_eq!(items[2].items.len(), 2);
    }

    #[test]
    fn imgur_gifv_to_mp4() {
        let item = media_from_url("https://i.imgur.com/ZenTaxR.gifv", None).unwrap();
        assert_eq!(item.kind, MediaKind::Video);
        assert_eq!(item.url, "https://i.imgur.com/ZenTaxR.mp4");
    }

    #[test]
    fn skips_removed_posts() {
        let json = r#"{
          "data": [
            {
              "title": "[deleted]",
              "url": "https://i.redd.it/x.jpg",
              "permalink": "/r/pics/comments/1/",
              "author": "[deleted]",
              "removed_by_category": "moderator"
            },
            {
              "title": "Alive",
              "url": "https://i.redd.it/ok.jpg",
              "permalink": "/r/pics/comments/2/",
              "author": "someone",
              "post_hint": "image"
            }
          ]
        }"#;
        let items = extract_images(json, "pics").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].primary_url(), "https://i.redd.it/ok.jpg");
    }
}
