# Todoist Terminal Client

This is a terminal client for Todoist, (inspired by Spotify-TUI). The project is designed to be a keyboard-driven interface for managing your Todoist tasks directly from your terminal.

  <img src="imgs/app.gif" width="75%">

## Features

- Doom Emacs-style keys: Vim movement, a `SPC` leader and a which-key menu, so every binding is discoverable (`?` lists them all)
- Views for Inbox, Today (with overdue), Upcoming, All, projects (as a tree, with sections) and labels
- Quick add with Todoist's natural-language syntax: `buy milk tomorrow 5pm p1 #shopping @errands`
- Single-key edits: complete, delete, undo, priority, postpone, due date, rename, labels, move
- Nested, foldable sub-tasks; search; fuzzy jump to any project or label
- Instant, optimistic updates with rollback on failure, and automatic background refresh

See [HOWTO_en.md](HOWTO_en.md) for the full key reference.

## Installing

1. Install cargo if you haven't done already
   On Linux and macOS systems, this is done as follows:
   ```
   curl https://sh.rustup.rs -sSf | sh
   ```
2. Clone this repository
   ```
   git clone https://github.com/illiteratewriter/todoist-rs.git
   ```
3. Run
   ```
   cargo install --path . --locked
   ```
   to install _todoist_ to path

## Project layout

- `main.rs`: startup and the event loop
- `app.rs`: app state, cursor/selection handling, background messages and sync
- `actions.rs`: what each action does (optimistic updates, undo, add, edit)
- `keymap.rs`: the single table of key bindings (drives dispatch, which-key and help)
- `keys.rs`: routes key presses to the current mode (normal, prompt, picker, edit panel, overlay)
- `view.rs`: which tasks a view contains and how they are laid out as rows
- `api_calls.rs`: all Todoist API v1 requests; failures are reported back to the UI
- `store.rs`, `tasks.rs`, `projects.rs`, `sections.rs`: data (`tasks.rs` also parses due dates)
- `prompt.rs`, `picker.rs`, `task_edit.rs`: state of the prompt, fuzzy picker and edit panel
- `tui.rs`, `tui/`: rendering
- `config.rs`: API token storage

Before committing: `cargo fmt && cargo clippy --all-targets && cargo test`.

## Contributing

Contributions are welcome! Please feel free to submit a pull request.

## Disclaimer

This project is a personal project and is not affiliated with, maintained, authorized, endorsed or sponsored by the Todoist company or any of its affiliates.
