//! Parse Reddit listing JSON into playable media.

use serde::Deserialize;
use std::collections::{HashMap, HashSet};

use super::types::{MediaItem, MediaKind, RedditError, RedditMedia};

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
pub(crate) fn title_looks_removed(title: &str) -> bool {
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
pub(crate) fn json_post_is_known_dead(p: &serde_json::Value) -> bool {
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
pub(crate) fn json_post_is_public(p: &serde_json::Value) -> bool {
    if json_post_is_known_dead(p) {
        return false;
    }
    // Must be explicitly still indexable (CDN can outlive the post page).
    p.get("is_robot_indexable").and_then(|v| v.as_bool()) == Some(true)
}

pub(crate) fn json_post_id(p: &serde_json::Value) -> Option<String> {
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

pub(crate) fn json_post_score(p: &serde_json::Value) -> i64 {
    p.get("score")
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
        .unwrap_or(0)
}

pub(crate) fn url_looks_deleted(url: &str) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn json_post_id_and_score() {
        let with_id = serde_json::json!({"id": "t3_abc123", "score": 42});
        assert_eq!(json_post_id(&with_id).as_deref(), Some("abc123"));
        assert_eq!(json_post_score(&with_id), 42);

        let from_permalink = serde_json::json!({
            "permalink": "/r/pics/comments/xyz99/title/",
            "score": 7.5
        });
        assert_eq!(json_post_id(&from_permalink).as_deref(), Some("xyz99"));
        assert_eq!(json_post_score(&from_permalink), 7);

        let empty = serde_json::json!({});
        assert_eq!(json_post_id(&empty), None);
        assert_eq!(json_post_score(&empty), 0);
    }

}
