//! Curated image/video-friendly subreddits for the random finder.
/// Which pool the random finder draws from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubredditPool {
    /// Safe-for-work photography / aesthetics communities.
    Sfw,
    /// Adult (18+) image communities only.
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

    pub fn label(self) -> &'static str {
        match self {
            Self::Sfw => "SFW",
            Self::NsfwOnly => "NSFW",
        }
    }
}

/// A curated community with a short plain-language blurb.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubredditEntry {
    pub name: &'static str,
    pub blurb: &'static str,
}

const SFW: &[SubredditEntry] = &[
    SubredditEntry { name: "pics", blurb: "Photographs and pictures from all over" },
    SubredditEntry { name: "Itookapicture", blurb: "Photos you took yourself" },
    SubredditEntry { name: "EarthPorn", blurb: "Stunning landscape photography" },
    SubredditEntry { name: "SpacePorn", blurb: "Space and astronomy images" },
    SubredditEntry { name: "CityPorn", blurb: "Beautiful cities and skylines" },
    SubredditEntry { name: "VillagePorn", blurb: "Charming towns and villages" },
    SubredditEntry { name: "RuralPorn", blurb: "Countryside and rural scenery" },
    SubredditEntry { name: "ArchitecturePorn", blurb: "Impressive architecture photos" },
    SubredditEntry { name: "AbandonedPorn", blurb: "Abandoned and forgotten places" },
    SubredditEntry { name: "RoomPorn", blurb: "Beautiful rooms and interiors" },
    SubredditEntry { name: "CozyPlaces", blurb: "Warm, cozy spaces to relax in" },
    SubredditEntry { name: "InteriorDesign", blurb: "Interior design inspiration" },
    SubredditEntry { name: "NatureIsFuckingLit", blurb: "Jaw-dropping nature photos" },
    SubredditEntry { name: "MostBeautiful", blurb: "Exceptionally beautiful scenes" },
    SubredditEntry { name: "ExposurePorn", blurb: "Long-exposure photography" },
    SubredditEntry { name: "WaterPorn", blurb: "Oceans, lakes, rivers, waterfalls" },
    SubredditEntry { name: "SkyPorn", blurb: "Skies, clouds, and sunsets" },
    SubredditEntry { name: "WinterPorn", blurb: "Snowy winter landscapes" },
    SubredditEntry { name: "AutumnPorn", blurb: "Fall colors and autumn scenes" },
    SubredditEntry { name: "BridgePorn", blurb: "Bridges from around the world" },
    SubredditEntry { name: "CabinPorn", blurb: "Cabins, cottages, and hideaways" },
    SubredditEntry { name: "DesignPorn", blurb: "Striking industrial & product design" },
    SubredditEntry { name: "ArtPorn", blurb: "Fine art photography and images" },
    SubredditEntry { name: "MuseumPorn", blurb: "Museums, galleries, exhibits" },
    SubredditEntry { name: "HistoryPorn", blurb: "Historical photographs" },
    SubredditEntry { name: "MapPorn", blurb: "Beautiful and interesting maps" },
    SubredditEntry { name: "FoodPorn", blurb: "Mouth-watering food photos" },
    SubredditEntry { name: "DessertPorn", blurb: "Desserts and sweet treats" },
    SubredditEntry { name: "Cinemagraphs", blurb: "Subtle looping photo-videos" },
    SubredditEntry { name: "PerfectTiming", blurb: "Perfectly timed photos" },
    SubredditEntry { name: "InterestingAsFuck", blurb: "Wildly interesting images" },
    SubredditEntry { name: "mildlyinteresting", blurb: "Mildly interesting everyday finds" },
    SubredditEntry { name: "oddlysatisfying", blurb: "Oddly satisfying visuals" },
    SubredditEntry { name: "Perfectfit", blurb: "Things that fit perfectly" },
    SubredditEntry { name: "aww", blurb: "Cute animals and adorable moments" },
    SubredditEntry { name: "rarepuppers", blurb: "Very good dogs" },
    SubredditEntry { name: "catpictures", blurb: "Pictures of cats" },
    SubredditEntry { name: "dogpictures", blurb: "Pictures of dogs" },
    SubredditEntry { name: "wildlifephotography", blurb: "Wildlife photography" },
    SubredditEntry { name: "birdpics", blurb: "Bird photography" },
    SubredditEntry { name: "macroporn", blurb: "Extreme close-up macro shots" },
    SubredditEntry { name: "insectporn", blurb: "Insects and bugs up close" },
    SubredditEntry { name: "BotanicalPorn", blurb: "Plants and botanical beauty" },
    SubredditEntry { name: "succulents", blurb: "Succulent plant photos" },
    SubredditEntry { name: "houseplants", blurb: "Indoor plants and greenery" },
    SubredditEntry { name: "Analog", blurb: "Film and analog photography" },
    SubredditEntry { name: "filmphotography", blurb: "Shot on film" },
    SubredditEntry { name: "streetphotography", blurb: "Street photography" },
    SubredditEntry { name: "portraitporn", blurb: "Portrait photography" },
    SubredditEntry { name: "HumanPorn", blurb: "Artistic photos of people" },
    SubredditEntry { name: "OldSchoolCool", blurb: "Cool photos from the past" },
    SubredditEntry { name: "ColorizedHistory", blurb: "Colorized historical photos" },
    SubredditEntry { name: "RetroFuturism", blurb: "Vintage visions of the future" },
    SubredditEntry { name: "Cyberpunk", blurb: "Neon cyberpunk aesthetics" },
    SubredditEntry { name: "ImaginaryLandscapes", blurb: "Fantasy landscape art" },
    SubredditEntry { name: "wallpapers", blurb: "Desktop wallpapers" },
    SubredditEntry { name: "WidescreenWallpaper", blurb: "Widescreen wallpapers" },
    SubredditEntry { name: "Animewallpaper", blurb: "Anime-style wallpapers" },
    SubredditEntry { name: "carporn", blurb: "Beautiful car photography" },
    SubredditEntry { name: "motorcycleporn", blurb: "Motorcycle photography" },
    SubredditEntry { name: "MachinePorn", blurb: "Machines, engines, and tech" },
    SubredditEntry { name: "ThingsCutInHalfPorn", blurb: "Cross-sections of everyday things" },
    SubredditEntry { name: "mechanical_gifs", blurb: "Satisfying mechanical GIFs" },
    SubredditEntry { name: "Art", blurb: "Art of all kinds" },
    SubredditEntry { name: "drawing", blurb: "Drawings and sketches" },
    SubredditEntry { name: "PixelArt", blurb: "Pixel art creations" },
    SubredditEntry { name: "ImaginaryMonsters", blurb: "Fantasy monster art" },
    SubredditEntry { name: "SpecArt", blurb: "Speculative & concept art" },
    SubredditEntry { name: "futureporn", blurb: "Futuristic tech and design" },
    SubredditEntry { name: "InfrastructurePorn", blurb: "Infrastructure and engineering" },
    SubredditEntry { name: "Aviationporn", blurb: "Aircraft photography" },
    SubredditEntry { name: "space", blurb: "Space discussion and images" },
    SubredditEntry { name: "astronomy", blurb: "Astronomy photos and news" },
    SubredditEntry { name: "astrophotography", blurb: "Photos of the night sky" },
    SubredditEntry { name: "WeatherPorn", blurb: "Dramatic weather photos" },
    SubredditEntry { name: "SevereWeather", blurb: "Storms and severe weather" },
    SubredditEntry { name: "FirePorn", blurb: "Fire and flames (safely)" },
    SubredditEntry { name: "Lava", blurb: "Lava and volcanic scenes" },
    SubredditEntry { name: "underwaterphotography", blurb: "Underwater photos" },
    SubredditEntry { name: "scuba", blurb: "Scuba and diving images" },
    SubredditEntry { name: "hiking", blurb: "Hiking trails and views" },
    SubredditEntry { name: "Outdoors", blurb: "Outdoor adventure photos" },
    SubredditEntry { name: "CampingandHiking", blurb: "Camping and hiking life" },
    SubredditEntry { name: "NationalPark", blurb: "National park scenery" },
    SubredditEntry { name: "travel", blurb: "Travel photos and stories" },
    SubredditEntry { name: "travelphotos", blurb: "Travel photography" },
    SubredditEntry { name: "japanpics", blurb: "Photos from Japan" },
    SubredditEntry { name: "italyphotos", blurb: "Photos from Italy" },
    SubredditEntry { name: "europe", blurb: "Europe travel and culture" },
];

/// Adult-only image/video communities. Never mixed into the SFW pool.
const NSFW_ONLY: &[SubredditEntry] = &[
    SubredditEntry { name: "nsfw", blurb: "General adult image community (18+)" },
    SubredditEntry { name: "NSFW_GIF", blurb: "Animated adult GIFs (18+)" },
    SubredditEntry { name: "RealGirls", blurb: "Amateur photos of real women (18+)" },
    SubredditEntry { name: "gonewild", blurb: "Amateur exhibitionist photos (18+)" },
    SubredditEntry { name: "AsiansGoneWild", blurb: "Amateur Asian adult photos (18+)" },
    SubredditEntry { name: "latinas", blurb: "Latina beauty and adult photos (18+)" },
    SubredditEntry { name: "Amateur", blurb: "Amateur adult photography (18+)" },
    SubredditEntry { name: "AmateurRoomPorn", blurb: "Amateur photos with room context (18+)" },
    SubredditEntry { name: "Nudes", blurb: "Artistic and casual nudes (18+)" },
    SubredditEntry { name: "boobs", blurb: "Breast-focused adult images (18+)" },
    SubredditEntry { name: "ass", blurb: "Butt-focused adult images (18+)" },
    SubredditEntry { name: "booty", blurb: "Booty-focused adult photos (18+)" },
    SubredditEntry { name: "milf", blurb: "MILF adult community (18+)" },
    SubredditEntry { name: "curvy", blurb: "Curvy body adult photos (18+)" },
    SubredditEntry { name: "thick", blurb: "Thick-body adult photos (18+)" },
    SubredditEntry { name: "palegirls", blurb: "Pale skin adult photos (18+)" },
    SubredditEntry { name: "redheads", blurb: "Redhead adult photos (18+)" },
    SubredditEntry { name: "brunette", blurb: "Brunette adult photos (18+)" },
    SubredditEntry { name: "blondes", blurb: "Blonde adult photos (18+)" },
    SubredditEntry { name: "altgonewild", blurb: "Alt / alternative adult amateurs (18+)" },
    SubredditEntry { name: "collegesluts", blurb: "College-age adult amateurs (18+)" },
    SubredditEntry { name: "LegalTeens", blurb: "18+ young adult content" },
    SubredditEntry { name: "BarelylegalTeens", blurb: "18+ young adult content" },
    SubredditEntry { name: "adorableporn", blurb: "Cute-style adult photos (18+)" },
    SubredditEntry { name: "prettygirls", blurb: "Attractive women, often adult (18+)" },
    SubredditEntry { name: "GodPussy", blurb: "Explicit adult close-ups (18+)" },
    SubredditEntry { name: "pussy", blurb: "Explicit adult images (18+)" },
    SubredditEntry { name: "lips", blurb: "Lips and related adult images (18+)" },
    SubredditEntry { name: "OnOff", blurb: "On/off clothing comparison (18+)" },
    SubredditEntry { name: "nsfwcosplay", blurb: "Adult cosplay photos (18+)" },
    SubredditEntry { name: "cosplaygirls", blurb: "Cosplay with adult-leaning content (18+)" },
    SubredditEntry { name: "rule34", blurb: "Rule 34 fan art (18+)" },
    SubredditEntry { name: "hentai", blurb: "Adult anime-style art (18+)" },
    SubredditEntry { name: "ecchi", blurb: "Suggestive anime-style images (18+)" },
    SubredditEntry { name: "yuri", blurb: "Women-loving-women anime (18+)" },
    SubredditEntry { name: "2busty2hide", blurb: "Busty adult photos (18+)" },
    SubredditEntry { name: "biggerthanyouthought", blurb: "Surprisingly large bust photos (18+)" },
    SubredditEntry { name: "TinyTits", blurb: "Petite-bust adult photos (18+)" },
    SubredditEntry { name: "smallboobs", blurb: "Small-bust adult photos (18+)" },
    SubredditEntry { name: "homegrowntits", blurb: "Natural-breast adult photos (18+)" },
    SubredditEntry { name: "Stacked", blurb: "Very busty adult photos (18+)" },
    SubredditEntry { name: "pawg", blurb: "PAWG adult photos (18+)" },
    SubredditEntry { name: "datgap", blurb: "Thigh gap focused adult photos (18+)" },
    SubredditEntry { name: "thighhighs", blurb: "Thigh-highs / socks fetish (18+)" },
    SubredditEntry { name: "stockings", blurb: "Stockings fetish photos (18+)" },
    SubredditEntry { name: "lingerie", blurb: "Lingerie photography (18+)" },
    SubredditEntry { name: "nsfwoutfits", blurb: "Sexy outfits (18+)" },
    SubredditEntry { name: "GoneMild", blurb: "Suggestive but milder amateurs (18+)" },
    SubredditEntry { name: "FitNakedGirls", blurb: "Fit women nude/athletic (18+)" },
    SubredditEntry { name: "fitgirls", blurb: "Fit women fitness photos (often 18+)" },
    SubredditEntry { name: "nsfw_gifs", blurb: "Adult GIF clips (18+)" },
    SubredditEntry { name: "60fpsporn", blurb: "Smooth high-framerate adult clips (18+)" },
    SubredditEntry { name: "holdthemoan", blurb: "Quiet/risky adult clips (18+)" },
    SubredditEntry { name: "AsianHotties", blurb: "Asian adult photos (18+)" },
    SubredditEntry { name: "AsianNSFW", blurb: "Asian NSFW community (18+)" },
    SubredditEntry { name: "IndiansGoneWild", blurb: "South Asian amateur adult (18+)" },
    SubredditEntry { name: "latinasgw", blurb: "Latina amateur adult (18+)" },
    SubredditEntry { name: "Ebony", blurb: "Black women adult photos (18+)" },
    SubredditEntry { name: "DarkAngels", blurb: "Dark-skinned adult models (18+)" },
    SubredditEntry { name: "workgonewild", blurb: "Work-related amateur adult (18+)" },
    SubredditEntry { name: "PublicFlashing", blurb: "Public flashing (18+)" },
    SubredditEntry { name: "FlashingGirls", blurb: "Flashing photos (18+)" },
    SubredditEntry { name: "ladybonersgw", blurb: "Attractive men gone wild (18+)" },
    SubredditEntry { name: "gaybrosgonemild", blurb: "Mild male amateur photos (18+)" },
];

pub fn pool_entries(pool: SubredditPool) -> &'static [SubredditEntry] {
    match pool {
        SubredditPool::Sfw => SFW,
        SubredditPool::NsfwOnly => NSFW_ONLY,
    }
}

/// Curated blurb if we know this sub; None for free-typed names.
pub fn curated_blurb(name: &str) -> Option<&'static str> {
    let key = name.trim();
    if key.is_empty() {
        return None;
    }
    for e in SFW.iter().chain(NSFW_ONLY.iter()) {
        if e.name.eq_ignore_ascii_case(key) {
            return Some(e.blurb);
        }
    }
    None
}

/// Pick a random entry from the pool. Prefer something other than `avoid` when possible.
pub fn pick_random_entry(pool: SubredditPool, avoid: Option<&str>) -> SubredditEntry {
    let list = pool_entries(pool);
    if list.is_empty() {
        return SubredditEntry {
            name: "pics",
            blurb: "Photographs and pictures from all over",
        };
    }
    let avoid_l = avoid.map(|s| s.trim().to_ascii_lowercase());
    for _ in 0..8 {
        let e = list[fastrand::usize(..list.len())];
        if avoid_l
            .as_ref()
            .map(|a| a != &e.name.to_ascii_lowercase())
            .unwrap_or(true)
        {
            return e;
        }
    }
    list[fastrand::usize(..list.len())]
}

/// Back-compat name picker.
pub fn pick_random_subreddit(pool: SubredditPool, avoid: Option<&str>) -> &'static str {
    pick_random_entry(pool, avoid).name
}

/// Fetch a live short description from Arctic Shift (CORS-friendly).
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
    let over18 = p.get("over18").and_then(|x| x.as_bool()).unwrap_or(false);
    let mut desc = public
        .or(title)
        .unwrap_or("Reddit community")
        .to_string();
    // Collapse whitespace / newlines for a one-line UI blurb.
    desc = desc.split_whitespace().collect::<Vec<_>>().join(" ");
    if desc.len() > 160 {
        desc.truncate(157);
        desc.push_str("…");
    }
    if over18 && !desc.to_ascii_lowercase().contains("18") {
        desc.push_str(" (18+)");
    }
    Some(desc)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_subreddit_description(_name: &str) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pools_nonempty_and_disjoint() {
        assert!(!SFW.is_empty());
        assert!(!NSFW_ONLY.is_empty());
        let sfw: std::collections::HashSet<_> =
            SFW.iter().map(|e| e.name.to_ascii_lowercase()).collect();
        for e in NSFW_ONLY {
            assert!(
                !sfw.contains(&e.name.to_ascii_lowercase()),
                "NSFW sub also in SFW list: {}",
                e.name
            );
        }
    }

    #[test]
    fn pick_returns_from_pool() {
        let s = pick_random_entry(SubredditPool::Sfw, None);
        assert!(SFW.iter().any(|x| x.name.eq_ignore_ascii_case(s.name)));
        assert!(!s.blurb.is_empty());
        let n = pick_random_entry(SubredditPool::NsfwOnly, None);
        assert!(NSFW_ONLY.iter().any(|x| x.name.eq_ignore_ascii_case(n.name)));
    }

    #[test]
    fn curated_blurb_lookup() {
        assert!(curated_blurb("pics").unwrap().contains("Photograph"));
        assert!(curated_blurb("NSFW").is_some() || curated_blurb("nsfw").is_some());
        assert_eq!(curated_blurb("not-a-real-sub-xyz"), None);
    }

    #[test]
    fn pool_parse() {
        assert_eq!(SubredditPool::from_str("sfw"), SubredditPool::Sfw);
        assert_eq!(SubredditPool::from_str("nsfw"), SubredditPool::NsfwOnly);
    }
}
