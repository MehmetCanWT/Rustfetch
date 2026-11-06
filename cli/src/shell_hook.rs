//! Shell startup hooks and initialization scripts for bash, zsh, and fish.
//!
//! Hook management is conservative by design:
//!
//! * the managed block is delimited by explicit markers, so user content is
//!   never parsed as part of it;
//! * a timestamped backup is written before the file is modified;
//! * invalid UTF-8 is refused instead of truncated;
//! * writes are atomic (create_new temp file + sync_all + rename);
//! * legacy (unmarked) blocks are only removed when they are syntactically
//!   balanced and start with a heading RustFetch actually wrote.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// First line of the managed block.
pub const HOOK_START_MARKER: &str = "# >>> rustfetch hook >>>";
/// Last line of the managed block.
pub const HOOK_END_MARKER: &str = "# <<< rustfetch hook <<<";

/// Headings written by RustFetch versions that did not use markers.
///
/// Only an exact match counts as a legacy block: an arbitrary user comment that
/// merely starts with "# RustFetch" must never be interpreted as our block.
const LEGACY_HEADINGS: &[&str] = &[
    "# RustFetch — Shell startup integration",
    "# RustFetch — Automatic responsive redraw for tiling WMs (MangoWM, Hyprland, Sway, etc.)",
];

pub fn shell_init_script(shell: &str) -> Option<&'static str> {
    match shell.to_lowercase().as_str() {
        "bash" => Some(
            r#"# RustFetch — Shell startup integration
if [[ $- == *i* ]]; then
    _rustfetch_init_hist=""
    _rustfetch_last_size="$(stty size 2>/dev/null)"
    _rustfetch_prompt_cmd() {
        if [ -z "$_rustfetch_init_hist" ]; then
            _rustfetch_init_hist=$HISTCMD
        elif [ "$HISTCMD" -gt "$_rustfetch_init_hist" ]; then
            trap - WINCH
        fi
    }
    if (( BASH_VERSINFO[0] > 4 || ( BASH_VERSINFO[0] == 4 && BASH_VERSINFO[1] >= 4 ) )); then
        PROMPT_COMMAND+=(_rustfetch_prompt_cmd)
    else
        PROMPT_COMMAND="_rustfetch_prompt_cmd${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
    fi
    _rustfetch_on_resize() {
        if [ -z "$_rustfetch_init_hist" ] || [ "$HISTCMD" -le "$_rustfetch_init_hist" ]; then
            if [ -z "${READLINE_LINE-}" ]; then
                local cur_size
                while :; do
                    cur_size="$(stty size 2>/dev/null)"
                    if [ -z "$cur_size" ] || [ "$cur_size" = "$_rustfetch_last_size" ]; then
                        break
                    fi
                    _rustfetch_last_size="$cur_size"
                    rustfetch --keep-logo --no-update-check
                done
                if (( BASH_VERSINFO[0] > 4 || ( BASH_VERSINFO[0] == 4 && BASH_VERSINFO[1] >= 4 ) )); then
                    local _rf_ps1
                    eval '_rf_ps1=${PS1@P}'
                    printf '%s' "${_rf_ps1//[$'\001\002']/}"
                else
                    printf '%s' "${PS1//[$'\001\002']/}"
                fi
            fi
        else
            trap - WINCH
        fi
    }
    trap '_rustfetch_on_resize' WINCH
    rustfetch
fi
"#,
        ),
        "zsh" => Some(
            r#"# RustFetch — Shell startup integration
if [[ -o interactive ]]; then
    _rustfetch_startup=1
    _rustfetch_last_size="$(stty size 2>/dev/null)"
    _rustfetch_preexec() {
        _rustfetch_startup=0
    }
    autoload -Uz add-zsh-hook 2>/dev/null && add-zsh-hook preexec _rustfetch_preexec 2>/dev/null
    typeset -g _rustfetch_has_prev_winch=0
    if (( ${+functions[TRAPWINCH]} )); then
        functions[_rustfetch_prev_trapwinch]="${functions[TRAPWINCH]}"
        _rustfetch_has_prev_winch=1
    fi
    TRAPWINCH() {
        if [[ "$_rustfetch_startup" == "1" && -z "$BUFFER" ]]; then
            local cur_size
            while true; do
                cur_size="$(stty size 2>/dev/null)"
                if [[ -z "$cur_size" || "$cur_size" == "$_rustfetch_last_size" ]]; then
                    break
                fi
                _rustfetch_last_size="$cur_size"
                rustfetch --keep-logo --no-update-check
            done
            zle && zle reset-prompt 2>/dev/null
        fi
        if (( _rustfetch_has_prev_winch )); then
            _rustfetch_prev_trapwinch
        fi
    }
    rustfetch
fi
"#,
        ),
        "fish" => Some(
            r#"# RustFetch — Shell startup integration
if status is-interactive
    set -g _rustfetch_startup 1
    set -g _rustfetch_last_size (stty size 2>/dev/null)
    function _rustfetch_preexec --on-event fish_preexec
        set -g _rustfetch_startup 0
    end
    function _rustfetch_on_resize --on-signal WINCH
        if test "$_rustfetch_startup" = "1" -a -z (commandline)
            set -l cur_size (stty size 2>/dev/null)
            if test -n "$cur_size" -a "$cur_size" != "$_rustfetch_last_size"
                set -g _rustfetch_last_size "$cur_size"
                rustfetch --keep-logo --no-update-check
                commandline -f repaint 2>/dev/null
            end
        end
    end
    rustfetch
end
"#,
        ),
        _ => None,
    }
}

pub fn get_shell_target_file(home: &std::path::Path) -> (&'static str, std::path::PathBuf) {
    let shell_path = std::env::var("SHELL").unwrap_or_default();
    let is_zsh = shell_path.ends_with("/zsh") || shell_path == "zsh";
    let is_fish = shell_path.ends_with("/fish") || shell_path == "fish";

    if is_zsh {
        ("zsh", home.join(".zshrc"))
    } else if is_fish {
        ("fish", home.join(".config/fish/config.fish"))
    } else {
        ("bash", home.join(".bashrc"))
    }
}

fn line_without_ending(segment: &str) -> &str {
    segment.trim_end_matches(['\n', '\r'])
}

fn line_trimmed(segment: &str) -> &str {
    line_without_ending(segment).trim()
}

fn is_legacy_heading(segment: &str) -> bool {
    let trimmed = line_trimmed(segment);
    LEGACY_HEADINGS.contains(&trimmed)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BlockKind {
    If,
    Loop,
    Case,
    Brace,
    FishEnd,
}

fn opener_kind(trimmed: &str) -> Option<BlockKind> {
    if trimmed.starts_with("if ") || trimmed.starts_with("if[") || trimmed.starts_with("if(") {
        Some(BlockKind::If)
    } else if trimmed.starts_with("while ") || trimmed.starts_with("for ") {
        Some(BlockKind::Loop)
    } else if trimmed.starts_with("case ") {
        Some(BlockKind::Case)
    } else if trimmed.starts_with("function ") {
        // fish functions are closed by "end".
        Some(BlockKind::FishEnd)
    } else if trimmed.ends_with('{') {
        // bash/zsh function definition: name() {
        Some(BlockKind::Brace)
    } else {
        None
    }
}

fn closer_kind(trimmed: &str) -> Option<BlockKind> {
    if trimmed == "fi" || trimmed.starts_with("fi ") || trimmed.starts_with("fi;") {
        Some(BlockKind::If)
    } else if trimmed == "done" || trimmed.starts_with("done ") || trimmed.starts_with("done;") {
        Some(BlockKind::Loop)
    } else if trimmed == "esac" || trimmed.starts_with("esac ") || trimmed.starts_with("esac;") {
        Some(BlockKind::Case)
    } else if trimmed == "end" || trimmed.starts_with("end ") || trimmed.starts_with("end;") {
        Some(BlockKind::FishEnd)
    } else if trimmed == "}" || trimmed.starts_with("} ") || trimmed.starts_with("};") {
        Some(BlockKind::Brace)
    } else {
        None
    }
}

fn closer_matches(opened: BlockKind, closed: BlockKind) -> bool {
    if closed == BlockKind::FishEnd {
        // In fish, "end" closes if/while/for/case/function.
        matches!(
            opened,
            BlockKind::If | BlockKind::Loop | BlockKind::Case | BlockKind::FishEnd
        )
    } else {
        opened == closed
    }
}

/// Returns the index (inclusive) of the closing line of a balanced legacy block,
/// or None when the block is not clearly balanced.
fn legacy_block_end(lines: &[&str], heading_index: usize) -> Option<usize> {
    let mut stack: Vec<BlockKind> = Vec::new();
    for (offset, line) in lines[heading_index + 1..].iter().enumerate() {
        let trimmed = line_trimmed(line);
        if trimmed.is_empty() {
            continue;
        }
        if let Some(closed) = closer_kind(trimmed) {
            let opened = stack.pop()?;
            if !closer_matches(opened, closed) {
                return None;
            }
            if stack.is_empty() {
                return Some(heading_index + 1 + offset);
            }
            continue;
        }
        if let Some(opened) = opener_kind(trimmed) {
            stack.push(opened);
            continue;
        }
        if stack.is_empty() {
            // A real hook block starts with a compound command right after the
            // heading; anything else is user content we must not delete.
            return None;
        }
    }
    None
}

fn find_marker_end(lines: &[&str]) -> Option<usize> {
    lines
        .iter()
        .position(|segment| line_trimmed(segment) == HOOK_END_MARKER)
}

fn drop_previous_blank(lines: &mut Vec<&str>) {
    if lines
        .last()
        .map(|segment| line_trimmed(segment).is_empty())
        .unwrap_or(false)
    {
        lines.pop();
    }
}

/// Removes the RustFetch hook from the content, preserving everything else
/// byte-for-byte (including line endings).
///
/// * A marker block (HOOK_START_MARKER .. HOOK_END_MARKER) is removed.
/// * Without markers, only a balanced legacy block introduced by one of
///   LEGACY_HEADINGS is removed.
/// * Lines outside a block are never removed, even when they mention
///   rustfetch, _rustfetch or TRAPWINCH.
pub fn strip_rustfetch_hook(content: &str) -> String {
    let lines: Vec<&str> = content.split_inclusive('\n').collect();

    let mut kept: Vec<&str> = Vec::with_capacity(lines.len());
    let mut index = 0;
    while index < lines.len() {
        if line_trimmed(lines[index]) == HOOK_START_MARKER {
            if let Some(relative_end) = find_marker_end(&lines[index + 1..]) {
                index += relative_end + 2;
                drop_previous_blank(&mut kept);
                continue;
            }
            // Unpaired start marker: keep it, never delete user content on a
            // best guess.
        }
        kept.push(lines[index]);
        index += 1;
    }

    let mut result: Vec<&str> = Vec::with_capacity(kept.len());
    let mut index = 0;
    while index < kept.len() {
        if is_legacy_heading(kept[index]) {
            if let Some(end) = legacy_block_end(&kept, index) {
                index = end + 1;
                drop_previous_blank(&mut result);
                continue;
            }
        }
        result.push(kept[index]);
        index += 1;
    }
    result.concat()
}

fn detect_newline(content: &str) -> &'static str {
    if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

fn convert_newlines(text: &str, newline: &str) -> String {
    if newline == "\n" {
        text.to_string()
    } else {
        text.replace('\n', newline)
    }
}

fn compose_with_block(cleaned: &str, script: &str) -> String {
    let newline = detect_newline(cleaned);
    let mut out = String::with_capacity(cleaned.len() + script.len() + 128);
    out.push_str(cleaned);
    if !out.is_empty() {
        if !out.ends_with('\n') {
            out.push_str(newline);
        }
        out.push_str(newline);
    }
    out.push_str(HOOK_START_MARKER);
    out.push_str(newline);
    out.push_str(&convert_newlines(script, newline));
    if !out.ends_with('\n') {
        out.push_str(newline);
    }
    out.push_str(HOOK_END_MARKER);
    out.push_str(newline);
    out
}

fn home_dir() -> Result<PathBuf, String> {
    match std::env::var_os("HOME") {
        Some(value) if !value.is_empty() => Ok(PathBuf::from(value)),
        _ => Err("HOME environment variable is not set".to_string()),
    }
}

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn timestamp_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn read_existing_utf8(path: &Path) -> Result<Option<String>, String> {
    match fs::read(path) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(text) => Ok(Some(text)),
            Err(_) => Err(format!(
                "{} is not valid UTF-8; refusing to modify it (convert it to UTF-8 first)",
                path.display()
            )),
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(format!("Failed to read {}: {err}", path.display())),
    }
}

fn backup_file(path: &Path) -> Result<Option<PathBuf>, String> {
    match fs::metadata(path) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("Failed to inspect {}: {err}", path.display())),
        Ok(meta) if meta.len() == 0 => return Ok(None),
        Ok(_) => {}
    }
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("Invalid shell configuration path: {}", path.display()))?;
    let backup = path.with_file_name(format!("{name}.{}.bak", timestamp_seconds()));
    fs::copy(path, &backup)
        .map_err(|err| format!("Failed to create backup {}: {err}", backup.display()))?;
    Ok(Some(backup))
}

pub(crate) fn write_atomic(path: &Path, content: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    fs::create_dir_all(&parent)
        .map_err(|err| format!("Failed to create {}: {err}", parent.display()))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("Invalid shell configuration path: {}", path.display()))?;
    let temp = parent.join(format!(
        ".{name}.rustfetch.{}.{}.tmp",
        std::process::id(),
        now_nanos()
    ));

    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temp)
        .map_err(|err| format!("Failed to create temporary file {}: {err}", temp.display()))?;
    let written = file
        .write_all(content.as_bytes())
        .and_then(|()| file.sync_all());
    if let Err(err) = written {
        drop(file);
        let _ = fs::remove_file(&temp);
        return Err(format!("Failed to write {}: {err}", path.display()));
    }
    #[cfg(unix)]
    {
        if let Ok(meta) = fs::metadata(path) {
            if let Err(err) = fs::set_permissions(&temp, meta.permissions()) {
                drop(file);
                let _ = fs::remove_file(&temp);
                return Err(format!(
                    "Failed to preserve permissions for {}: {err}",
                    path.display()
                ));
            }
        }
    }
    drop(file);
    if let Err(err) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(format!("Failed to replace {}: {err}", path.display()));
    }
    #[cfg(unix)]
    {
        if let Ok(directory) = fs::File::open(&parent) {
            let _ = directory.sync_all();
        }
    }
    Ok(())
}

fn install_hook_at(target: &Path, shell: &str) -> Result<Option<PathBuf>, String> {
    let script = shell_init_script(shell)
        .ok_or_else(|| format!("Unsupported shell '{shell}'. Supported shells: bash, zsh, fish"))?;
    let existing = read_existing_utf8(target)?;
    let backup = if existing.is_some() {
        backup_file(target)?
    } else {
        None
    };
    let cleaned = strip_rustfetch_hook(existing.as_deref().unwrap_or(""));
    let content = compose_with_block(&cleaned, script);
    write_atomic(target, &content)?;
    Ok(backup)
}

enum UninstallOutcome {
    Missing,
    NotInstalled,
    Removed { backup: Option<PathBuf> },
}

fn uninstall_hook_at(target: &Path) -> Result<UninstallOutcome, String> {
    let Some(existing) = read_existing_utf8(target)? else {
        return Ok(UninstallOutcome::Missing);
    };
    let cleaned = strip_rustfetch_hook(&existing);
    if cleaned == existing {
        return Ok(UninstallOutcome::NotInstalled);
    }
    let backup = backup_file(target)?;
    write_atomic(target, &cleaned)?;
    Ok(UninstallOutcome::Removed { backup })
}

pub fn install_shell_hook() {
    let result = home_dir().and_then(|home| {
        let (shell, target) = get_shell_target_file(&home);
        install_hook_at(&target, shell).map(|backup| (target, backup))
    });
    match result {
        Ok((target, backup)) => {
            let mut message = format!(
                "Successfully installed RustFetch startup hook to {}",
                target.display()
            );
            if let Some(backup) = backup {
                message.push_str(&format!(" (backup: {})", backup.display()));
            }
            println!("{message}");
        }
        Err(err) => {
            eprintln!("Error: {err}");
            std::process::exit(1);
        }
    }
}

pub fn uninstall_shell_hook() {
    let result = home_dir().and_then(|home| {
        let (_, target) = get_shell_target_file(&home);
        uninstall_hook_at(&target).map(|outcome| (target, outcome))
    });
    match result {
        Ok((target, UninstallOutcome::Missing)) => {
            println!("No shell configuration found at {}", target.display());
        }
        Ok((target, UninstallOutcome::NotInstalled)) => {
            println!(
                "No RustFetch hook found in {}; nothing to remove",
                target.display()
            );
        }
        Ok((target, UninstallOutcome::Removed { backup })) => {
            let mut message = format!(
                "Successfully removed RustFetch hook from {}",
                target.display()
            );
            if let Some(backup) = backup {
                message.push_str(&format!(" (backup: {})", backup.display()));
            }
            println!("{message}");
        }
        Err(err) => {
            eprintln!("Error: {err}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rustfetch-hook-{tag}-{}-{}",
            std::process::id(),
            now_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn read(path: &Path) -> String {
        fs::read_to_string(path).unwrap()
    }

    fn backup_files(dir: &Path) -> Vec<PathBuf> {
        fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.to_string_lossy().ends_with(".bak"))
            .collect()
    }

    #[test]
    fn user_comment_and_trapwinch_are_preserved() {
        let input = r#"# my bashrc
export PATH="$HOME/.local/bin:$PATH"

# RustFetch notes: I keep my personal settings for the tool
export RUSTFETCH_LOGO_DIR="$HOME/Pictures/logos"
alias ll='ls -la'
alias rfu='rfetch --no-logo'

TRAPWINCH() {
    echo "my own resize handler"
}
"#;
        assert_eq!(strip_rustfetch_hook(input), input);
    }

    #[test]
    fn unrelated_rustfetch_lines_are_preserved() {
        let input = "alias rfu='rfetch --no-logo'\n_rustfetch_helper() { :; }\n";
        assert_eq!(strip_rustfetch_hook(input), input);
    }

    #[test]
    fn marked_block_is_removed_and_strip_is_stable() {
        let original = "export A=1\n";
        let script = shell_init_script("bash").unwrap();
        let installed = compose_with_block(original, script);
        assert!(installed.contains(HOOK_START_MARKER));
        assert!(installed.contains(HOOK_END_MARKER));
        let stripped = strip_rustfetch_hook(&installed);
        assert_eq!(stripped, original);
        assert_eq!(strip_rustfetch_hook(&stripped), stripped);
    }

    #[test]
    fn legacy_balanced_block_is_removed() {
        let input = r#"# .bashrc
export PATH="$HOME/.local/bin:$PATH"

# RustFetch — Automatic responsive redraw for tiling WMs (MangoWM, Hyprland, Sway, etc.)
if [[ $- == *i* ]]; then
    _rustfetch_last_size=""
    _rustfetch_on_resize() {
        local cur_size="$(stty size 2>/dev/null)"
        if [ -n "$cur_size" ]; then
            clear
            rustfetch
        fi
    }
    trap '_rustfetch_on_resize' WINCH
    rustfetch
fi

alias ll='ls -la'
"#;
        let stripped = strip_rustfetch_hook(input);
        assert!(!stripped.contains("_rustfetch"));
        assert!(!stripped.contains("WINCH"));
        assert!(stripped.contains(r#"export PATH="$HOME/.local/bin:$PATH""#));
        assert!(stripped.contains("alias ll='ls -la'"));
    }

    #[test]
    fn unbalanced_legacy_block_is_kept() {
        let input =
            "# RustFetch — Shell startup integration\nif [ -n \"$PS1\" ]; then\nexport KEEP=1\n";
        assert_eq!(strip_rustfetch_hook(input), input);
    }

    #[test]
    fn install_is_idempotent_and_writes_backup() {
        let dir = temp_dir("idempotent");
        let target = dir.join(".bashrc");
        fs::write(&target, "export A=1\n").unwrap();

        let backup = install_hook_at(&target, "bash").unwrap();
        assert!(backup.is_some(), "existing content must be backed up");
        let first = read(&target);
        assert!(first.contains(HOOK_START_MARKER));
        assert!(first.contains("export A=1"));

        let backup2 = install_hook_at(&target, "bash").unwrap();
        assert!(backup2.is_some());
        assert_eq!(read(&target), first, "second install must be a no-op");
        assert_eq!(
            backup_files(&dir).len(),
            1,
            "one timestamped backup expected"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn install_refuses_non_utf8_without_touching_the_file() {
        let dir = temp_dir("nonutf8");
        let target = dir.join(".bashrc");
        let original = b"# my rc \xff\xfe legacy latin1 line\nexport KEEP=1\n";
        fs::write(&target, original).unwrap();

        let error = install_hook_at(&target, "bash").unwrap_err();
        assert!(
            error.contains("not valid UTF-8"),
            "unexpected error: {error}"
        );
        assert_eq!(fs::read(&target).unwrap(), original);
        assert!(backup_files(&dir).is_empty(), "no backup on refused write");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn uninstall_restores_the_original_file() {
        let dir = temp_dir("roundtrip");
        let target = dir.join(".zshrc");
        let original = "# zshrc\nexport PATH=\"$HOME/.local/bin:$PATH\"\nalias g=git\n";
        fs::write(&target, original).unwrap();

        install_hook_at(&target, "zsh").unwrap();
        assert_ne!(read(&target), original);
        match uninstall_hook_at(&target).unwrap() {
            UninstallOutcome::Removed { backup } => assert!(backup.is_some()),
            _ => panic!("hook should have been removed"),
        }
        assert_eq!(read(&target), original);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn crlf_line_endings_are_preserved() {
        let dir = temp_dir("crlf");
        let target = dir.join(".bashrc");
        let original = "export A=1\r\nexport B=2\r\n";
        fs::write(&target, original).unwrap();

        install_hook_at(&target, "bash").unwrap();
        let installed = read(&target);
        assert_eq!(
            installed.matches('\n').count(),
            installed.matches("\r\n").count(),
            "installed file must not mix line endings"
        );
        assert!(installed.contains("export A=1\r\n"));
        assert!(installed.contains(&format!("{HOOK_START_MARKER}\r\n")));

        match uninstall_hook_at(&target).unwrap() {
            UninstallOutcome::Removed { .. } => {}
            _ => panic!("hook should have been removed"),
        }
        assert_eq!(read(&target), original);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn zsh_hook_chains_previous_trapwinch() {
        let script = shell_init_script("zsh").unwrap();
        assert!(script.contains("_rustfetch_prev_trapwinch"));
        assert!(script.contains("functions[TRAPWINCH]"));
    }

    #[test]
    fn bash_hook_guards_old_bash_versions() {
        let script = shell_init_script("bash").unwrap();
        assert!(script.contains("BASH_VERSINFO"));
        assert!(script.contains("PROMPT_COMMAND+=(_rustfetch_prompt_cmd)"));
        assert!(script.contains("eval '_rf_ps1="));
    }
}
