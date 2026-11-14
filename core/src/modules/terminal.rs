use crate::{Info, Module};

pub struct Terminal;

impl Module for Terminal {
    fn name(&self) -> &'static str {
        "terminal"
    }

    fn detect(&self) -> Option<Info> {
        let terminal = read_terminal()?;
        Some(Info::new("Terminal", terminal))
    }
}

fn read_terminal() -> Option<String> {
    let deferred_mux = std::env::var("TERM_PROGRAM").ok().and_then(|p| {
        let trimmed = p.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    if let Some(ref prog) = deferred_mux
        && !matches!(prog.to_ascii_lowercase().as_str(), "tmux" | "screen")
    {
        return Some(format_terminal_name(prog));
    }
    if std::env::var("GHOSTTY_RESOURCES_DIR").is_ok() {
        return Some("Ghostty".to_string());
    }
    if std::env::var("KITTY_WINDOW_ID").is_ok() || std::env::var("KITTY_PID").is_ok() {
        return Some("kitty".to_string());
    }
    if std::env::var("ALACRITTY_WINDOW_ID").is_ok() || std::env::var("ALACRITTY_SOCKET").is_ok() {
        return Some("Alacritty".to_string());
    }
    if std::env::var("WEZTERM_PANE").is_ok() || std::env::var("WEZTERM_EXECUTABLE").is_ok() {
        return Some("WezTerm".to_string());
    }
    if std::env::var("FOOT_SERVER_PATH").is_ok() {
        return Some("foot".to_string());
    }
    if std::env::var("KONSOLE_VERSION").is_ok() {
        return Some("Konsole".to_string());
    }
    if std::env::var("PTYXIS_VERSION").is_ok() {
        return Some("Ptyxis".to_string());
    }
    if std::env::var("KGX_VERSION").is_ok() {
        return Some("GNOME Console".to_string());
    }
    if std::env::var("WT_SESSION").is_ok() || std::env::var("WT_PROFILE_ID").is_ok() {
        return Some("Windows Terminal".to_string());
    }
    if std::env::var("TILIX_ID").is_ok() {
        return Some("Tilix".to_string());
    }
    if std::env::var("BLACKBOX_VERSION").is_ok() {
        return Some("BlackBox".to_string());
    }
    if std::env::var("RIO_VERSION").is_ok() {
        return Some("Rio".to_string());
    }
    if std::env::var("CONTOUR_VERSION").is_ok() {
        return Some("Contour".to_string());
    }
    if std::env::var("GUAKE_PID").is_ok() {
        return Some("Guake".to_string());
    }
    if std::env::var("TERMINOLOGY").is_ok() {
        return Some("Terminology".to_string());
    }
    if std::env::var("TABBY_CONFIG_DIRECTORY").is_ok() {
        return Some("Tabby".to_string());
    }
    if std::env::var("HYPER_TERM").is_ok() {
        return Some("Hyper".to_string());
    }
    if std::env::var("WARP_IS_LOCAL_SHELL_SESSION").is_ok() {
        return Some("Warp".to_string());
    }
    if let Ok(emu) = std::env::var("TERMINAL_EMULATOR") {
        let trimmed = emu.trim();
        if !trimmed.is_empty() {
            return Some(format_terminal_name(trimmed));
        }
    }
    #[cfg(target_os = "linux")]
    if let Some(proc_name) = detect_terminal_from_process() {
        return Some(proc_name);
    }
    if let Some(mux) = deferred_mux {
        return Some(format_terminal_name(&mux));
    }
    // VTE_VERSION is exported by every VTE-based terminal (Ptyxis, Tilix,
    // Terminator, XFCE Terminal, ...), so it only identifies the generic VTE
    // family and only after the process tree failed to name a concrete emulator.
    if std::env::var("VTE_VERSION").is_ok() {
        return Some("VTE".to_string());
    }
    parse_terminal(None, None, None, std::env::var("TERM").ok().as_deref())
}

pub fn format_terminal_name(raw: &str) -> String {
    match raw.to_lowercase().as_str() {
        "vscode" => "Visual Studio Code".to_string(),
        "iterm.app" => "iTerm2".to_string(),
        "apple_terminal" => "Apple Terminal".to_string(),
        "ghostty" => "Ghostty".to_string(),
        "wezterm" => "WezTerm".to_string(),
        "warpterminal" => "Warp".to_string(),
        "alacritty" => "Alacritty".to_string(),
        "kitty" => "kitty".to_string(),
        "foot" => "foot".to_string(),
        "konsole" => "Konsole".to_string(),
        "tabby" => "Tabby".to_string(),
        _ => raw.to_string(),
    }
}

/// Maps a `/proc/<pid>/comm` value to a terminal name.
///
/// The kernel truncates command names to 15 characters, so long names such as
/// `gnome-terminal-server` appear as `gnome-terminal-`; prefix matching keeps
/// those probes working.
pub fn match_terminal_comm(name: &str) -> Option<&'static str> {
    let name = name.trim().to_lowercase();
    if name.is_empty() {
        return None;
    }
    for (prefix, terminal) in [
        ("gnome-terminal", "GNOME Terminal"),
        ("ptyxis", "Ptyxis"),
        ("kgx", "GNOME Console"),
        ("konsole", "Konsole"),
        ("kitty", "kitty"),
        ("alacritty", "Alacritty"),
        ("ghostty", "Ghostty"),
        ("wezterm", "WezTerm"),
        ("footclient", "foot"),
        ("foot", "foot"),
        ("xfce4-terminal", "XFCE Terminal"),
        ("mate-terminal", "MATE Terminal"),
        ("terminator", "Terminator"),
        ("tilix", "Tilix"),
        ("blackbox", "BlackBox"),
        ("contour", "Contour"),
        ("urxvt", "urxvt"),
        ("rxvt-unicode", "urxvt"),
        ("xterm", "xterm"),
        ("qterminal", "QTerminal"),
        ("lxterminal", "LXTerminal"),
        ("guake", "Guake"),
        ("yakuake", "Yakuake"),
        ("tilda", "Tilda"),
        ("tabby", "Tabby"),
        ("warp-terminal", "Warp"),
        ("warp", "Warp"),
        ("hyper", "Hyper"),
        ("rio", "Rio"),
        ("st-256color", "st"),
    ] {
        if name.starts_with(prefix) {
            return Some(terminal);
        }
    }
    if name == "st" {
        return Some("st");
    }
    None
}

#[cfg(target_os = "linux")]
fn detect_terminal_from_process() -> Option<String> {
    use std::fs;
    let mut pid = std::os::unix::process::parent_id();
    for _ in 0..6 {
        if pid <= 1 {
            break;
        }
        if let Ok(comm) = fs::read_to_string(format!("/proc/{pid}/comm"))
            && let Some(name) = match_terminal_comm(&comm)
        {
            return Some(name.to_string());
        }
        let status_path = format!("/proc/{pid}/status");
        if let Ok(status) = fs::read_to_string(&status_path) {
            let mut next_pid = None;
            for line in status.lines() {
                if let Some(rest) = line.strip_prefix("PPid:\t")
                    && let Ok(p) = rest.trim().parse::<u32>()
                {
                    next_pid = Some(p);
                    break;
                }
            }
            match next_pid {
                Some(p) if p != pid => pid = p,
                _ => break,
            }
        } else {
            break;
        }
    }
    None
}

pub fn parse_terminal(
    term_program: Option<&str>,
    kitty_window_id: Option<&str>,
    terminal_emulator: Option<&str>,
    term: Option<&str>,
) -> Option<String> {
    let check = |s: Option<&str>| s.map(str::trim).filter(|s| !s.is_empty()).map(String::from);

    check(term_program)
        .or_else(|| check(kitty_window_id).map(|_| "kitty".to_string()))
        .or_else(|| check(terminal_emulator))
        .or_else(|| {
            term.map(str::trim)
                .filter(|s| !s.is_empty() && *s != "dumb")
                .map(String::from)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_terminal_term_program_precedence() {
        let result = parse_terminal(
            Some("ghostty"),
            Some("123"),
            Some("alacritty"),
            Some("xterm-256color"),
        );
        assert_eq!(result.as_deref(), Some("ghostty"));

        let result_wezterm = parse_terminal(Some("WezTerm"), None, None, Some("xterm-256color"));
        assert_eq!(result_wezterm.as_deref(), Some("WezTerm"));
    }

    #[test]
    fn test_parse_terminal_kitty_window_id() {
        let result = parse_terminal(None, Some("1"), None, Some("xterm-kitty"));
        assert_eq!(result.as_deref(), Some("kitty"));
    }

    #[test]
    fn test_parse_terminal_emulator() {
        let result = parse_terminal(
            None,
            None,
            Some("JetBrains-JediTerm"),
            Some("xterm-256color"),
        );
        assert_eq!(result.as_deref(), Some("JetBrains-JediTerm"));
    }

    #[test]
    fn test_parse_terminal_term_fallback() {
        let result = parse_terminal(None, None, None, Some("xterm-256color"));
        assert_eq!(result.as_deref(), Some("xterm-256color"));
    }

    #[test]
    fn test_parse_terminal_ignores_dumb_and_empty() {
        assert_eq!(parse_terminal(None, None, None, Some("dumb")), None);
        assert_eq!(
            parse_terminal(Some("   "), Some(""), Some(""), Some("dumb")),
            None
        );
        assert_eq!(parse_terminal(None, None, None, None), None);
    }

    #[test]
    fn test_format_terminal_name() {
        assert_eq!(format_terminal_name("vscode"), "Visual Studio Code");
        assert_eq!(format_terminal_name("iTerm.app"), "iTerm2");
        assert_eq!(format_terminal_name("ghostty"), "Ghostty");
        assert_eq!(format_terminal_name("wezterm"), "WezTerm");
        assert_eq!(format_terminal_name("warpterminal"), "Warp");
        assert_eq!(format_terminal_name("custom_term"), "custom_term");
    }

    #[test]
    fn test_match_terminal_comm_handles_15_char_truncation() {
        // /proc/<pid>/comm for gnome-terminal-server is truncated by the kernel.
        assert_eq!(
            match_terminal_comm("gnome-terminal-"),
            Some("GNOME Terminal")
        );
        assert_eq!(
            match_terminal_comm("gnome-terminal-server"),
            Some("GNOME Terminal")
        );
        assert_eq!(match_terminal_comm("welp"), None);
        assert_eq!(match_terminal_comm(""), None);
    }

    #[test]
    fn test_match_terminal_comm_names() {
        assert_eq!(match_terminal_comm("kitty"), Some("kitty"));
        assert_eq!(match_terminal_comm("Alacritty\n"), Some("Alacritty"));
        assert_eq!(match_terminal_comm("xfce4-terminal"), Some("XFCE Terminal"));
        assert_eq!(match_terminal_comm("wezterm-gui"), Some("WezTerm"));
        assert_eq!(match_terminal_comm("st"), Some("st"));
        assert_eq!(match_terminal_comm("st-256color"), Some("st"));
        assert_eq!(match_terminal_comm("code"), None);
    }
}
