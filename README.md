# Opencode Session Search

Lists all the [Opencode](https://opencode.ai/) sessions across all folders and lets you fuzzy search. Has columns for title, last message, directory and date.

## Usage

```
cargo install --git https://github.com/kasbah/opencode-session-search
```

```
opencode-session-search
```

- `<F2>` to switch between sorting by date or search score.
- Prefix searches with `title:`, `mes:` (last message) or `dir:` (directory) to restrict search to specific columns.
- Up/down arrows to select. Press enter to open in Opencode in current folder (`opencode -s <session_id>`).
- Tested on Linux only for now.

![screenshot](https://raw.githubusercontent.com/kasbah/opencode-session-search/main/screenshot.png)
