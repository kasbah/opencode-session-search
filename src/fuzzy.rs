use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use crate::db::Session;

/// A session paired with its fuzzy match score.
pub struct ScoredSession {
    pub session: Session,
    pub score: i64,
}

/// Filter and score sessions by fuzzy matching on the given fields.
///
/// - `query`: the fuzzy search string
/// - `match_title`: whether to match against the session title
/// - `match_dir`: whether to match against the session directory
///
/// Returns sessions sorted by descending score (best match first).
pub fn filter_sessions(
    sessions: Vec<Session>,
    query: &str,
    match_title: bool,
    match_dir: bool,
) -> Vec<ScoredSession> {
    let matcher = SkimMatcherV2::default();

    let mut scored: Vec<ScoredSession> = sessions
        .into_iter()
        .filter_map(|session| {
            let title_score = if match_title {
                matcher.fuzzy_match(&session.title, query).unwrap_or(0)
            } else {
                0
            };
            let dir_score = if match_dir {
                matcher.fuzzy_match(&session.directory, query).unwrap_or(0)
            } else {
                0
            };

            let score = title_score.max(dir_score);
            if score > 0 {
                Some(ScoredSession { session, score })
            } else {
                None
            }
        })
        .collect();

    scored.sort_by(|a, b| b.score.cmp(&a.score));
    scored
}
