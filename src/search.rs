//! Fuzzy search over application entries using nucleo (the Helix editor matcher),
//! blended with persisted launch-frequency boosts (T4.2).

use std::collections::HashMap;

use nucleo::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo::{Config, Matcher, Utf32Str};

use crate::apps::AppEntry;
use crate::usage;

/// Score a single haystack against the query. `None` means "no match".
fn score(query: &Pattern, haystack: &str, scratch: &mut Vec<char>, matcher: &mut Matcher) -> Option<u32> {
    let haystack = Utf32Str::new(haystack, scratch);
    query.score(haystack, matcher)
}

/// Rank application indices for the given query.
///
/// An empty query returns every index ordered by launch frequency (most used
/// first), then alphabetically. Results for a real query are ordered by the
/// best fuzzy score plus a capped frequency boost, then by shorter names.
pub fn rank(apps: &[AppEntry], query: &str) -> Vec<usize> {
    rank_with_counts(apps, query, &usage::snapshot())
}

/// Same ranking against an explicit frequency table (test-friendly).
pub fn rank_with_counts(
    apps: &[AppEntry],
    query: &str,
    counts: &HashMap<String, u32>,
) -> Vec<usize> {
    let query = query.trim();
    if query.is_empty() {
        let mut indices: Vec<usize> = (0..apps.len()).collect();
        indices.sort_by(|a, b| {
            let fa = frequency_of(counts, &apps[*a].id);
            let fb = frequency_of(counts, &apps[*b].id);
            fb.cmp(&fa).then_with(|| {
                let na = apps[*a].name.to_lowercase();
                let nb = apps[*b].name.to_lowercase();
                na.cmp(&nb)
            })
        });
        return indices;
    }

    let pattern = Pattern::new(query, CaseMatching::Ignore, Normalization::Smart, AtomKind::Fuzzy);
    let mut matcher = Matcher::new(Config::DEFAULT);
    let mut scratch = Vec::new();

    let mut scored: Vec<(u32, usize)> = apps
        .iter()
        .enumerate()
        .filter_map(|(index, app)| {
            let mut best = score(&pattern, &app.name, &mut scratch, &mut matcher);
            for keyword in &app.keywords {
                if let Some(candidate) = score(&pattern, keyword, &mut scratch, &mut matcher) {
                    // Keyword hits are slightly weaker than direct name hits.
                    best = Some(best.unwrap_or(0).max(candidate * 9 / 10));
                }
            }
            best.map(|value| {
                // Frequently launched apps get a capped bump.
                let boost = frequency_of(counts, &app.id)
                    .min(usage::MAX_BOOST_STEPS)
                    * 8;
                (value.saturating_add(boost), index)
            })
        })
        .collect();

    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| {
        let na = &apps[a.1].name.to_lowercase();
        let nb = &apps[b.1].name.to_lowercase();
        na.len().cmp(&nb.len()).then_with(|| na.cmp(nb))
    }));

    scored.into_iter().map(|(_, index)| index).collect()
}

fn frequency_of(counts: &HashMap<String, u32>, app_id: &str) -> u32 {
    counts.get(app_id).copied().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<AppEntry> {
        vec![
            AppEntry::new("firefox.desktop", "Firefox"),
            AppEntry::new("org.gnome.Files", "Files"),
            AppEntry::new("code.desktop", "Visual Studio Code"),
            AppEntry::new("foot.desktop", "Foot"),
        ]
    }

    fn no_usage() -> HashMap<String, u32> {
        HashMap::new()
    }

    #[test]
    fn empty_query_returns_everything() {
        let apps = fixture();
        let counts = no_usage();
        // Alphabetical tie-break: Files, Firefox, Foot, Visual Studio Code.
        assert_eq!(rank_with_counts(&apps, "", &counts), vec![1, 0, 3, 2]);
        assert_eq!(rank_with_counts(&apps, "   ", &counts), vec![1, 0, 3, 2]);
    }

    #[test]
    fn empty_query_orders_by_frequency() {
        let apps = fixture();
        let mut counts = no_usage();
        counts.insert("foot.desktop".into(), 10);
        counts.insert("firefox.desktop".into(), 3);
        // Foot (10 uses) first, Firefox (3) second, never-used alphabetical.
        assert_eq!(
            rank_with_counts(&apps, "", &counts),
            vec![3, 0, 1, 2]
        );
    }

    #[test]
    fn exact_prefix_wins() {
        let apps = fixture();
        let counts = no_usage();
        let ranked = rank_with_counts(&apps, "fire", &counts);
        assert_eq!(ranked.first(), Some(&0));
    }

    #[test]
    fn fuzzy_subsequence_matches() {
        let apps = fixture();
        let counts = no_usage();
        let ranked = rank_with_counts(&apps, "ffox", &counts);
        assert!(ranked.contains(&0));
    }

    #[test]
    fn case_insensitive_matching() {
        let apps = fixture();
        let counts = no_usage();
        assert_eq!(rank_with_counts(&apps, "FILES", &counts).first(), Some(&1));
        assert_eq!(rank_with_counts(&apps, "files", &counts).first(), Some(&1));
    }

    #[test]
    fn no_match_yields_empty() {
        let apps = fixture();
        let counts = no_usage();
        assert!(rank_with_counts(&apps, "zzzzzzzz", &counts).is_empty());
    }

    #[test]
    fn better_matches_come_first() {
        let apps = fixture();
        let counts = no_usage();
        let ranked = rank_with_counts(&apps, "code", &counts);
        // "Visual Studio Code" contains "code" exactly; "Foot" does not match at all.
        assert!(!ranked.contains(&3));
        assert_eq!(ranked.first(), Some(&2));
    }

    #[test]
    fn frequency_boost_lifts_weaker_match() {
        let apps = fixture();
        // "foot" barely matches "Foot" (exact prefix, strong) — construct the
        // opposite: "file" strongly matches "Files", weakly "Firefox" (f-i-r...
        // no). Use a query both can match: "fo" matches "Foot" (prefix) and
        // "Firefox" (prefix). Without usage Firefox (shorter) wins.
        let mut counts = no_usage();
        counts.insert("firefox.desktop".into(), 64);
        let ranked = rank_with_counts(&apps, "fo", &counts);
        assert_eq!(ranked.first(), Some(&0), "boost must lift Firefox");
    }

    #[test]
    fn boost_is_capped_so_a_rare_exact_match_still_wins() {
        let apps = fixture();
        let mut counts = no_usage();
        counts.insert("foot.desktop".into(), u32::MAX);
        // "foot" exactly matches Foot; Firefox only fuzzy-matches. Even with
        // an uncapped raw count, the boost caps at MAX_BOOST_STEPS * 8 = 512.
        let ranked = rank_with_counts(&apps, "foot", &counts);
        assert_eq!(ranked.first(), Some(&3));
    }
}
