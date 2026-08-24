//! Fuzzy search over application entries using nucleo (the Helix editor matcher).

use nucleo::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo::{Config, Matcher, Utf32Str};

use crate::apps::AppEntry;

/// Score a single haystack against the query. `None` means "no match".
fn score(query: &Pattern, haystack: &str, scratch: &mut Vec<char>, matcher: &mut Matcher) -> Option<u32> {
    let haystack = Utf32Str::new(haystack, scratch);
    query.score(haystack, matcher)
}

/// Rank application indices for the given query.
///
/// An empty query returns every index in their existing (alphabetical) order.
/// Results are ordered by best fuzzy score, then by shorter names.
pub fn rank(apps: &[AppEntry], query: &str) -> Vec<usize> {
    let query = query.trim();
    if query.is_empty() {
        return (0..apps.len()).collect();
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
            best.map(|value| (value, index))
        })
        .collect();

    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| {
        let na = &apps[a.1].name.to_lowercase();
        let nb = &apps[b.1].name.to_lowercase();
        na.len().cmp(&nb.len()).then_with(|| na.cmp(nb))
    }));

    scored.into_iter().map(|(_, index)| index).collect()
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

    #[test]
    fn empty_query_returns_everything() {
        let apps = fixture();
        assert_eq!(rank(&apps, ""), vec![0, 1, 2, 3]);
        assert_eq!(rank(&apps, "   "), vec![0, 1, 2, 3]);
    }

    #[test]
    fn exact_prefix_wins() {
        let apps = fixture();
        let ranked = rank(&apps, "fire");
        assert_eq!(ranked.first(), Some(&0));
    }

    #[test]
    fn fuzzy_subsequence_matches() {
        let apps = fixture();
        let ranked = rank(&apps, "ffox");
        assert!(ranked.contains(&0));
    }

    #[test]
    fn case_insensitive_matching() {
        let apps = fixture();
        assert_eq!(rank(&apps, "FILES").first(), Some(&1));
        assert_eq!(rank(&apps, "files").first(), Some(&1));
    }

    #[test]
    fn no_match_yields_empty() {
        let apps = fixture();
        assert!(rank(&apps, "zzzzzzzz").is_empty());
    }

    #[test]
    fn better_matches_come_first() {
        let apps = fixture();
        let ranked = rank(&apps, "code");
        // "Visual Studio Code" contains "code" exactly; "Foot" does not match at all.
        assert!(!ranked.contains(&3));
        assert_eq!(ranked.first(), Some(&2));
    }
}
