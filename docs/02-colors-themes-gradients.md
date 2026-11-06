# Colors, Themes & Gradients

RustFetch features an authentic, 24-bit TrueColor rendering pipeline that operates independently of terminal palette remappings, alongside 13 built-in ricing themes, multi-stop linear text gradients, and flexible palette previews.

---

## 🎨 Built-in Color Themes

RustFetch includes 13 production themes curated from the most popular Linux desktop ricing aesthetics. The identifiers below are the canonical names printed by `rustfetch --list-themes` (kept in sync with `rustfetch_render::theme_names()`):

| Theme Identifier | Name | Primary | Border |
|---|---|---|---|
| `catppuccin` | Catppuccin Mocha | Mauve `#cba6f7` | Lavender `#b4befe` |
| `catppuccin-latte` | Catppuccin Latte | `#8839ef` | `#7287fd` |
| `catppuccin-macchiato` | Catppuccin Macchiato | `#c6a0f6` | `#b7bdf8` |
| `catppuccin-frappe` | Catppuccin Frappé | `#ca9ee6` | `#babbf1` |
| `dracula` | Dracula | Purple `#bd93f9` | `#6272a4` |
| `tokyo-night` | Tokyo Night | Blue `#7aa2f7` | `#565f89` |
| `gruvbox` | Gruvbox Dark | Orange `#fe8019` | `#665c54` |
| `nord` | Nord | Frost Cyan `#88c0d0` | `#4c566a` |
| `rose-pine` | Rosé Pine | Rose `#ebbcba` | `#6e6a86` |
| `cyberpunk` | Cyberpunk | Neon Yellow `#fcee0a` | Deep Purple `#711c91` |
| `monokai` | Monokai | Pink `#f92672` | `#75715e` |
| `onedark` | One Dark | Blue `#61afef` | `#5c6370` |
| `solarized` | Solarized Dark | Blue `#268bd2` | `#586e75` |

Common aliases are accepted as well (for example `catppuccin-mocha`, `mocha`, `latte`, `macchiato`, `frappe`, `tokyonight`, `gruvbox-dark`, `rosepine`, `one-dark`, and `solarized-dark`), but only the canonical identifiers are listed by `--list-themes`.

### Using a Theme
```bash
# Apply a theme via CLI
rustfetch --theme catppuccin

# Or combine with box borders
rustfetch --theme dracula --border --border-style rounded
```

In `config.toml`:
```toml
[general]
theme = "tokyo-night"
```

When a theme is active, it automatically sets default colors for module keys, header titles, and border outlines unless explicitly overridden.

---

## 🌈 24-bit TrueColor & Text Gradients

RustFetch supports full 24-bit TrueColor RGB escape codes (`\x1b[38;2;R;G;Bm`). Any module color, border color, or header color can be specified as a standard 6-digit or 3-digit HEX code (`#bd93f9`, `#ff79c6`, `#fff`).

### Multi-Stop Linear Gradients
You can apply a continuous linear gradient across the output header and module labels by setting the `gradient` array in `config.toml`:

```toml
[general]
gradient = ["#f38ba8", "#cba6f7", "#89b4fa"]
```

RustFetch performs linear RGB interpolation across each line of text:
```
  t = current_char_index / total_characters
  R(t) = R_start * (1 - t) + R_end * t
```

---

## 🎛️ Color Palette Customization

RustFetch allows configuring how the terminal color preview is displayed:

```toml
[general.colors]
enabled = true             # Toggle color preview
symbol = "●"               # Symbol glyph: "●", "■", "󰮯", "◆", ""
block = false              # If true, outputs colored background blocks ("  ")
position = "bottom"        # "bottom", "top", or "both"
rows = 1                   # 1 (8 standard colors) or 2 (16 colors with bright variants)
# custom = ["#1e1e2e", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#cba6f7"]
```

### Visual Preview Examples:

- **1-Row Dots (`symbol = "●"`, `rows = 1`):**
  ```text
  ● ● ● ● ● ● ● ●
  ```
- **2-Row Blocks (`block = true`, `rows = 2`):**
  ```text
  ████ ████ ████ ████ ████ ████ ████ ████
  ████ ████ ████ ████ ████ ████ ████ ████
  ```
- **Custom Color List:** Providing a `custom` array overrides standard ANSI palette colors with exact TrueColor swatches of your choice.
