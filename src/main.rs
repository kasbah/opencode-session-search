mod db;
mod display;
mod fuzzy;

use clap::Parser;

/// Explore opencode session data from the command line.
#[derive(Parser)]
#[command(name = "oc-sessions", version, about)]
struct Cli {
    /// Fuzzy search query (matches against both title and directory)
    query: Option<String>,

    /// Fuzzy match only on session title
    #[arg(long)]
    title: Option<String>,

    /// Fuzzy match only on session directory
    #[arg(long)]
    dir: Option<String>,

    /// Maximum number of results to display
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// Include subagent sessions (explore/plan children)
    #[arg(long)]
    all: bool,

    /// Sort results by: date, title, dir
    #[arg(long, default_value = "date")]
    sort: String,
}

fn main() {
    let cli = Cli::parse();

    let sessions = match db::query_sessions(cli.all) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    // Determine if we need fuzzy filtering
    let has_query = cli.query.is_some();
    let has_title = cli.title.is_some();
    let has_dir = cli.dir.is_some();

    let mut filtered: Vec<db::Session> = if has_query || has_title || has_dir {
        let mut result = sessions.clone();

        // Apply --title filter
        if let Some(ref q) = cli.title {
            let scored = fuzzy::filter_sessions(result, q, true, false);
            result = scored.into_iter().map(|s| s.session).collect();
        }

        // Apply --dir filter
        if let Some(ref q) = cli.dir {
            let scored = fuzzy::filter_sessions(result, q, false, true);
            result = scored.into_iter().map(|s| s.session).collect();
        }

        // Apply positional query (matches both title and dir)
        if let Some(ref q) = cli.query {
            let scored = fuzzy::filter_sessions(result, q, true, true);
            result = scored.into_iter().map(|s| s.session).collect();
        }

        result
    } else {
        sessions
    };

    // Sort
    match cli.sort.as_str() {
        "title" => filtered.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase())),
        "dir" => {
            filtered.sort_by(|a, b| a.directory.to_lowercase().cmp(&b.directory.to_lowercase()))
        }
        _ => filtered.sort_by(|a, b| b.time_created.cmp(&a.time_created)), // date desc
    }

    // Apply limit
    filtered.truncate(cli.limit);

    display::print_table(&filtered);
}
