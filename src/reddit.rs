//! Fetch a random top image from a subreddit (browser-side via CORS proxies).

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
            RedditError::Network(s) => write!(f, "Could not reach Reddit ({s})"),
            RedditError::NoImages => write!(f, "No image posts found in top results"),
            RedditError::Parse(s) => write!(f, "Unexpected Reddit response ({s})"),
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
        .all(|c| c.is_ascii_alphanumeric() || c == '_' )
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
struct PostData {
    title: Option<String>,
    url: Option<String>,
    permalink: Option<String>,
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

/// Parse Reddit listing JSON into candidate image posts (SFW only).
pub fn extract_images(json: &str, subreddit: &str) -> Result<Vec<RedditImage>, RedditError> {
    let listing: Listing =
        serde_json::from_str(json).map_err(|e| RedditError::Parse(e.to_string()))?;
    let mut out = Vec::new();
    for child in listing.data.children {
        let p = child.data;
        if p.over_18.unwrap_or(false) {
            continue;
        }
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
        out.push(RedditImage {
            url: decode_url(&url),
            title: p.title.unwrap_or_default(),
            permalink: format!(
                "https://www.reddit.com{}",
                p.permalink.unwrap_or_default()
            ),
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
        || u.ends_with(".jpg")
        || u.ends_with(".jpeg")
        || u.ends_with(".png")
        || u.ends_with(".webp")
        || u.ends_with(".gif")
}

fn decode_url(url: &str) -> String {
    url.replace("&amp;", "&")
}

/// Reddit listing URL (needs a CORS proxy in the browser).
#[cfg(target_arch = "wasm32")]
pub fn reddit_listing_url(subreddit: &str) -> String {
    format!(
        "https://www.reddit.com/r/{subreddit}/top.json?limit=50&t=month&raw_json=1"
    )
}

#[cfg(target_arch = "wasm32")]
fn proxy_urls(target: &str) -> Vec<String> {
    let enc = urlencoding_minimal(target);
    vec![
        // Encode full target so query params survive.
        format!("https://corsproxy.io/?{enc}"),
        format!("https://api.allorigins.win/raw?url={enc}"),
    ]
}

/// Minimal encode for query values (enough for Reddit URLs).
#[cfg(target_arch = "wasm32")]
fn urlencoding_minimal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Fetch listing text via the first working CORS proxy.
#[cfg(target_arch = "wasm32")]
pub async fn fetch_listing_json(subreddit: &str) -> Result<String, RedditError> {
    let target = reddit_listing_url(subreddit);
    let mut last_err = RedditError::Network("no proxy tried".into());
    for proxy in proxy_urls(&target) {
        match gloo_net::http::Request::get(&proxy).send().await {
            Ok(resp) if resp.ok() => match resp.text().await {
                Ok(text) if text.trim_start().starts_with('{') => return Ok(text),
                Ok(_) => last_err = RedditError::Parse("non-JSON body".into()),
                Err(e) => last_err = RedditError::Network(e.to_string()),
            },
            Ok(resp) => {
                last_err = RedditError::Network(format!("HTTP {}", resp.status()));
            }
            Err(e) => last_err = RedditError::Network(e.to_string()),
        }
    }
    Err(last_err)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_listing_json(_subreddit: &str) -> Result<String, RedditError> {
    Err(RedditError::Network("wasm only".into()))
}

/// Load a random top image for the subreddit.
pub async fn load_random_image(raw_sub: &str) -> Result<RedditImage, RedditError> {
    let sub = normalize_subreddit(raw_sub).ok_or(RedditError::InvalidSubreddit)?;
    let json = fetch_listing_json(&sub).await?;
    let images = extract_images(&json, &sub)?;
    let idx = fastrand::usize(..images.len());
    Ok(images[idx].clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize() {
        assert_eq!(normalize_subreddit("pics").as_deref(), Some("pics"));
        assert_eq!(normalize_subreddit("r/EarthPorn").as_deref(), Some("EarthPorn"));
        assert_eq!(normalize_subreddit("/r/cats/").as_deref(), Some("cats"));
        assert_eq!(normalize_subreddit("bad name"), None);
        assert_eq!(normalize_subreddit(""), None);
    }

    #[test]
    fn extract_from_sample() {
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
              },
              {
                "data": {
                  "title": "Video",
                  "url": "https://v.redd.it/vid",
                  "permalink": "/r/x/",
                  "over_18": false,
                  "is_video": true
                }
              }
            ]
          }
        }"#;
        let imgs = extract_images(json, "cats").unwrap();
        assert_eq!(imgs.len(), 1);
        assert_eq!(imgs[0].url, "https://i.redd.it/abc123.jpg");
        assert_eq!(imgs[0].title, "A cat");
    }
}
