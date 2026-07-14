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
            Self::NsfwOnly => "NSFW only",
        }
    }
}

/// High-quality SFW image/video subs (mostly photo/aesthetics, few text posts).
const SFW: &[&str] = &[
    "pics",
    "Itookapicture",
    "EarthPorn",
    "SpacePorn",
    "CityPorn",
    "VillagePorn",
    "RuralPorn",
    "ArchitecturePorn",
    "AbandonedPorn",
    "RoomPorn",
    "CozyPlaces",
    "InteriorDesign",
    "NatureIsFuckingLit",
    "MostBeautiful",
    "ExposurePorn",
    "WaterPorn",
    "SkyPorn",
    "WinterPorn",
    "AutumnPorn",
    "BridgePorn",
    "CabinPorn",
    "DesignPorn",
    "ArtPorn",
    "MuseumPorn",
    "HistoryPorn",
    "MapPorn",
    "FoodPorn",
    "DessertPorn",
    "Cinemagraphs",
    "PerfectTiming",
    "InterestingAsFuck",
    "mildlyinteresting",
    "oddlysatisfying",
    "Perfectfit",
    "aww",
    "rarepuppers",
    "catpictures",
    "dogpictures",
    "wildlifephotography",
    "birdpics",
    "macroporn",
    "insectporn",
    "BotanicalPorn",
    "succulents",
    "houseplants",
    "Analog",
    "filmphotography",
    "streetphotography",
    "portraitporn",
    "HumanPorn",
    "OldSchoolCool",
    "ColorizedHistory",
    "RetroFuturism",
    "Cyberpunk",
    "ImaginaryLandscapes",
    "wallpapers",
    "WidescreenWallpaper",
    "Animewallpaper",
    "carporn",
    "motorcycleporn",
    "MachinePorn",
    "ThingsCutInHalfPorn",
    "mechanical_gifs",
    "Art",
    "drawing",
    "PixelArt",
    "ImaginaryMonsters",
    "SpecArt",
    "futureporn",
    "InfrastructurePorn",
    "Aviationporn",
    "space",
    "astronomy",
    "astrophotography",
    "WeatherPorn",
    "SevereWeather",
    "FirePorn",
    "Lava",
    "underwaterphotography",
    "scuba",
    "hiking",
    "Outdoors",
    "CampingandHiking",
    "NationalPark",
    "travel",
    "travelphotos",
    "japanpics",
    "italyphotos",
    "europe",
];

/// Adult-only image/video communities. Finder never mixes these into the SFW pool.
const NSFW_ONLY: &[&str] = &[
    "nsfw",
    "NSFW_GIF",
    "RealGirls",
    "gonewild",
    "AsiansGoneWild",
    "latinas",
    "Amateur",
    "AmateurRoomPorn",
    "Nudes",
    "boobs",
    "ass",
    "booty",
    "milf",
    "curvy",
    "thick",
    "palegirls",
    "redheads",
    "brunette",
    "blondes",
    "altgonewild",
    "collegesluts",
    "LegalTeens",
    "BarelylegalTeens",
    "adorableporn",
    "prettygirls",
    "GodPussy",
    "pussy",
    "lips",
    "OnOff",
    "nsfwcosplay",
    "cosplaygirls",
    "rule34",
    "hentai",
    "ecchi",
    "yuri",
    "2busty2hide",
    "biggerthanyouthought",
    "TinyTits",
    "smallboobs",
    "homegrowntits",
    "Stacked",
    "pawg",
    "datgap",
    "thighhighs",
    "stockings",
    "lingerie",
    "nsfwoutfits",
    "GoneMild",
    "demisani",
    "Shemales",
    "traps",
    "futanari",
    "gaybrosgonemild",
    "ladybonersgw",
    "MassiveCock",
    "penis",
    "boltedontits",
    "rearpussy",
    "spreadeagle",
    "Facesitting",
    "freeuse",
    "public",
    "PublicFlashing",
    "FlashingGirls",
    "workgonewild",
    "WeddingRingsShowing",
    "FitNakedGirls",
    "fitgirls",
    "yoga",
    "nsfw_gifs",
    "60fpsporn",
    "holdthemoan",
    "quiver",
    "jilling",
    "breeding",
    "creampies",
    "cumsluts",
    "oral",
    "blowjobs",
    "deepthroat",
    "titfuck",
    "paag",
    "AsianHotties",
    "AsianNSFW",
    "IndiansGoneWild",
    "latinasgw",
    "Ebony",
    "DarkAngels",
    "blackchickswhitedicks",
];

pub fn pool_subs(pool: SubredditPool) -> &'static [&'static str] {
    match pool {
        SubredditPool::Sfw => SFW,
        SubredditPool::NsfwOnly => NSFW_ONLY,
    }
}

/// Pick a random sub from the pool. Prefer something other than `avoid` when possible.
pub fn pick_random_subreddit(pool: SubredditPool, avoid: Option<&str>) -> &'static str {
    let list = pool_subs(pool);
    debug_assert!(!list.is_empty());
    if list.is_empty() {
        return "pics";
    }

    let avoid_l = avoid.map(|s| s.trim().to_ascii_lowercase());
    // A few attempts to skip the current sub.
    for _ in 0..8 {
        let name = list[fastrand::usize(..list.len())];
        if avoid_l
            .as_ref()
            .map(|a| a != &name.to_ascii_lowercase())
            .unwrap_or(true)
        {
            return name;
        }
    }
    list[fastrand::usize(..list.len())]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pools_nonempty_and_disjoint() {
        assert!(!SFW.is_empty());
        assert!(!NSFW_ONLY.is_empty());
        let sfw: std::collections::HashSet<_> = SFW.iter().map(|s| s.to_ascii_lowercase()).collect();
        for n in NSFW_ONLY {
            assert!(
                !sfw.contains(&n.to_ascii_lowercase()),
                "NSFW sub also in SFW list: {n}"
            );
        }
    }

    #[test]
    fn pick_returns_from_pool() {
        let s = pick_random_subreddit(SubredditPool::Sfw, None);
        assert!(SFW.iter().any(|x| x.eq_ignore_ascii_case(s)));
        let n = pick_random_subreddit(SubredditPool::NsfwOnly, None);
        assert!(NSFW_ONLY.iter().any(|x| x.eq_ignore_ascii_case(n)));
    }

    #[test]
    fn pool_parse() {
        assert_eq!(SubredditPool::from_str("sfw"), SubredditPool::Sfw);
        assert_eq!(SubredditPool::from_str("nsfw"), SubredditPool::NsfwOnly);
        assert_eq!(SubredditPool::from_str("NSFW_ONLY"), SubredditPool::NsfwOnly);
    }
}
