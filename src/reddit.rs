//! Fetch a random top image from a subreddit.
//!
//! Direct Reddit JSON is blocked in browsers (no CORS / bot walls).
//! We use Pullpush (and Arctic Shift fallback) with CORS enabled.
//! Search order for images: top week → top day → top month → all-time / recent.

use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedditImage {
    pub url: String,
    pub title: String,
    pub permalink: String,
    pub subreddit: String,
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
            RedditError::InvalidSubreddit => write!(f, "Enter a valid subreddit name"),
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

/// Normalize `r/pics`, `/r/pics/`, `pics` → `pics`.
pub fn normalize_subreddit(raw: &str) -> Option<String> {
    let s = raw.trim().trim_start_matches('/').trim();
    let s = s
        .strip_prefix("r/")
        .or_else(|| s.strip_prefix("R/"))
        .unwrap_or(s)
        .trim()
        .trim_end_matches('/');
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

/// Parse listing JSON into candidate image posts.
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
    let mut seen_urls = std::collections::HashSet::new();
    for p in posts {
        if p.is_video.unwrap_or(false) || p.is_gallery.unwrap_or(false) {
            continue;
        }
        if let Some(hint) = &p.post_hint {
            if hint == "hosted:video" || hint == "rich:video" {
                continue;
            }
        }
        let url = pick_image_url(&p);
        let Some(url) = url else { continue };
        if !is_image_url(&url) {
            continue;
        }
        let url = decode_url(&url);
        if !seen_urls.insert(url.clone()) {
            continue;
        }
        let permalink = p.permalink.unwrap_or_default();
        let permalink = if permalink.starts_with("http") {
            permalink
        } else {
            format!("https://www.reddit.com{permalink}")
        };
        out.push(RedditImage {
            url,
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

fn pick_image_url(p: &PostData) -> Option<String> {
    if let Some(url) = &p.url {
        if is_image_url(url) {
            return Some(url.clone());
        }
    }
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

/// Pick a random image, preferring ones not in `avoid_urls` (recent history).
pub fn pick_random_image(images: &[RedditImage], avoid_urls: &[String]) -> Option<RedditImage> {
    if images.is_empty() {
        return None;
    }
    let fresh: Vec<&RedditImage> = images
        .iter()
        .filter(|img| !avoid_urls.iter().any(|u| u == &img.url))
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

#[cfg(not(target_arch = "wasm32"))]
#[cfg(target_arch = "wasm32")]
fn now_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Pullpush windows: top by score within time range (via `since` unix ts).
/// Order: week → day → month → all-time score → recent.
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

/// Try week → day → month → fallbacks until we get images.
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

/// Load a random top image; avoids recently shown URLs when possible.
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
    fn normalize() {
        assert_eq!(normalize_subreddit("pics").as_deref(), Some("pics"));
        assert_eq!(
            normalize_subreddit("r/EarthPorn").as_deref(),
            Some("EarthPorn")
        );
        assert_eq!(normalize_subreddit("/r/cats/").as_deref(), Some("cats"));
        assert_eq!(normalize_subreddit("bad name"), None);
        assert_eq!(normalize_subreddit(""), None);
    }

    #[test]
    fn extract_from_reddit_listing() {
        let json = r#"{
          "data": {
            "children": [
              {
                "data": {
                  "title": "A cat",
                  "url": "https://i.redd.it/abc123.jpg",
                  "permalink": "/r/cats/comments/x/a_cat/",
                  "over_18": false,
                  "post_hint": "image",
                  "is_video": false
                }
              },
              {
                "data": {
                  "title": "NSFW",
                  "url": "https://i.redd.it/nsfw.jpg",
                  "permalink": "/r/x/",
                  "over_18": true,
                  "post_hint": "image"
                }
              }
            ]
          }
        }"#;
        let imgs = extract_images(json, "cats").unwrap();
        assert_eq!(imgs.len(), 2);
    }

    #[test]
    fn extract_from_pullpush_array() {
        let json = r#"{
          "data": [
            {
              "title": "Top post",
              "url": "https://i.redd.it/top.jpg",
              "permalink": "/r/pics/comments/1/top/",
              "over_18": false,
              "post_hint": "image",
              "is_video": false
            },
            {
              "title": "Link only",
              "url": "https://example.com/article",
              "permalink": "/r/pics/comments/2/",
              "over_18": false,
              "is_video": false
            }
          ]
        }"#;
        let imgs = extract_images(json, "pics").unwrap();
        assert_eq!(imgs.len(), 1);
        assert!(imgs[0].permalink.starts_with("https://www.reddit.com/"));
    }

    #[test]
    fn pick_avoids_recent() {
        let imgs = vec![
            RedditImage {
                url: "https://i.redd.it/a.jpg".into(),
                title: "a".into(),
                permalink: "https://reddit.com/a".into(),
                subreddit: "pics".into(),
            },
            RedditImage {
                url: "https://i.redd.it/b.jpg".into(),
                title: "b".into(),
                permalink: "https://reddit.com/b".into(),
                subreddit: "pics".into(),
            },
        ];
        let avoid = vec!["https://i.redd.it/a.jpg".into()];
        // With only one free, always get b
        for _ in 0..20 {
            let p = pick_random_image(&imgs, &avoid).unwrap();
            assert_eq!(p.url, "https://i.redd.it/b.jpg");
        }
    }
}
