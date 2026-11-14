# 3D Terminal Graphics Engine

RustFetch features an authentic, real-time 3D ASCII and mesh rasterization engine built directly in pure Rust. Without requiring an external GPU window (X11/Wayland) or external dependencies, it extrudes any 2D ASCII drawing or Linux distro logo into a rotating 3D relief model with dynamic lighting and perspective projection.

---

## 🚀 Quick Start

Run RustFetch in continuous 3D animated mode with the `--3d` flag:

```bash
# Launch 3D engine with auto-detected distro logo
rustfetch --3d

# Adjust rotation speed and extrusion relief depth
rustfetch --3d --speed 1.5 --depth 1.4

# High-resolution 2x2 Quadrant block shading
rustfetch --3d --shading-mode blocks

# Edge-only and solid-silhouette looks
rustfetch --3d --shading-mode wireframe
rustfetch --3d --shading-mode solid

# Ultra-crisp 2x3 Unicode Sextant sub-pixel shading
rustfetch --3d --shading-mode sextants

# Stop after N frames (useful for benchmarks and test scripts)
rustfetch --3d --frames 120
```

---

## ⚙️ CLI Options & Flags

| Flag | Type | Default | Description |
|---|---|---|---|
| `--3d` | Bool | `false` | Enables real-time 3D graphics mode. |
| `--logo <DISTRO>` | String | Host Distro | Distro logo to extrude (`arch`, `ubuntu`, `fedora`, `debian`, `gentoo`, etc.). |
| `--ascii <PATH>` | File Path | - | Extrudes a custom user ASCII text file into 3D. |
| `--shading-mode <MODE>` | Enum | `ascii` | Sub-cell rasterization mode: `ascii`, `blocks`, `sextants`, `braille`, `dot`, `wireframe`, or `solid`. |
| `--shading <CHARS>` | String | mode default | Custom shading character ramp (overrides the mode's built-in ramp). |
| `--speed <FLOAT>` | Float | `1.0` | Angular rotation speed multiplier. |
| `--rotate-x` | Bool | config (`true`) | Rotate around the X axis (passing only `--rotate-x` disables Y rotation). |
| `--rotate-y` | Bool | config (`true`) | Rotate around the Y axis (passing only `--rotate-y` disables X rotation). |
| `--depth <FLOAT>` | Float | `1.0` | Z-axis relief depth extrusion multiplier. |
| `--size <FLOAT>` | Float | `1.0` | Scale multiplier of the rendered 3D model. |
| `--frames <N>` | Integer | Infinite | Stop after N frames instead of continuous rotation. |
| `--fps <FPS>` | Float | `30.0` | Target frame rate. |
| `--width <COLS>` | Integer | Auto (42) | Width of 3D rendering canvas in terminal columns. |
| `--hold` | Bool | `false` | Keep animation running without exiting on keypress. |
| `--exit-on-key` | Bool | `true` | Exit the animation on the first keypress. |
| `--infinite` | Bool | `false` | Loop indefinitely, overriding configured frame limits. |

---

## 🎨 Shading Modes

RustFetch supports seven sub-cell rasterization modes depending on your terminal emulator and font. Unknown values fall back to `ascii` and are reported as a config warning (`rustfetch --doctor`):

### 1. `ascii` (Standard 1x1 Cell)
- **Sub-grid:** 1x1
- **Character Ramp:** `.,-~:;=!*#$@`
- **Compatibility:** 100% universal across all terminals, TTY consoles, serial connections, and SSH sessions.
- **Usage:**
  ```bash
  rustfetch --3d --shading-mode ascii
  ```

### 2. `blocks` (Quadrant 2x2 Sub-Cell)
- **Sub-grid:** 2x2 (4 sub-pixels per terminal cell)
- **Glyph Set:** `▘▝▀▖▌▞▛▗▚▐▜▄▙▟█`
- **Visuals:** Uses quadrant block elements to provide 4x geometric edge resolution, dramatically reducing stair-stepping artifacts on curves.
- **Usage:**
  ```bash
  rustfetch --3d --shading-mode blocks
  ```

### 3. `sextants` (Unicode Legacy Computing 2x3 Sub-Cell)
- **Sub-grid:** 2x3 (6 sub-pixels per terminal cell, `U+1FB00..U+1FB3B`)
- **Visuals:** Treats each terminal character cell as 6 independent pixels. Curved lines and silhouettes appear near-pixel-perfect.
- **Requirements:** A modern terminal (Kitty, WezTerm, Alacritty, Foot) and a complete Unicode font.
- **Usage:**
  ```bash
  rustfetch --3d --shading-mode sextants
  ```

### 4. `braille` (Braille 2x4 Sub-Cell)
- **Sub-grid:** 2x4 Unicode braille dots.
- **Visuals:** Very fine dot-matrix edges; works in almost every modern terminal with a Unicode font.

### 5. `dot` (Dot-Matrix)
- **Visuals:** Stippled dot-matrix relief, useful for a lighter, less blocky look.

### 6. `wireframe` (Edge-Only)
- **Visuals:** Draws only the mesh silhouette/edges instead of filled surfaces — good for a technical HUD look.

### 7. `solid` (Filled Silhouette)
- **Visuals:** Fills the projected silhouette with solid shading rather than a character ramp.

Custom ramps apply to every mode:
```bash
rustfetch --3d --shading-mode blocks --shading " .:-=+*#%@"
```

---

## 🧠 Architectural Overview: 2D ASCII to 3D Mesh

The 3D pipeline executes the following stages on every frame:

1. **Ink Density Analysis (`char_weight_utf8`):**
   Evaluates glyph coverage area (e.g. `' '` = 0.0, `'.'` = 0.1, `'#'` = 0.22, `'M'` = 1.0, `'█'` = 1.0).
2. **Relief Mesh Generation:**
   Constructs a heightmap on the Z-axis. The front relief face is placed at positive Z, and a symmetric back face is placed at negative Z.
3. **Edge Extrusion & Surface Normals:**
   Traces perimeter boundaries adjacent to empty space and generates polygon sidewalls, computing exact surface normals (`nx`, `ny`, `nz`).
4. **Lighting & Specular Shading:**
   Samples vertices against a virtual directional light source using Lambertian diffuse shading plus Blinn-Phong specular highlights.
5. **Authentic Dual-Tone TrueColor Distro Branding:**
   - **Arch:** Outer `#1793d1` (Cyan), Inner `#33a2e6` (Light Cyan)
   - **Ubuntu:** Outer `#e95420` (Orange), Inner `#ffffff` (White)
   - **Fedora:** Outer `#29a8e0` (Blue), Inner `#ffffff` (White)
   - **Debian:** Outer `#d70a53` (Red), Inner `#ffffff` (White)
   - **Gentoo:** Outer `#fe7ae6` (Pink), Inner `#ffffff` (White)

---

## ⌨️ Typing Handover (`exit_on_key`)

When running in an interactive shell, the 3D animation loops smoothly without flickering. The instant you press any key (or `q`, `Ctrl+C`), the animation immediately terminates, restores normal terminal mode via an RAII guard, and leaves the final rendered frame cleanly on screen.

---

## 📝 Configuration in `config.toml`

```toml
[general]
3d = true                  # Launch in 3D animated mode by default

[general.logo.three_d]
enabled = true
shading_mode = "blocks"    # ascii, blocks, sextants, braille, dot, wireframe, solid
shading = " .:-=+*#%@"     # Optional custom ramp
speed = 1.2                # 0.05..=10
depth = 1.2                # 0.1..=5
size = 1.0                 # 0.1..=3.0
width = 42                 # 1..=512
height = 24                # 1..=512
light = "top-left"
frames = 120               # Optional: stop after N frames
exit_on_key = true
```

The target frame rate is set per run with `--fps <FPS>` (default `30`) or through
`[[modules]]`-independent `[general.logo.animation] fps` for ASCII animations; it is
not a `three_d` config key.
