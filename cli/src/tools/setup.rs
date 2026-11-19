use crate::logos;
use crate::tui_live::{TuiPalette, ansi_to_ratatui_line, border_type_from_style};
use crossterm::{
    cursor::{MoveTo, Show},
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use rustfetch_core::config::Config;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::time::Duration;

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

struct SetupGuard;

impl Drop for SetupGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
    }
}

fn setup_items(config: &Config) -> [(&'static str, String); 8] {
    let theme_value = config.general.theme.as_deref().unwrap_or("none");
    let border_style_value = config.general.border_style.as_deref().unwrap_or("rounded");
    let on_off = |enabled: bool| if enabled { "Enabled" } else { "Disabled" }.to_string();
    [
        ("Color Theme", theme_value.to_string()),
        ("Logo / Mascot", config.general.logo.distro.clone()),
        (
            "Display Mode",
            if config.general.logo.three_d.enabled {
                "3D Animated".to_string()
            } else {
                "Standard 2D".to_string()
            },
        ),
        ("Outer Border Box", on_off(config.general.border)),
        ("Border Style", border_style_value.to_string()),
        ("Nerd Font Icons", on_off(config.general.icons)),
        ("Center Output", on_off(config.general.center)),
        ("GitHub Update Check", on_off(config.general.check_updates)),
    ]
}

fn render_setup_frame(
    frame: &mut Frame<'_>,
    config: &Config,
    selected_row: usize,
    status_msg: &str,
) {
    let area = frame.area();
    let palette = TuiPalette::resolve(config.general.theme.as_deref(), None);
    let border_type = border_type_from_style(config.general.border_style.as_deref());

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(area);

    // Header
    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(Style::default().fg(palette.border));
    let header_text = Line::from(vec![
        Span::styled(
            " RustFetch Interactive Configuration Wizard (--setup) ",
            Style::default()
                .fg(palette.primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " • Existing config backed up to .bak on save",
            Style::default().fg(palette.muted),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(header_text)
            .block(header_block)
            .alignment(Alignment::Center),
        root[0],
    );

    // Body: Settings (left) + Live Preview (right) on wide screens, stacked on narrow screens
    if root[1].width >= 72 {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
            .split(root[1]);
        render_settings_list(frame, cols[0], config, selected_row, palette, border_type);
        render_preview_pane(frame, cols[1], config, palette, border_type);
    } else {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(root[1]);
        render_settings_list(frame, rows[0], config, selected_row, palette, border_type);
        render_preview_pane(frame, rows[1], config, palette, border_type);
    }

    // Footer: status + keybinds
    let footer_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(root[2]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" ℹ ", Style::default().fg(palette.primary)),
            Span::styled(status_msg, Style::default().fg(palette.fg)),
        ])),
        footer_rows[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                " [↑/↓ j/k]",
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Select  ", Style::default().fg(palette.muted)),
            Span::styled(
                "[←/→ Enter]",
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Change  ", Style::default().fg(palette.muted)),
            Span::styled(
                "[s]",
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Save  ", Style::default().fg(palette.muted)),
            Span::styled(
                "[q]",
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Quit", Style::default().fg(palette.muted)),
        ])),
        footer_rows[1],
    );
}

fn render_settings_list(
    frame: &mut Frame<'_>,
    area: Rect,
    config: &Config,
    selected_row: usize,
    palette: TuiPalette,
    border_type: ratatui::widgets::BorderType,
) {
    let items_data = setup_items(config);
    let list_items: Vec<ListItem<'static>> = items_data
        .iter()
        .enumerate()
        .map(|(idx, (label, value))| {
            let is_sel = idx == selected_row;
            let prefix = if is_sel { " ❯ " } else { "   " };
            let val_text = if is_sel {
                format!("◀ {value} ▶")
            } else {
                value.clone()
            };
            let label_style = if is_sel {
                Style::default()
                    .fg(palette.primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.fg)
            };
            let val_style = if is_sel {
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.muted)
            };
            let line = Line::from(vec![
                Span::styled(prefix, label_style),
                Span::styled(format!("{}. {:<22}", idx + 1, label), label_style),
                Span::styled(val_text, val_style),
            ]);
            let item = ListItem::new(line);
            if is_sel {
                item.style(Style::default().bg(palette.highlight_bg))
            } else {
                item
            }
        })
        .collect();

    let block = Block::default()
        .title(" Settings ")
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(Style::default().fg(palette.border));
    let mut state = ListState::default();
    state.select(Some(selected_row));
    frame.render_stateful_widget(List::new(list_items).block(block), area, &mut state);
}

fn render_preview_pane(
    frame: &mut Frame<'_>,
    area: Rect,
    config: &Config,
    palette: TuiPalette,
    border_type: ratatui::widgets::BorderType,
) {
    let title = format!(" Live Preview ({}) ", config.general.logo.distro);
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(Style::default().fg(if config.general.border {
            palette.border
        } else {
            palette.muted
        }));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width < 4 || inner.height < 2 {
        return;
    }

    let preview_logo = logos::get_logo(if config.general.logo.distro == "auto" {
        "cat"
    } else {
        &config.general.logo.distro
    });
    let max_logo_rows = inner.height.saturating_sub(4).max(4) as usize;
    let mut lines: Vec<Line<'static>> = preview_logo
        .colored_lines()
        .into_iter()
        .take(max_logo_rows)
        .map(|l| ansi_to_ratatui_line(&l))
        .collect();

    lines.push(Line::raw(""));
    let icon_os = if config.general.icons { "󰣇 " } else { "" };
    let mode_tag = if config.general.logo.three_d.enabled {
        "3D Animated"
    } else {
        "2D Standard"
    };
    lines.push(Line::from(vec![
        Span::styled(
            format!("{icon_os}Theme: "),
            Style::default()
                .fg(palette.primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            config
                .general
                .theme
                .as_deref()
                .unwrap_or("none")
                .to_string(),
            Style::default().fg(palette.accent),
        ),
        Span::styled(
            format!("  •  {mode_tag}"),
            Style::default().fg(palette.muted),
        ),
    ]));

    let align = if config.general.center {
        Alignment::Center
    } else {
        Alignment::Left
    };
    frame.render_widget(Paragraph::new(lines).alignment(align), inner);
}

pub fn run_setup_tui(config: &Config, config_path: &Path) -> Result<(), String> {
    if !io::stdout().is_terminal() || !io::stdin().is_terminal() {
        return Err(
            "Interactive setup (--setup / --tui) requires an interactive terminal (TTY)"
                .to_string(),
        );
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
    enable_raw_mode().map_err(|e| format!("Failed to enable raw mode: {e}"))?;
    let mut stdout = io::stdout();
    if let Err(error) = execute!(
        stdout,
        EnterAlternateScreen,
        Clear(ClearType::All),
        MoveTo(0, 0)
    ) {
        let _ = disable_raw_mode();
        return Err(format!("Failed to enter the alternate screen: {error}"));
    }
    let _guard = SetupGuard;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        Terminal::new(backend).map_err(|e| format!("Failed to initialize TUI terminal: {e}"))?;

    let item_count = 8_usize;
    let mut selected_row = 0_usize;
    let mut status_msg = String::from(
        "Comments/key order are not preserved on save; a .bak backup is created automatically.",
    );

    loop {
        terminal
            .draw(|f| render_setup_frame(f, &config, selected_row, &status_msg))
            .map_err(|e| format!("Failed to draw setup TUI: {e}"))?;

        let event = loop {
            if crate::terminate_requested() {
                return Ok(());
            }
            match event::poll(Duration::from_millis(100)) {
                Ok(true) => match event::read() {
                    Ok(ev) => break ev,
                    Err(_) => return Ok(()),
                },
                Ok(false) => continue,
                Err(_) => return Ok(()),
            }
        };

        let Event::Key(key) = event else {
            if matches!(event, Event::Resize(_, _)) {
                let _ = terminal.clear();
            }
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

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
                        let next = cycle_index(position, themes.len(), forward);
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
                        let next = cycle_index(position, logos_list.len(), forward);
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
                        let next = cycle_index(position, border_styles.len(), forward);
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
                            "✓ Saved to {} (backup: {})",
                            config_path.display(),
                            backup.display()
                        ),
                        None => format!("✓ Saved to {}", config_path.display()),
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

fn cycle_index(position: usize, len: usize, forward: bool) -> usize {
    if len == 0 {
        return 0;
    }
    if forward {
        (position + 1) % len
    } else if position == 0 {
        len - 1
    } else {
        position - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    #[test]
    fn setup_tui_renders_cleanly_at_wide_and_narrow_sizes() {
        let config = Config::default();
        for (w, h) in [(110, 28), (60, 22)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).expect("test terminal");
            terminal
                .draw(|f| render_setup_frame(f, &config, 0, "Ready"))
                .expect("render setup frame");
            let text: String = terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .map(|c| c.symbol())
                .collect();
            assert!(text.contains("Settings"), "missing Settings at {w}x{h}");
            assert!(
                text.contains("Live Preview"),
                "missing Live Preview at {w}x{h}"
            );
            assert!(text.contains("Color Theme"), "missing items at {w}x{h}");
            assert!(text.contains("Ready"), "missing status bar at {w}x{h}");
        }
    }
}
