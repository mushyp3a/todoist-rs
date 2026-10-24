# Todoist Terminal Client

This is a terminal client for Todoist, (inspired by Spotify-TUI). The project is designed to be a keyboard-driven interface for managing your Todoist tasks directly from your terminal.

  <img src="imgs/app.gif" width="75%">

## Implemented Features

- View and manage your Todoist tasks and projects.
- Create new tasks with due dates and descriptions.
- Edit existing tasks.

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

- `main.rs`: app state, startup and the event loop
- `key_handler.rs`: every key binding, per screen
- `api_calls.rs`: all Todoist API v1 requests; background calls report failures to the error modal
- `tasks.rs`, `projects.rs`, `sections.rs`: data types and list state (`tasks.rs` also parses due dates)
- `new_task.rs`, `task_edit.rs`: state of the two task forms
- `tui.rs`, `tui/`: rendering
- `config.rs`: API token storage

Before committing: `cargo fmt && cargo clippy --all-targets && cargo test`.

## Contributing

Contributions are welcome! Please feel free to submit a pull request.

## Disclaimer

This project is a personal project and is not affiliated with, maintained, authorized, endorsed or sponsored by the Todoist company or any of its affiliates.
