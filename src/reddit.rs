//! Fetch a random top image (or gallery) from a subreddit.

use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedditImage {
    /// One or more image URLs (gallery posts have multiple).
    pub urls: Vec<String>,
    pub title: String,
    pub permalink: String,
    pub subreddit: String,
}

impl RedditImage {
    pub fn primary_url(&self) -> &str {
        self.urls.first().map(|s| s.as_str()).unwrap_or("")
    }

    pub fn is_gallery(&self) -> bool {
        self.urls.len() > 1
    }
}

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
            RedditError::Network(s) => write!(f, "Could not load images ({s})"),
            RedditError::NoImages => {
                write!(
                    f,
                    "No image posts found (week/day/month) — try another subreddit"
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

    // Full / partial URLs: https://www.reddit.com/r/foo/..., reddit.com/r/foo
    let lower = s.to_ascii_lowercase();
    if let Some(idx) = lower.find("/r/") {
        let rest = &s[idx + 3..];
        let name = rest.split(['/', '?', '#']).next().unwrap_or("").trim();
        return validate_sub_name(name);
    }
    // reddit.com/r/foo without leading slash before r (rare)
    if let Some(idx) = lower.find("r/") {
        // only if it looks like a host path, not "super/foo"
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
}

/// Parse listing JSON into candidate image/gallery posts.
pub fn extract_images(json: &str, subreddit: &str) -> Result<Vec<RedditImage>, RedditError> {
    if let Ok(arr) = serde_json::from_str::<PostArrayResponse>(json) {
        return posts_to_images(arr.data, subreddit);
    }
    let listing: Listing =
        serde_json::from_str(json).map_err(|e| RedditError::Parse(e.to_string()))?;
    let posts: Vec<PostData> = listing.data.children.into_iter().map(|c| c.data).collect();
    posts_to_images(posts, subreddit)
}

fn posts_to_images(posts: Vec<PostData>, subreddit: &str) -> Result<Vec<RedditImage>, RedditError> {
    let mut out = Vec::new();
    let mut seen_keys = HashSet::new();
    for p in posts {
        if p.is_video.unwrap_or(false) {
            continue;
        }
        if let Some(hint) = &p.post_hint {
            if hint == "hosted:video" || hint == "rich:video" {
                continue;
            }
        }

        let urls = collect_post_urls(&p);
        if urls.is_empty() {
            continue;
        }
        let key = urls.join("|");
        if !seen_keys.insert(key) {
            continue;
        }

        let permalink = p.permalink.unwrap_or_default();
        let permalink = if permalink.starts_with("http") {
            permalink
        } else {
            format!("https://www.reddit.com{permalink}")
        };
        out.push(RedditImage {
            urls,
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

fn collect_post_urls(p: &PostData) -> Vec<String> {
    // Galleries first
    if p.is_gallery.unwrap_or(false) || p.gallery_data.is_some() {
        if let Some(urls) = gallery_urls(p) {
            if !urls.is_empty() {
                return urls;
            }
        }
    }

    let mut urls = Vec::new();
    if let Some(url) = &p.url {
        if is_image_url(url) {
            urls.push(decode_url(url));
        }
    }
    if urls.is_empty() {
        if let Some(url) = p
            .preview
            .as_ref()
            .and_then(|pr| pr.images.as_ref())
            .and_then(|imgs| imgs.first())
            .and_then(|img| img.source.as_ref())
            .and_then(|s| s.url.clone())
        {
            if is_image_url(&url) {
                urls.push(decode_url(&url));
            }
        }
    }
    urls
}

fn gallery_urls(p: &PostData) -> Option<Vec<String>> {
    let items = p.gallery_data.as_ref()?.items.as_ref()?;
    let meta = p.media_metadata.as_ref()?;
    let mut urls = Vec::new();
    for item in items {
        let id = item.media_id.as_ref()?;
        let m = meta.get(id)?;
        if m.status.as_deref() == Some("failed") {
            continue;
        }
        // Prefer still images; skip pure video entries
        if m.e.as_deref() == Some("RedditVideo") {
            continue;
        }
        let src = m.s.as_ref()?;
        let u = src.u.as_ref().or(src.gif.as_ref())?;
        let u = decode_url(u);
        if is_image_url(&u) || u.contains("preview.redd.it") || u.contains("i.redd.it") {
            urls.push(u);
        }
    }
    if urls.is_empty() {
        None
    } else {
        Some(urls)
    }
}

fn is_image_url(url: &str) -> bool {
    let u = url.split('?').next().unwrap_or(url).to_ascii_lowercase();
    if u.contains("v.redd.it") || u.contains("youtube.com") || u.contains("youtu.be") {
        return false;
    }
    u.contains("i.redd.it")
        || u.contains("preview.redd.it")
        || u.contains("i.imgur.com")
        || u.contains("imgur.com/")
        || u.ends_with(".jpg")
        || u.ends_with(".jpeg")
        || u.ends_with(".png")
        || u.ends_with(".webp")
        || u.ends_with(".gif")
}

fn decode_url(url: &str) -> String {
    url.replace("&amp;", "&")
}

pub fn pick_random_image(images: &[RedditImage], avoid_urls: &[String]) -> Option<RedditImage> {
    if images.is_empty() {
        return None;
    }
    let fresh: Vec<&RedditImage> = images
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
            "top today",
        ),
        (
            format!(
                "https://api.pullpush.io/reddit/search/submission/?subreddit={subreddit}&sort=desc&sort_type=score&size=100&since={month}"
            ),
            "top month",
        ),
        (
            format!(
                "https://api.pullpush.io/reddit/search/submission/?subreddit={subreddit}&sort=desc&sort_type=score&size=100"
            ),
            "top all-time",
        ),
        (
            format!(
                "https://api.pullpush.io/reddit/search/submission/?subreddit={subreddit}&sort=desc&sort_type=created_utc&size=100"
            ),
            "recent",
        ),
        (
            format!(
                "https://arctic-shift.photon-reddit.com/api/posts/search?subreddit={subreddit}&limit=100&sort=desc"
            ),
            "archive",
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

#[cfg(target_arch = "wasm32")]
pub async fn fetch_images_with_fallback(
    subreddit: &str,
) -> Result<(Vec<RedditImage>, &'static str), RedditError> {
    let mut last_err = RedditError::NoImages;
    for (url, label) in listing_endpoints(subreddit) {
        match fetch_text(&url).await {
            Ok(text) if text.trim_start().starts_with('{') => match extract_images(&text, subreddit)
            {
                Ok(imgs) if !imgs.is_empty() => return Ok((imgs, label)),
                Ok(_) => last_err = RedditError::NoImages,
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
) -> Result<(Vec<RedditImage>, &'static str), RedditError> {
    Err(RedditError::Network("browser only".into()))
}

pub async fn load_random_image(
    raw_sub: &str,
    avoid_urls: &[String],
) -> Result<(RedditImage, &'static str), RedditError> {
    let sub = normalize_subreddit(raw_sub).ok_or(RedditError::InvalidSubreddit)?;
    let (images, window) = fetch_images_with_fallback(&sub).await?;
    let img = pick_random_image(&images, avoid_urls).ok_or(RedditError::NoImages)?;
    Ok((img, window))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_names_and_urls() {
        assert_eq!(normalize_subreddit("pics").as_deref(), Some("pics"));
        assert_eq!(
            normalize_subreddit("r/EarthPorn").as_deref(),
            Some("EarthPorn")
        );
        assert_eq!(normalize_subreddit("/r/cats/").as_deref(), Some("cats"));
        assert_eq!(
            normalize_subreddit("https://www.reddit.com/r/FiftyFifty/").as_deref(),
            Some("FiftyFifty")
        );
        assert_eq!(
            normalize_subreddit("https://reddit.com/r/pics/comments/abc/title").as_deref(),
            Some("pics")
        );
        assert_eq!(
            normalize_subreddit("https://old.reddit.com/r/aww").as_deref(),
            Some("aww")
        );
        assert_eq!(normalize_subreddit("bad name"), None);
        assert_eq!(normalize_subreddit(""), None);
    }

    #[test]
    fn extract_single_and_gallery() {
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
              "title": "Gallery",
              "url": "https://www.reddit.com/gallery/xyz",
              "permalink": "/r/pics/comments/2/gal/",
              "is_gallery": true,
              "is_video": false,
              "gallery_data": {
                "items": [
                  { "media_id": "aaa" },
                  { "media_id": "bbb" }
                ]
              },
              "media_metadata": {
                "aaa": {
                  "e": "Image",
                  "s": { "u": "https://preview.redd.it/aaa.jpg?width=100" }
                },
                "bbb": {
                  "e": "Image",
                  "s": { "u": "https://preview.redd.it/bbb.jpg?width=100" }
                }
              }
            }
          ]
        }"#;
        let imgs = extract_images(json, "pics").unwrap();
        assert_eq!(imgs.len(), 2);
        assert_eq!(imgs[0].urls.len(), 1);
        assert_eq!(imgs[1].urls.len(), 2);
        assert!(imgs[1].is_gallery());
    }

    #[test]
    fn pick_avoids_recent() {
        let imgs = vec![
            RedditImage {
                urls: vec!["https://i.redd.it/a.jpg".into()],
                title: "a".into(),
                permalink: "https://reddit.com/a".into(),
                subreddit: "pics".into(),
            },
            RedditImage {
                urls: vec!["https://i.redd.it/b.jpg".into()],
                title: "b".into(),
                permalink: "https://reddit.com/b".into(),
                subreddit: "pics".into(),
            },
        ];
        let avoid = vec!["https://i.redd.it/a.jpg".into()];
        for _ in 0..20 {
            let p = pick_random_image(&imgs, &avoid).unwrap();
            assert_eq!(p.primary_url(), "https://i.redd.it/b.jpg");
        }
    }
}
