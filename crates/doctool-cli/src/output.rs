use std::io::IsTerminal;

/// Match apps/cli: emit JSON when --json is set or stdout is not a TTY.
pub fn use_json(explicit_json: bool) -> bool {
    explicit_json || !std::io::stdout().is_terminal()
}
