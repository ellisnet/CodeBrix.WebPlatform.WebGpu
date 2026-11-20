//! `SafeAreaGuides`: NoSkia's preset guides, with their sources (C# `SafeAreaGuides`, the same
//! names, figures and source notes).

use crate::safe_areas::{SafeAreaCrop, SafeAreaGuide, SafeAreaGuideOptions, SafeAreaMargins, SafeAreaTimeRule};

const META_AD_SOURCE: &str = "REPORTED: the Meta Ads Guide figures for Reels and Stories ads as repeated by third-party guides (November 2026): keep the top 14% (about 270 px of 1920), the bottom 35% (672 px) and 6% of each side (about 65 px of 1080) clear. Reels ads carrying a disclaimer should keep the bottom 40% clear. Other sites report other, mutually inconsistent figures; Meta Ads Manager shows its own safe-zone overlay. Advisory.";

/// The preset guides (C# `SafeAreaGuides`): each call builds the guide. Every figure is the C#
/// library's, with its source note (REPORTED / MEASURED figures are advisory; FACT ones are
/// broadcast standards).
pub struct SafeAreaGuides;

fn margins(top: f32, bottom: f32, left: f32, right: f32) -> SafeAreaMargins {
    SafeAreaMargins::new(top, bottom, left, right).expect("a preset's margins are valid")
}

fn guide(name: &str, margins: SafeAreaMargins, source: &str, crops: Vec<SafeAreaCrop>, time_rule: Option<SafeAreaTimeRule>) -> SafeAreaGuide {
    SafeAreaGuide::new(name, margins, SafeAreaGuideOptions { source: source.to_string(), crops, time_rule }).expect("a preset's name is valid")
}

fn meta_ad(name: &str) -> SafeAreaGuide {
    guide(name, margins(0.14, 0.35, 0.06, 0.06), META_AD_SOURCE, Vec::new(), None)
}

impl SafeAreaGuides {
    /// Instagram Reels ads (C# `InstagramReelsAd`): top 14 %, bottom 35 %, 6 % each side.
    pub fn instagram_reels_ad() -> SafeAreaGuide {
        meta_ad("Instagram Reels ad")
    }

    /// Instagram Stories ads (C# `InstagramStoriesAd`): as Reels ads.
    pub fn instagram_stories_ad() -> SafeAreaGuide {
        meta_ad("Instagram Stories ad")
    }

    /// Facebook Reels ads (C# `FacebookReelsAd`): as Instagram's.
    pub fn facebook_reels_ad() -> SafeAreaGuide {
        meta_ad("Facebook Reels ad")
    }

    /// Facebook Stories ads (C# `FacebookStoriesAd`): as Instagram's.
    pub fn facebook_stories_ad() -> SafeAreaGuide {
        meta_ad("Facebook Stories ad")
    }

    /// YouTube Shorts ads (C# `YouTubeShortsAd`): 288 / 672 of 1920 top / bottom, 48 / 192 of
    /// 1080 left / right.
    pub fn youtube_shorts_ad() -> SafeAreaGuide {
        guide("YouTube Shorts ad", margins(288.0 / 1920.0, 672.0 / 1920.0, 48.0 / 1080.0, 192.0 / 1080.0), "REPORTED: Google's universal video ad safe-zone template as repeated by third-party guides (November 2026): on 1080 x 1920 keep the top 288 px (15%), the bottom 672 px (35%), the left 48 px (4.4%) and the right 192 px (17.8%) clear, leaving x 48..888, y 288..1248. Google publishes no separate zone for ordinary Shorts. Advisory.", Vec::new(), None)
    }

    /// TikTok (C# `TikTok`): 288 / 461 of 1920, 86 / 216 of 1080.
    pub fn tiktok() -> SafeAreaGuide {
        guide("TikTok", margins(288.0 / 1920.0, 461.0 / 1920.0, 86.0 / 1080.0, 216.0 / 1080.0), "MEASURED by third parties from TikTok ad previews (reported March 2026; lower confidence): on 1080 x 1920 about 288 px top (15%), 461 px bottom (24%), 86 px left (8%) and 216 px right (20%). TikTok publishes its zone as a downloadable overlay template rather than as numbers. Advisory.", Vec::new(), None)
    }

    /// Ordinary (non-ad) portrait posts (C# `OrdinaryPortraitPost`): 200 / 370 of 1920, 60 / 164
    /// of 1080.
    pub fn ordinary_portrait_post() -> SafeAreaGuide {
        guide("Ordinary portrait post", margins(200.0 / 1920.0, 370.0 / 1920.0, 60.0 / 1080.0, 164.0 / 1080.0), "MEASURED by third parties for ordinary (non-ad) posts on the major portrait platforms; the figures are ranges, not constants (top 120-200 px, bottom 250-370 px, right 120-164 px, left 50-60 px of 1080 x 1920), and this guide uses the larger end of each. One measurement of organic Shorts found far more (about 380 px top and 390 px bottom). Advisory.", Vec::new(), None)
    }

    /// The Instagram profile grid (C# `InstagramProfileGrid`): a centred 3:4 crop.
    pub fn instagram_profile_grid() -> SafeAreaGuide {
        let crop = SafeAreaCrop::new("profile grid 3:4 crop", 3.0, 4.0).expect("a valid crop");
        guide("Instagram profile grid", SafeAreaMargins::NONE, "REPORTED by third-party guides (November 2026): the profile grid shows a centered 3:4 crop of a 9:16 Reel (1080 x 1440, losing 240 px top and bottom). Some sources say the grid is still square (1:1) on desktop web. Advisory.", vec![crop], None)
    }

    /// The Instagram feed preview (C# `InstagramFeedPreview`): a centred 4:5 crop.
    pub fn instagram_feed_preview() -> SafeAreaGuide {
        let crop = SafeAreaCrop::new("feed preview 4:5 crop", 4.0, 5.0).expect("a valid crop");
        guide("Instagram feed preview", SafeAreaMargins::NONE, "REPORTED by third-party guides (November 2026): the feed preview shows a centered 4:5 crop of a 9:16 Reel (1080 x 1350, losing 285 px top and bottom). Advisory.", vec![crop], None)
    }

    /// Broadcast title safe (C# `BroadcastTitleSafe`): 5 % on every edge.
    pub fn broadcast_title_safe() -> SafeAreaGuide {
        guide("Broadcast title safe", margins(0.05, 0.05, 0.05, 0.05), "FACT: SMPTE ST 2046-1 title-safe area, 90% of the width and height (5% margin per edge); EBU R95 and ITU-R BT.1848 give the same zone, and Netflix recommends it. The legacy pre-HD value was 80%.", Vec::new(), None)
    }

    /// Broadcast action safe (C# `BroadcastActionSafe`): 3.5 % on every edge.
    pub fn broadcast_action_safe() -> SafeAreaGuide {
        guide("Broadcast action safe", margins(0.035, 0.035, 0.035, 0.035), "FACT: SMPTE ST 2046-1 action-safe area, 93% of the width and height (3.5% margin per edge); EBU R95 and ITU-R BT.1848 give the same zone, and Netflix recommends it. The legacy pre-HD value was 90%.", Vec::new(), None)
    }

    /// The YouTube end screen (C# `YouTubeEndScreen`): the whole frame, in the final 20 seconds.
    pub fn youtube_end_screen() -> SafeAreaGuide {
        let rule = SafeAreaTimeRule::final_seconds(20.0).expect("a valid rule");
        guide("YouTube end screen", SafeAreaMargins::WHOLE_FRAME, "REPORTED by third-party guides (November 2026): end screens occupy the last 5-20 seconds of a video (which must be at least 25 s long; up to four elements on 16:9), so the safest rule is to keep the final 20 seconds free of anything that matters. Info cards can appear at any time; no positions are published for the title bar, progress bar, watermark or captions. Advisory.", Vec::new(), Some(rule))
    }
}
