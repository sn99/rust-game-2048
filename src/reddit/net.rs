//! Network fetch, CDN probes, and media batch loading (WASM).

#[cfg(target_arch = "wasm32")]
use std::collections::HashSet;

#[cfg(target_arch = "wasm32")]
use super::parse::{
    extract_images, json_post_id, json_post_is_public, json_post_score, url_looks_deleted,
};
use super::normalize::normalize_subreddit;
#[cfg(target_arch = "wasm32")]
use super::types::{MediaItem, MediaKind, RedditError, RedditMedia};
#[cfg(not(target_arch = "wasm32"))]
use super::types::{RedditError, RedditMedia};

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
/// Kept modest to cap archive/CDN traffic per wave.
#[cfg(target_arch = "wasm32")]
const TOP_POOL_SIZE: usize = 24;
/// Hard cap on posts requested in one batch (app should stay ≤ this).
#[cfg(target_arch = "wasm32")]
const MAX_BATCH_COUNT: usize = 4;

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

#[cfg(target_arch = "wasm32")]
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

/// How thoroughly to validate posts after the listing response.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadTier {
    /// Metadata + non-tombstone URLs only — minimal latency for UI.
    Fast,
    /// Batch id re-verify + CDN probe for each kept post — higher quality queue.
    Quality,
}

pub async fn load_random_image(
    raw_sub: &str,
    avoid_urls: &[String],
) -> Result<(RedditMedia, &'static str), RedditError> {
    let mut batch = load_media_batch(raw_sub, avoid_urls, 1, LoadTier::Fast).await?;
    batch.pop().ok_or(RedditError::NoImages)
}

/// Fetch up to `count` posts.
/// - [`LoadTier::Fast`]: one listing, no id re-verify, no CDN probe.
/// - [`LoadTier::Quality`]: listing + batch id re-verify + probe only kept posts.
pub async fn load_media_batch(
    raw_sub: &str,
    avoid_urls: &[String],
    count: usize,
    tier: LoadTier,
) -> Result<Vec<(RedditMedia, &'static str)>, RedditError> {
    let sub = normalize_subreddit(raw_sub).ok_or(RedditError::InvalidSubreddit)?;
    if count == 0 {
        return Ok(Vec::new());
    }
    #[cfg(target_arch = "wasm32")]
    let count = count.min(MAX_BATCH_COUNT);
    load_media_batch_for_sub(&sub, avoid_urls, count, tier).await
}

#[cfg(target_arch = "wasm32")]
async fn load_media_batch_for_sub(
    sub: &str,
    avoid_urls: &[String],
    count: usize,
    tier: LoadTier,
) -> Result<Vec<(RedditMedia, &'static str)>, RedditError> {
    // Fast waves get a fresh abort controller; quality upgrades share it only if nothing else runs.
    if tier == LoadTier::Fast {
        let _ = take_abort_signal();
    }
    let mut last_err = RedditError::NoImages;

    // Prefer week listing; one recent fallback if empty.
    for (label, days) in [("top week", Some(7_i64)), ("recent", None)] {
        match fetch_arctic_window(sub, days, 1, 0).await {
            Ok(posts) => {
                match try_pick_batch_from_posts(posts, sub, avoid_urls, 0, count, tier).await {
                    Ok(media) if !media.is_empty() => {
                        return Ok(media.into_iter().map(|m| (m, label)).collect());
                    }
                    Ok(_) => last_err = RedditError::NoImages,
                    Err(e) => last_err = e,
                }
            }
            Err(e) => last_err = e,
        }
    }

    Err(last_err)
}

#[cfg(not(target_arch = "wasm32"))]
async fn load_media_batch_for_sub(
    _sub: &str,
    _avoid_urls: &[String],
    _count: usize,
    _tier: LoadTier,
) -> Result<Vec<(RedditMedia, &'static str)>, RedditError> {
    Err(RedditError::Network("browser only".into()))
}

/// Pick up to `max_n` posts from a listing.
/// Fast: metadata only. Quality: re-verify ids + CDN-probe only the posts we keep.
#[cfg(target_arch = "wasm32")]
async fn try_pick_batch_from_posts(
    posts: Vec<serde_json::Value>,
    sub: &str,
    avoid_urls: &[String],
    soft_min_score: i64,
    max_n: usize,
    tier: LoadTier,
) -> Result<Vec<RedditMedia>, RedditError> {
    // Strict: only posts still robot-indexable (never pad with unknowns — those are often deleted).
    let mut public: Vec<serde_json::Value> = posts
        .iter()
        .filter(|p| json_post_is_public(p))
        .cloned()
        .collect();
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

    let mut top: Vec<serde_json::Value> = ranked.into_iter().take(TOP_POOL_SIZE).collect();
    if top.is_empty() {
        return Err(RedditError::NoImages);
    }

    // Archive re-check for every tier so deleted posts never ship to the UI.
    {
        let ids: Vec<String> = top.iter().filter_map(json_post_id).collect();
        let (public_ids, ok) = reverify_public_ids(&ids).await;
        if ok {
            top.retain(|p| {
                json_post_id(p)
                    .map(|id| public_ids.contains(&id))
                    .unwrap_or(false)
            });
            if top.is_empty() {
                return Err(RedditError::NoImages);
            }
        }
        // If reverify network failed entirely, keep metadata-public listing (best effort).
    }

    let mut media_list = posts_json_to_media(top, sub)?;
    media_list.retain(|m| !media_seen_in_session(m, avoid_urls) && !m.id.is_empty());
    for m in &mut media_list {
        m.items.retain(|it| !url_looks_deleted(&it.url));
    }
    media_list.retain(|m| !m.items.is_empty());
    if media_list.is_empty() {
        return Err(RedditError::NoImages);
    }

    let shuffle_n = media_list.len().min(max_n.saturating_mul(2).max(4));
    for i in (1..shuffle_n).rev() {
        let j = fastrand::usize(..=i);
        media_list.swap(i, j);
    }

    match tier {
        LoadTier::Fast => {
            // Metadata + id reverify only (no CDN probes) for snappy Play/Next.
            let out: Vec<RedditMedia> = media_list.into_iter().take(max_n).collect();
            if out.is_empty() {
                Err(RedditError::NoImages)
            } else {
                Ok(out)
            }
        }
        LoadTier::Quality => {
            // CDN-probe only posts we keep (+ a couple of spares).
            let max_probes = max_n.saturating_add(2);
            let mut out = Vec::with_capacity(max_n.min(media_list.len()));
            let mut probes = 0usize;
            for candidate in media_list {
                if out.len() >= max_n || probes >= max_probes {
                    break;
                }
                if media_seen_in_session(&candidate, avoid_urls) {
                    continue;
                }
                probes += 1;
                if let Some(live) = filter_live_media(candidate).await {
                    if !media_seen_in_session(&live, avoid_urls) && !live.items.is_empty() {
                        out.push(live);
                    }
                }
            }
            if out.is_empty() {
                Err(RedditError::NoImages)
            } else {
                Ok(out)
            }
        }
    }
}

/// Batch re-fetch posts by id; return still-public ids and whether any request succeeded.
#[cfg(target_arch = "wasm32")]
async fn reverify_public_ids(ids: &[String]) -> (HashSet<String>, bool) {
    let mut out = HashSet::new();
    if ids.is_empty() {
        return (out, true);
    }
    let signal = ACTIVE_ABORT.with(|c| c.borrow().as_ref().map(|ctrl| ctrl.signal()));
    let mut any_ok = false;
    for chunk in ids.chunks(25) {
        let joined = chunk.join(",");
        let url = format!("https://arctic-shift.photon-reddit.com/api/posts/ids?ids={joined}");
        let Ok(text) = fetch_text_with_opts(&url, 500.0, signal.as_ref()).await else {
            continue;
        };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        let Some(arr) = v.get("data").and_then(|d| d.as_array()) else {
            continue;
        };
        any_ok = true;
        for p in arr {
            // Only keep still-public; missing from response = gone/deleted.
            if json_post_is_public(p) {
                if let Some(id) = json_post_id(p) {
                    out.insert(id);
                }
            }
        }
        // Ids not present in the response are treated as deleted (not added to `out`).
        let _ = chunk;
    }
    (out, any_ok)
}

/// Drop posts that are no longer public or whose media CDN is dead.
/// Used for Unlocked gallery cleanup and restored media checks.
pub async fn filter_still_available_media(media: Vec<RedditMedia>) -> Vec<RedditMedia> {
    #[cfg(target_arch = "wasm32")]
    {
        if media.is_empty() {
            return media;
        }
        let ids: Vec<String> = media
            .iter()
            .map(|m| m.id.clone())
            .filter(|id| !id.is_empty())
            .collect();
        let (public_ids, reverify_ok) = reverify_public_ids(&ids).await;

        let mut kept = Vec::with_capacity(media.len());
        for m in media {
            let mut m = m;
            m.items.retain(|it| !url_looks_deleted(&it.url));
            if m.items.is_empty() {
                continue;
            }
            // Archive says gone / removed → drop (only when reverify succeeded).
            if reverify_ok && !m.id.is_empty() && !public_ids.contains(&m.id) {
                continue;
            }
            // CDN probe: catch deleted files that still look “public” in the archive.
            if let Some(live) = filter_live_media(m).await {
                kept.push(live);
            }
        }
        kept
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        media
    }
}

/// Keep only media that still loads (filters Reddit-deleted files).
#[cfg(target_arch = "wasm32")]
async fn filter_live_media(media: RedditMedia) -> Option<RedditMedia> {
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

