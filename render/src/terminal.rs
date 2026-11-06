use std::mem;

/// Default cell size (in pixels) used when the terminal cannot report pixels.
pub const DEFAULT_CELL_WIDTH: u16 = 10;
pub const DEFAULT_CELL_HEIGHT: u16 = 20;

pub fn get_winsize_cells() -> Option<(u16, u16)> {
    // SAFETY: TIOCGWINSZ ioctl only queries the terminal emulator window dimensions
    // and writes to our locally allocated libc::winsize struct.
    unsafe {
        let mut ws: libc::winsize = mem::zeroed();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) == 0
            && ws.ws_col > 0
            && ws.ws_row > 0
        {
            return Some((ws.ws_col, ws.ws_row));
        }
    }
    None
}

pub fn get_cell_size() -> Option<(u16, u16)> {
    // SAFETY: TIOCGWINSZ ioctl safely queries the terminal emulator window dimensions in pixels.
    unsafe {
        let mut ws: libc::winsize = mem::zeroed();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) == 0
            && ws.ws_col > 0
            && ws.ws_row > 0
            && ws.ws_xpixel > 0
            && ws.ws_ypixel > 0
        {
            return Some((ws.ws_xpixel / ws.ws_col, ws.ws_ypixel / ws.ws_row));
        }
    }
    None
}

/// Returns (cell_w, cell_h) in pixels, defaulting to (10, 20) if the terminal
/// does not report pixel sizes. This is the single source of truth for the
/// fallback cell size used by every renderer.
pub fn cell_size_or_default() -> (u16, u16) {
    match get_cell_size() {
        Some((w, h)) if w > 0 && h > 0 => (w, h),
        _ => (DEFAULT_CELL_WIDTH, DEFAULT_CELL_HEIGHT),
    }
}

fn env_flag_is_set(get: &impl Fn(&str) -> Option<String>, key: &str) -> bool {
    get(key).is_some()
}

fn env_value_contains(get: &impl Fn(&str) -> Option<String>, key: &str, needle: &str) -> bool {
    get(key)
        .map(|value| value.to_lowercase().contains(needle))
        .unwrap_or(false)
}

/// NO_COLOR only disables colour (and image protocols) when it has a
/// non-empty value. An empty value is treated as unset.
fn no_color_requested(get: &impl Fn(&str) -> Option<String>) -> bool {
    get("NO_COLOR")
        .map(|value| !value.is_empty())
        .unwrap_or(false)
}

fn dumb_terminal(get: &impl Fn(&str) -> Option<String>) -> bool {
    get("TERM").as_deref() == Some("dumb")
}

fn colors_disabled(get: &impl Fn(&str) -> Option<String>) -> bool {
    no_color_requested(get) || dumb_terminal(get)
}

/// Pure kitty-graphics capability detection. Kept free of environment access so
/// it can be unit tested with a fake environment.
pub(crate) fn detect_kitty(get: impl Fn(&str) -> Option<String>, is_tty: bool) -> bool {
    if !is_tty || colors_disabled(&get) {
        return false;
    }
    if env_flag_is_set(&get, "KITTY_WINDOW_ID")
        || env_flag_is_set(&get, "KITTY_PID")
        || env_flag_is_set(&get, "GHOSTTY_RESOURCES_DIR")
        || env_flag_is_set(&get, "WEZTERM_PANE")
        || env_flag_is_set(&get, "RIO_VERSION")
    {
        return true;
    }
    if env_value_contains(&get, "TERM", "kitty") || env_value_contains(&get, "TERM", "ghostty") {
        return true;
    }
    if let Some(program) = get("TERM_PROGRAM") {
        let program = program.to_lowercase();
        if program.contains("ghostty") || program.contains("wezterm") || program.contains("rio") {
            return true;
        }
    }
    false
}

/// Pure sixel capability detection.
pub(crate) fn detect_sixel(get: impl Fn(&str) -> Option<String>, is_tty: bool) -> bool {
    if !is_tty || colors_disabled(&get) {
        return false;
    }
    if env_flag_is_set(&get, "SIXEL_SUPPORT")
        || env_flag_is_set(&get, "FOOT_SERVER_PATH")
        || env_flag_is_set(&get, "CONTOUR_VERSION")
    {
        return true;
    }
    if let Some(term) = get("TERM") {
        let term = term.to_lowercase();
        if term.contains("foot")
            || term.contains("mlterm")
            || term.contains("yaft")
            || term.contains("sixel")
        {
            return true;
        }
    }
    if let Some(program) = get("TERM_PROGRAM") {
        let program = program.to_lowercase();
        if program.contains("foot")
            || program.contains("wezterm")
            || program.contains("contour")
            || program.contains("mlterm")
        {
            return true;
        }
    }
    false
}

/// Pure iTerm2 inline-image capability detection.
///
/// Only iTerm2 itself is trusted: LC_TERMINAL or a TERM_PROGRAM containing
/// "iterm". Terminal-specific env vars that do not implement OSC 1337 (for
/// example KONSOLE_VERSION) must not opt in.
pub(crate) fn detect_iterm(get: impl Fn(&str) -> Option<String>, is_tty: bool) -> bool {
    if !is_tty || colors_disabled(&get) {
        return false;
    }
    if get("LC_TERMINAL").as_deref() == Some("iTerm2") {
        return true;
    }
    if let Some(program) = get("TERM_PROGRAM") {
        if program.to_lowercase().contains("iterm") {
            return true;
        }
    }
    false
}

fn stdout_is_tty() -> bool {
    use std::io::IsTerminal;
    std::io::stdout().is_terminal()
}

pub fn supports_kitty() -> bool {
    detect_kitty(|key| std::env::var(key).ok(), stdout_is_tty())
}

pub fn supports_sixel() -> bool {
    detect_sixel(|key| std::env::var(key).ok(), stdout_is_tty())
}

pub fn supports_iterm() -> bool {
    detect_iterm(|key| std::env::var(key).ok(), stdout_is_tty())
}

pub fn is_tmux() -> bool {
    if std::env::var("TMUX").is_ok() {
        return true;
    }
    if let Ok(term) = std::env::var("TERM") {
        if term.starts_with("tmux") || term.starts_with("screen") {
            return true;
        }
    }
    false
}

pub fn wrap_tmux_passthrough(seq: &str) -> String {
    if !is_tmux() {
        return seq.to_string();
    }
    wrap_tmux_passthrough_always(seq)
}

pub fn wrap_tmux_passthrough_always(seq: &str) -> String {
    let mut out = String::with_capacity(seq.len() + 32);
    out.push_str("\x1bPtmux;");
    for ch in seq.chars() {
        if ch == '\x1b' {
            out.push_str("\x1b\x1b");
        } else {
            out.push(ch);
        }
    }
    out.push_str("\x1b\\");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_env(vars: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |key: &str| {
            vars.iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| (*value).to_string())
        }
    }

    #[test]
    fn test_wrap_tmux_passthrough_always() {
        let kitty_cmd = "\x1b_Ga=d,d=a\x1b\\";
        let wrapped = wrap_tmux_passthrough_always(kitty_cmd);
        assert_eq!(wrapped, "\x1bPtmux;\x1b\x1b_Ga=d,d=a\x1b\x1b\\\x1b\\");

        let sixel_cmd = "\x1bPq\"1;1;2;1\x1b\\";
        let wrapped_sixel = wrap_tmux_passthrough_always(sixel_cmd);
        assert_eq!(
            wrapped_sixel,
            "\x1bPtmux;\x1b\x1bPq\"1;1;2;1\x1b\x1b\\\x1b\\"
        );
    }

    #[test]
    fn kitty_detection_requires_tty_and_known_terminals() {
        assert!(!detect_kitty(fake_env(&[]), true));
        assert!(!detect_kitty(fake_env(&[]), false));
        assert!(detect_kitty(fake_env(&[("KITTY_WINDOW_ID", "1")]), true));
        assert!(detect_kitty(fake_env(&[("TERM", "xterm-kitty")]), true));
        assert!(detect_kitty(fake_env(&[("TERM_PROGRAM", "WezTerm")]), true));
        // wayst is a sixel terminal, not a kitty terminal.
        assert!(!detect_kitty(fake_env(&[("TERM", "wayst")]), true));
        // Empty NO_COLOR does not disable anything.
        assert!(detect_kitty(
            fake_env(&[("TERM", "xterm-kitty"), ("NO_COLOR", "")]),
            true
        ));
        assert!(!detect_kitty(
            fake_env(&[("TERM", "xterm-kitty"), ("NO_COLOR", "1")]),
            true
        ));
    }

    #[test]
    fn sixel_detection_rejects_non_terminal_names() {
        assert!(detect_sixel(fake_env(&[("TERM", "foot")]), true));
        assert!(detect_sixel(fake_env(&[("TERM_PROGRAM", "contour")]), true));
        // chafa is an image converter, not a terminal.
        assert!(!detect_sixel(fake_env(&[("TERM", "chafa")]), true));
        assert!(!detect_sixel(fake_env(&[("TERM", "xterm-chafa")]), true));
    }

    #[test]
    fn iterm_detection_only_trusts_iterm_and_lc_terminal() {
        assert!(detect_iterm(fake_env(&[("LC_TERMINAL", "iTerm2")]), true));
        assert!(detect_iterm(
            fake_env(&[("TERM_PROGRAM", "iTerm.app")]),
            true
        ));
        // Konsole does not implement OSC 1337.
        assert!(!detect_iterm(
            fake_env(&[("KONSOLE_VERSION", "230800")]),
            true
        ));
        assert!(!detect_iterm(
            fake_env(&[("TERM_PROGRAM", "WezTerm")]),
            true
        ));
        assert!(!detect_iterm(fake_env(&[]), true));
        assert!(!detect_iterm(fake_env(&[("LC_TERMINAL", "iTerm2")]), false));
    }

    #[test]
    fn empty_no_color_is_ignored() {
        assert!(!colors_disabled(&fake_env(&[("NO_COLOR", "")])));
        assert!(colors_disabled(&fake_env(&[("NO_COLOR", "1")])));
        assert!(colors_disabled(&fake_env(&[("TERM", "dumb")])));
    }

    #[test]
    fn default_cell_size_is_shared() {
        // Must never return a zero cell dimension.
        let (w, h) = cell_size_or_default();
        assert!(w > 0 && h > 0);
        assert_eq!((DEFAULT_CELL_WIDTH, DEFAULT_CELL_HEIGHT), (10, 20));
    }
}
