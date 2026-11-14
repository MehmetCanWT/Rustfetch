use crate::logos;
use rustfetch_core::config::Config;
use std::io::Write;
use std::path::{Path, PathBuf};

pub(super) fn theme_choices() -> Vec<String> {
    let names = rustfetch_render::theme_names();
    let mut choices = Vec::with_capacity(names.len() + 1);
    choices.push("none".to_string());
    choices.extend(names.iter().map(|name| (*name).to_string()));
    choices
}

fn backup_config(path: &Path) -> Result<Option<PathBuf>, String> {
    match std::fs::metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Failed to inspect {}: {error}", path.display())),
        Ok(metadata) if metadata.len() == 0 => Ok(None),
        Ok(_) => {
            let mut backup_name = path.as_os_str().to_os_string();
            backup_name.push(".bak");
            let backup = PathBuf::from(backup_name);
            std::fs::copy(path, &backup)
                .map_err(|error| format!("Failed to back up {}: {error}", path.display()))?;
            Ok(Some(backup))
        }
    }
}

pub(super) fn save_config(config: &Config, path: &Path) -> Result<Option<PathBuf>, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    }
    let backup = backup_config(path)?;
    let toml = config
        .to_toml_string()
        .map_err(|error| format!("Failed to serialize the configuration: {error}"))?;
    crate::shell_hook::write_atomic(path, &toml)?;
    Ok(backup)
}

pub fn run_setup_tui(config: &Config, config_path: &Path) -> Result<(), String> {
    use crossterm::{
        cursor::{Hide, MoveTo, Show},
        event::{Event, KeyCode, KeyModifiers, read},
        execute, queue,
        terminal::{
            Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
            enable_raw_mode,
        },
    };
    use std::io::IsTerminal;

    let mut stdout = std::io::stdout();
    if !stdout.is_terminal() || !std::io::stdin().is_terminal() {
        return Err(
            "Interactive setup (--setup / --tui) requires an interactive terminal (TTY)"
                .to_string(),
        );
    }

    struct SetupGuard;
    impl Drop for SetupGuard {
        fn drop(&mut self) {
            let _ = disable_raw_mode();
            let _ = execute!(std::io::stdout(), Show, LeaveAlternateScreen);
        }
    }

    let mut config = config.clone();
    let themes = theme_choices();
    let border_styles = [
        "rounded", "single", "double", "thick", "dashed", "block", "ascii", "brackets",
    ];
    let mut logos_list: Vec<&str> = vec!["auto"];
    for &(name, _) in crate::logos::list_available_logos() {
        if !logos_list.contains(&name) {
            logos_list.push(name);
        }
    }

    crate::install_terminal_panic_hook();
    if let Err(error) = execute!(stdout, EnterAlternateScreen, Hide) {
        return Err(format!("Failed to enter the alternate screen: {error}"));
    }
    if let Err(error) = enable_raw_mode() {
        let _ = execute!(stdout, Show, LeaveAlternateScreen);
        return Err(format!("Failed to enable raw mode: {error}"));
    }
    let _guard = SetupGuard;

    let mut selected_row = 0_usize;
    let mut status_msg = String::from(
        "Use Up/Down to select, Left/Right or Enter to toggle, [S] to Save, [Q] to Quit. Configuration comments are NOT preserved on save.",
    );

    loop {
        let _ = queue!(stdout, Clear(ClearType::All), MoveTo(0, 0));

        let theme_value = config.general.theme.as_deref().unwrap_or("none");
        let border_style_value = config.general.border_style.as_deref().unwrap_or("rounded");
        let mode_3d = config.general.logo.three_d.enabled;

        let items = [
            ("1. Color Theme", theme_value.to_string()),
            ("2. Logo / Mascot", config.general.logo.distro.clone()),
            (
                "3. Display Mode",
                if mode_3d {
                    "3D Animated".to_string()
                } else {
                    "Standard 2D".to_string()
                },
            ),
            (
                "4. Outer Border Box",
                if config.general.border {
                    "Enabled"
                } else {
                    "Disabled"
                }
                .to_string(),
            ),
            ("5. Border Style", border_style_value.to_string()),
            (
                "6. Nerd Font Icons",
                if config.general.icons {
                    "Enabled"
                } else {
                    "Disabled"
                }
                .to_string(),
            ),
            (
                "7. Center Output",
                if config.general.center {
                    "Enabled"
                } else {
                    "Disabled"
                }
                .to_string(),
            ),
            (
                "8. GitHub Update Check",
                if config.general.check_updates {
                    "Enabled"
                } else {
                    "Disabled"
                }
                .to_string(),
            ),
        ];
        let item_count = items.len();

        print!(
            "  \x1b[1;36m╭──────────────────────────────────────────────────────────────╮\x1b[0m\r\n"
        );
        print!(
            "  \x1b[1;36m│\x1b[0m  \x1b[1mRustFetch Interactive Configuration Wizard (--setup)\x1b[0m        \x1b[1;36m│\x1b[0m\r\n"
        );
        print!(
            "  \x1b[1;36m│\x1b[0m  \x1b[2mExisting settings are preserved; a .bak backup is kept.\x1b[0m   \x1b[1;36m│\x1b[0m\r\n"
        );
        print!(
            "  \x1b[1;36m│\x1b[0m  \x1b[2mComments/key order are not preserved; back up manually if needed.\x1b[0m \x1b[1;36m│\x1b[0m\r\n"
        );
        print!(
            "  \x1b[1;36m╰──────────────────────────────────────────────────────────────╯\x1b[0m\r\n\r\n"
        );

        for (index, (label, value)) in items.iter().enumerate() {
            if index == selected_row {
                print!("  \x1b[1;32m❯ {label:<30}\x1b[0m \x1b[1;33m◀ {value} ▶\x1b[0m\r\n");
            } else {
                print!("    {label:<30} \x1b[2m{value}\x1b[0m\r\n");
            }
        }

        let preview_logo = logos::get_logo(if config.general.logo.distro == "auto" {
            "cat"
        } else {
            &config.general.logo.distro
        });
        let preview_lines = preview_logo.colored_lines();
        print!(
            "\r\n  \x1b[1mLive Logo Preview ({}):\x1b[0m\r\n",
            config.general.logo.distro
        );
        for line in preview_lines.iter().take(8) {
            print!("    {line}\r\n");
        }

        print!("\r\n  \x1b[36mℹ\x1b[0m {status_msg}\r\n");
        let _ = stdout.flush();

        let event = loop {
            if crate::terminate_requested() {
                return Ok(());
            }
            match crossterm::event::poll(std::time::Duration::from_millis(100)) {
                Ok(true) => match read() {
                    Ok(ev) => break ev,
                    Err(_) => return Ok(()),
                },
                Ok(false) => continue,
                Err(_) => return Ok(()),
            }
        };
        let Event::Key(key) = event else {
            continue;
        };
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => return Ok(()),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(()),
            KeyCode::Up | KeyCode::Char('k') => {
                selected_row = if selected_row == 0 {
                    item_count - 1
                } else {
                    selected_row - 1
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                selected_row = (selected_row + 1) % item_count;
            }
            KeyCode::Right
            | KeyCode::Left
            | KeyCode::Enter
            | KeyCode::Char(' ')
            | KeyCode::Char('l')
            | KeyCode::Char('h') => {
                let forward = !matches!(key.code, KeyCode::Left | KeyCode::Char('h'));
                match selected_row {
                    0 => {
                        let current = config.general.theme.as_deref().unwrap_or("none");
                        let position = themes
                            .iter()
                            .position(|theme| theme == current)
                            .unwrap_or(0);
                        let next = if forward {
                            (position + 1) % themes.len()
                        } else if position == 0 {
                            themes.len() - 1
                        } else {
                            position - 1
                        };
                        config.general.theme = if themes[next] == "none" {
                            None
                        } else {
                            Some(themes[next].clone())
                        };
                    }
                    1 => {
                        let current = config.general.logo.distro.as_str();
                        let position = logos_list
                            .iter()
                            .position(|logo| *logo == current)
                            .unwrap_or(0);
                        let next = if forward {
                            (position + 1) % logos_list.len()
                        } else if position == 0 {
                            logos_list.len() - 1
                        } else {
                            position - 1
                        };
                        config.general.logo.distro = logos_list[next].to_string();
                    }
                    2 => {
                        config.general.logo.three_d.enabled = !config.general.logo.three_d.enabled;
                    }
                    3 => {
                        config.general.border = !config.general.border;
                    }
                    4 => {
                        let current = config.general.border_style.as_deref().unwrap_or("rounded");
                        let position = border_styles
                            .iter()
                            .position(|style| *style == current)
                            .unwrap_or(0);
                        let next = if forward {
                            (position + 1) % border_styles.len()
                        } else if position == 0 {
                            border_styles.len() - 1
                        } else {
                            position - 1
                        };
                        config.general.border_style = Some(border_styles[next].to_string());
                    }
                    5 => {
                        config.general.icons = !config.general.icons;
                    }
                    6 => {
                        config.general.center = !config.general.center;
                    }
                    7 => {
                        config.general.check_updates = !config.general.check_updates;
                    }
                    _ => {}
                }
            }
            KeyCode::Char('s') | KeyCode::Char('S') => match save_config(&config, config_path) {
                Ok(backup) => {
                    status_msg = match backup {
                        Some(backup) => format!(
                            "✓ Saved to {} (backup: {}); note: comments are not preserved.",
                            config_path.display(),
                            backup.display()
                        ),
                        None => format!(
                            "✓ Saved to {}; note: comments are not preserved.",
                            config_path.display()
                        ),
                    };
                }
                Err(error) => {
                    status_msg = format!("✗ Save failed: {error}");
                }
            },
            _ => {}
        }
    }
}
