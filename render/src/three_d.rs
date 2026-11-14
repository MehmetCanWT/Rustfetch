/// Shading mode determining how sub-cells are sampled and rasterized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShadingMode {
    /// 1x1 sub-cell using standard ASCII characters.
    #[default]
    Ascii,
    /// 2x2 sub-cell using quadrant block elements.
    Blocks,
    /// 2x3 sub-cell using Unicode legacy computing sextant blocks (U+1FB00..U+1FB3B).
    Sextants,
    /// 2x4 sub-cell using Unicode braille patterns (U+2800..U+28FF).
    Braille,
    /// 1x1 sub-cell using a dot ramp.
    Dot,
    /// 1x1 silhouette outline using box drawing characters.
    Wireframe,
    /// 1x1 solid full blocks.
    Solid,
}

impl ShadingMode {
    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_lowercase().replace(['_', ' '], "-").as_str() {
            "blocks" | "block" | "quadrants" | "quadrant" | "halfblock" | "halfblocks" => {
                Self::Blocks
            }
            "sextants" | "sextant" => Self::Sextants,
            "braille" | "braille-dots" => Self::Braille,
            "dot" | "dots" | "dotted" => Self::Dot,
            "wireframe" | "wire" | "wire-frame" => Self::Wireframe,
            "solid" | "filled" => Self::Solid,
            _ => Self::Ascii,
        }
    }

    pub fn sub_grid(&self) -> (usize, usize) {
        match self {
            Self::Ascii | Self::Dot | Self::Wireframe | Self::Solid => (1, 1),
            Self::Blocks => (2, 2),
            Self::Sextants => (3, 2), // (rows, cols)
            Self::Braille => (4, 2),  // (rows, cols)
        }
    }
}

pub const RAMP_ASCII: &str = ".,-~:;=!*#$@";
pub const RAMP_BLOCKS: &str = "░▒▓█";

/// Glyph ramp used by the dot shading mode.
const DOT_RAMP: [&str; 3] = ["·", "•", "●"];

/// Quadrant block glyphs indexed by 4-bit bitmask (bit 0 = top-left .. bit 3 = bottom-right).
const QUADRANT_GLYPHS: [&str; 16] = [
    " ", "▘", "▝", "▀", "▖", "▌", "▞", "▛", "▗", "▚", "▐", "▜", "▄", "▙", "▟", "█",
];

fn build_sextant_table() -> Vec<String> {
    let mut table = Vec::with_capacity(64);
    for mask in 0..64 {
        if mask == 0 {
            table.push(" ".to_string());
        } else if mask == 63 {
            table.push("█".to_string());
        } else if mask == 21 {
            table.push("▌".to_string());
        } else if mask == 42 {
            table.push("▐".to_string());
        } else {
            let mut cp = 0x1FB00 + mask - 1;
            if mask > 21 {
                cp -= 1;
            }
            if mask > 42 {
                cp -= 1;
            }
            let s = std::char::from_u32(cp as u32)
                .map(|c| c.to_string())
                .unwrap_or_else(|| " ".to_string());
            table.push(s);
        }
    }
    table
}

fn braille_bit(sr: usize, sc: usize) -> u8 {
    match (sc, sr) {
        (0, 0) => 0x01,
        (0, 1) => 0x02,
        (0, 2) => 0x04,
        (0, 3) => 0x40,
        (1, 0) => 0x08,
        (1, 1) => 0x10,
        (1, 2) => 0x20,
        (1, 3) => 0x80,
        _ => 0,
    }
}

fn braille_char(mask: u8) -> char {
    std::char::from_u32(0x2800 + mask as u32).unwrap_or(' ')
}

fn char_str(ch: char, buf: &mut [u8; 4]) -> &str {
    ch.encode_utf8(buf)
}

fn wire_glyph(up: bool, down: bool, left: bool, right: bool) -> Option<char> {
    match (up, down, left, right) {
        (true, true, true, true) => Some('┼'),
        (true, true, false, false) => Some('│'),
        (false, false, true, true) => Some('─'),
        (true, true, false, true) => Some('├'),
        (true, true, true, false) => Some('┤'),
        (false, true, true, true) => Some('┬'),
        (true, false, true, true) => Some('┴'),
        (false, true, false, true) => Some('┌'),
        (false, true, true, false) => Some('┐'),
        (true, false, false, true) => Some('└'),
        (true, false, true, false) => Some('┘'),
        (false, true, false, false) => Some('│'),
        (true, false, false, false) => Some('│'),
        (false, false, false, true) => Some('─'),
        (false, false, true, false) => Some('─'),
        (false, false, false, false) => None,
    }
}

fn default_ramp(mode: ShadingMode) -> Vec<String> {
    let ramp = match mode {
        ShadingMode::Blocks | ShadingMode::Sextants => RAMP_BLOCKS,
        _ => RAMP_ASCII,
    };
    ramp.chars().map(|c| c.to_string()).collect()
}

pub fn sanitize_finite(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

pub fn sanitize_size_scale(value: f32) -> f32 {
    sanitize_finite(value, 1.0).clamp(0.1, 3.0)
}

pub fn sanitize_depth_scale(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value.clamp(0.01, 100.0)
    } else {
        1.0
    }
}

pub fn sanitize_speed(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value.clamp(0.05, 5.0)
    } else {
        1.0
    }
}

/// Computes ink density / height for a UTF-8 character.
pub fn char_weight_utf8(ch: char) -> f32 {
    let u = ch as u32;

    // Single-byte ASCII
    if u < 0x80 {
        return match ch {
            'M' => 1.00,
            'N' => 0.88,
            'm' | 'W' => 0.76,
            'd' | 'B' | 'Q' | 'R' => 0.66,
            'h' | 'b' | 'p' | 'q' | 'k' => 0.56,
            'y' | 'g' | 'w' => 0.46,
            'o' | 'n' | 'u' | 'v' | 'x' | 'z' => 0.38,
            's' | 'a' | 'e' | 'c' => 0.30,
            '+' | '*' | '#' => 0.22,
            ':' | ';' => 0.18,
            '=' | '~' => 0.22,
            '-' | '_' => 0.14,
            '`' | '^' => 0.08,
            '.' | ',' => 0.10,
            '/' | '\\' | '|' => 0.16,
            '\'' | '"' => 0.06,
            ' ' => 0.0,
            'A'..='Z' => 0.70,
            'a'..='z' => 0.45,
            '0'..='9' => 0.45,
            _ => 0.20,
        };
    }

    match u {
        0x2588 => 1.00,
        0x2593 => 0.75,
        0x2592 => 0.50,
        0x2591 => 0.25,
        0x2580 | 0x2584 | 0x258C | 0x2590 => 0.50,
        0x259B | 0x259C | 0x2599 | 0x259F => 0.75,
        0x2596 | 0x2597 | 0x2598 | 0x259D => 0.25,
        0x2500..=0x257F => 0.22,
        0x2800..=0x28FF => {
            let dots = (u & 0xFF).count_ones() as f32;
            (dots / 8.0).max(0.12)
        }
        _ => 0.35,
    }
}

/// A 3D vertex with normal and optional ANSI color string.
#[derive(Debug, Clone)]
pub struct Point3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub nx: f32,
    pub ny: f32,
    pub nz: f32,
    pub color: Option<String>,
    pub is_outer_side: bool,
}

/// A parsed 3D logo model ready for rotation and rasterization.
#[derive(Debug, Clone)]
pub struct Model3D {
    pub points: Vec<Point3D>,
    pub orig_rows: usize,
    pub orig_cols: usize,
}

impl Model3D {
    /// Constructs a 3D relief model from lines of ASCII art with optional ANSI escape codes.
    pub fn from_ascii_lines(
        lines: &[String],
        size_scale: f32,
        depth_scale: f32,
        shading_mode: ShadingMode,
    ) -> Self {
        let size_scale = sanitize_size_scale(size_scale);
        let depth_scale = sanitize_depth_scale(depth_scale);
        let (sub_rows, _sub_cols) = shading_mode.sub_grid();

        let mut raw_cells: Vec<Vec<char>> = Vec::new();
        let mut raw_colors: Vec<Vec<Option<String>>> = Vec::new();
        let mut max_cols = 0;

        for line in lines {
            let mut row_chars = Vec::new();
            let mut row_colors = Vec::new();
            let mut cur_color: Option<String> = None;
            let mut chars = line.chars().peekable();

            while let Some(c) = chars.next() {
                if c == '\x1b' && chars.peek() == Some(&'[') {
                    let mut seq = String::from("\x1b[");
                    chars.next();
                    while let Some(&next) = chars.peek() {
                        seq.push(next);
                        chars.next();
                        if next.is_ascii_alphabetic() {
                            break;
                        }
                    }
                    if seq.ends_with('m') {
                        if seq == "\x1b[0m" || seq == "\x1b[m" || seq == "\x1b[39m" {
                            cur_color = None;
                        } else {
                            let tc_seq = match seq.as_str() {
                                "\x1b[34m" | "\x1b[1;34m" => {
                                    Some("\x1b[38;2;41;168;224m".to_string())
                                }
                                "\x1b[37m" | "\x1b[1;37m" => {
                                    Some("\x1b[38;2;255;255;255m".to_string())
                                }
                                "\x1b[35m" | "\x1b[1;35m" => {
                                    Some("\x1b[38;2;254;122;230m".to_string())
                                }
                                "\x1b[36m" | "\x1b[1;36m" => {
                                    Some("\x1b[38;2;23;147;209m".to_string())
                                }
                                "\x1b[31m" | "\x1b[1;31m" => {
                                    Some("\x1b[38;2;239;68;68m".to_string())
                                }
                                "\x1b[32m" | "\x1b[1;32m" => {
                                    Some("\x1b[38;2;34;197;94m".to_string())
                                }
                                "\x1b[33m" | "\x1b[1;33m" => {
                                    Some("\x1b[38;2;234;179;8m".to_string())
                                }
                                "\x1b[96m" | "\x1b[1;96m" => {
                                    Some("\x1b[38;2;51;162;230m".to_string())
                                }
                                "\x1b[94m" | "\x1b[1;94m" => {
                                    Some("\x1b[38;2;60;130;246m".to_string())
                                }
                                "\x1b[95m" | "\x1b[1;95m" => {
                                    Some("\x1b[38;2;254;122;230m".to_string())
                                }
                                "\x1b[97m" | "\x1b[1;97m" => {
                                    Some("\x1b[38;2;255;255;255m".to_string())
                                }
                                _ if seq.starts_with("\x1b[38;2;")
                                    || seq.starts_with("\x1b[38;5;") =>
                                {
                                    Some(seq)
                                }
                                _ => None,
                            };
                            if let Some(color_seq) = tc_seq {
                                cur_color = Some(color_seq);
                            }
                        }
                    }
                    continue;
                }
                row_chars.push(c);
                row_colors.push(cur_color.clone());
            }

            max_cols = max_cols.max(row_chars.len());
            raw_cells.push(row_chars);
            raw_colors.push(row_colors);
        }

        let num_rows = raw_cells.len();
        let num_cols = max_cols;

        if num_rows == 0 || num_cols == 0 {
            return Self {
                points: Vec::new(),
                orig_rows: 0,
                orig_cols: 0,
            };
        }

        let mut hmap = vec![vec![0.0f32; num_cols]; num_rows];
        let mut sum = 0.0f32;
        let mut sum2 = 0.0f32;
        let mut count = 0;

        for r in 0..num_rows {
            for c in 0..num_cols {
                if c < raw_cells[r].len() {
                    let w = char_weight_utf8(raw_cells[r][c]);
                    hmap[r][c] = w;
                    if w > 0.0 {
                        sum += w;
                        sum2 += w * w;
                        count += 1;
                    }
                }
            }
        }

        let mut effective_depth = depth_scale;
        if count > 0 {
            let mean = sum / count as f32;
            let variance = (sum2 / count as f32) - (mean * mean);
            let stddev = variance.max(0.0).sqrt();
            if stddev < 0.25 {
                let boost = 1.0 + 2.0 * ((0.25 - stddev) / 0.25);
                effective_depth *= boost;
            }
        }

        let norm = if num_rows > 0 && num_rows < 30 {
            (28.0f32 / (num_rows as f32)).clamp(1.0, 1.55)
        } else {
            1.0f32
        };
        let sx = 0.07f32 * norm;
        let sy = 0.14f32 * norm;
        let cx = (num_cols as f32 - 1.0) * 0.5;
        let cy = (num_rows as f32 - 1.0) * 0.5;
        let zmax = 0.18f32 * effective_depth * norm;

        let mut gnx = vec![vec![0.0f32; num_cols]; num_rows];
        let mut gny = vec![vec![0.0f32; num_cols]; num_rows];
        let mut gnz = vec![vec![1.0f32; num_cols]; num_rows];

        for r in 0..num_rows {
            for c in 0..num_cols {
                if hmap[r][c] <= 0.0 {
                    continue;
                }
                let dhdx = if c > 0 && c < num_cols - 1 {
                    (hmap[r][c + 1] - hmap[r][c - 1]) * 0.5
                } else if c == 0 && num_cols > 1 {
                    hmap[r][c + 1] - hmap[r][c]
                } else if c > 0 {
                    hmap[r][c] - hmap[r][c - 1]
                } else {
                    0.0
                } / sx;

                let dhdy = if r > 0 && r < num_rows - 1 {
                    (hmap[r + 1][c] - hmap[r - 1][c]) * 0.5
                } else if r == 0 && num_rows > 1 {
                    hmap[r + 1][c] - hmap[r][c]
                } else if r > 0 {
                    hmap[r][c] - hmap[r - 1][c]
                } else {
                    0.0
                } / sy;

                let nnx = -dhdx;
                let nny = dhdy;
                let nnz = 1.0f32;
                let len = (nnx * nnx + nny * nny + nnz * nnz).sqrt();
                gnx[r][c] = nnx / len;
                gny[r][c] = nny / len;
                gnz[r][c] = nnz / len;
            }
        }

        // Hard upper bounds keep a bad size_scale from exploding memory/CPU.
        let z_layers = ((6.0 * size_scale) as usize).clamp(6, 48);
        let subdiv = ((size_scale * sub_rows as f32) as usize).clamp(sub_rows, sub_rows * 4);

        let mut points = Vec::new();

        for row in 0..num_rows {
            for col in 0..num_cols {
                let h = hmap[row][col];
                if h <= 0.0 {
                    continue;
                }

                let cell_color = raw_colors[row].get(col).cloned().flatten();

                for sr in 0..subdiv {
                    for sc in 0..subdiv {
                        let frow = row as f32 + (sr as f32 / subdiv as f32);
                        let fcol = col as f32 + (sc as f32 / subdiv as f32);

                        let ih = if sr > 0 || sc > 0 {
                            let fr = sr as f32 / subdiv as f32;
                            let fc = sc as f32 / subdiv as f32;
                            let nr = (row + if sr > 0 { 1 } else { 0 }).min(num_rows - 1);
                            let nc = (col + if sc > 0 { 1 } else { 0 }).min(num_cols - 1);
                            let h00 = hmap[row][col];
                            let h10 = hmap[nr][col];
                            let h01 = hmap[row][nc];
                            let h11 = hmap[nr][nc];
                            h00 * (1.0 - fr) * (1.0 - fc)
                                + h10 * fr * (1.0 - fc)
                                + h01 * (1.0 - fr) * fc
                                + h11 * fr * fc
                        } else {
                            h
                        };

                        if ih <= 0.0 {
                            continue;
                        }

                        let ox = (fcol - cx) * sx;
                        let oy = (cy - frow) * sy;
                        let zr = ih * zmax;

                        let mut is_edge = false;
                        for dr in -1i32..=1 {
                            for dc in -1i32..=1 {
                                if dr == 0 && dc == 0 {
                                    continue;
                                }
                                let nr = row as i32 + dr;
                                let nc = col as i32 + dc;
                                if nr < 0
                                    || nr >= num_rows as i32
                                    || nc < 0
                                    || nc >= num_cols as i32
                                    || hmap[nr as usize][nc as usize] <= 0.0
                                {
                                    is_edge = true;
                                    break;
                                }
                            }
                            if is_edge {
                                break;
                            }
                        }

                        let layers = if is_edge || ih < 0.15 { 2 } else { z_layers };

                        for k in 0..layers {
                            let t = ((k as f32) / ((layers - 1) as f32)) - 0.5;
                            let pz = t * 2.0 * zr;

                            let (nx, ny, nz) = if k == 0 {
                                (gnx[row][col], gny[row][col], -gnz[row][col])
                            } else if k == layers - 1 {
                                (-gnx[row][col], gny[row][col], gnz[row][col])
                            } else {
                                let mut ex = 0.0f32;
                                let mut ey = 0.0f32;
                                for dr in -1i32..=1 {
                                    for dc in -1i32..=1 {
                                        if dr == 0 && dc == 0 {
                                            continue;
                                        }
                                        let nr = row as i32 + dr;
                                        let nc = col as i32 + dc;
                                        let nh = if nr >= 0
                                            && nr < num_rows as i32
                                            && nc >= 0
                                            && nc < num_cols as i32
                                        {
                                            hmap[nr as usize][nc as usize]
                                        } else {
                                            0.0
                                        };
                                        if nh < h {
                                            ex += dc as f32;
                                            ey += -dr as f32;
                                        }
                                    }
                                }
                                let el = (ex * ex + ey * ey).sqrt();
                                if el > 1e-6 {
                                    ex /= el;
                                    ey /= el;
                                }
                                let tn = (k as f32 / (layers - 1) as f32) * 2.0 - 1.0;
                                let side = (1.0 - tn * tn).max(0.0).sqrt();
                                (ex * side, ey * side, tn)
                            };

                            let is_outer_side = k > 0 && k < layers - 1;

                            points.push(Point3D {
                                x: ox,
                                y: oy,
                                z: pz,
                                nx,
                                ny,
                                nz,
                                color: cell_color.clone(),
                                is_outer_side,
                            });
                        }
                    }
                }
            }
        }

        Self {
            points,
            orig_rows: num_rows,
            orig_cols: num_cols,
        }
    }
}

/// Renderer responsible for rotating, lighting, and rasterizing 3D models.
pub struct Renderer3D {
    pub shading_mode: ShadingMode,
    pub shading_chars: Vec<String>,
    pub sextant_table: Vec<String>,
    pub light_dir: (f32, f32, f32),
    pub half_dir: (f32, f32, f32),
    pub size_scale: f32,
    pub speed: f32,
    pub outer_color: String,
    pub inner_color: String,
}

impl Default for Renderer3D {
    fn default() -> Self {
        let (lx, ly, lz) = (0.4082f32, 0.8165f32, -0.4082f32);
        let vx = 0.0f32;
        let vy = 0.0f32;
        let vz = -1.0f32;
        let hx = lx + vx;
        let hy = ly + vy;
        let hz = lz + vz;
        let hlen = (hx * hx + hy * hy + hz * hz).sqrt();

        Self {
            shading_mode: ShadingMode::Ascii,
            shading_chars: default_ramp(ShadingMode::Ascii),
            sextant_table: build_sextant_table(),
            light_dir: (lx, ly, lz),
            half_dir: (hx / hlen, hy / hlen, hz / hlen),
            size_scale: 1.0,
            speed: 1.0,
            outer_color: "\x1b[1;36m".to_string(),
            inner_color: "\x1b[1;37m".to_string(),
        }
    }
}

fn normalize_distro_key(distro: &str) -> String {
    distro
        .trim()
        .to_lowercase()
        .replace(['_', '-'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Returns the (outer_color, inner_color) ANSI escape codes for a distribution.
///
/// Matching is exact (after normalisation) so unrelated names that merely
/// contain a distro token (for example Vanilla OS vs van) do not get the wrong
/// branding. Uses 24-bit TrueColor so terminal themes cannot override the
/// official branding colours.
pub fn distro_3d_colors(distro: &str) -> (&'static str, &'static str) {
    let key = normalize_distro_key(distro);
    match key.as_str() {
        "gentoo" | "gentoo linux" => ("\x1b[38;2;254;122;230m", "\x1b[38;2;255;255;255m"),
        "arch" | "arch linux" | "archlinux" | "arch linux arm" => {
            ("\x1b[38;2;23;147;209m", "\x1b[38;2;51;162;230m")
        }
        "ubuntu" | "ubuntu linux" | "asahi" | "asahi linux" => {
            ("\x1b[38;2;233;84;32m", "\x1b[38;2;255;255;255m")
        }
        "debian" | "debian gnu linux" | "debian gnu/linux" => {
            ("\x1b[38;2;215;10;83m", "\x1b[38;2;255;255;255m")
        }
        "bazzite" | "bazzite linux" => ("\x1b[38;2;138;79;255m", "\x1b[38;2;41;168;224m"),
        "fedora" | "fedora linux" | "nobara" | "nobara linux" => {
            ("\x1b[38;2;41;168;224m", "\x1b[38;2;255;255;255m")
        }
        "nixos" | "nixos linux" => ("\x1b[38;2;82;119;195m", "\x1b[38;2;126;186;228m"),
        "void" | "void linux" => ("\x1b[38;2;71;128;99m", "\x1b[38;2;104;166;132m"),
        "alpine" | "alpine linux" => ("\x1b[38;2;41;168;224m", "\x1b[38;2;255;255;255m"),
        "opensuse"
        | "opensuse tumbleweed"
        | "opensuse leap"
        | "opensuse microos"
        | "suse"
        | "sles" => ("\x1b[38;2;115;186;37m", "\x1b[38;2;255;255;255m"),
        "mint" | "linux mint" => ("\x1b[38;2;135;189;101m", "\x1b[38;2;255;255;255m"),
        "manjaro" | "manjaro linux" => ("\x1b[38;2;53;191;92m", "\x1b[38;2;255;255;255m"),
        "macos" | "mac os" | "darwin" | "pop" | "pop os" | "pop! os" | "pop!_os" => {
            ("\x1b[38;2;51;162;230m", "\x1b[38;2;255;255;255m")
        }
        "cachyos" | "cachy os" | "cachy" => ("\x1b[38;2;0;180;216m", "\x1b[38;2;0;204;153m"),
        "freebsd" | "free bsd" | "openbsd" | "open bsd" | "netbsd" | "net bsd"
        | "dragonfly bsd" | "bsd" => ("\x1b[38;2;218;31;40m", "\x1b[38;2;255;255;255m"),
        "bengal" | "bengal cat" | "cat" | "kedi" => {
            ("\x1b[38;2;249;175;79m", "\x1b[38;2;102;217;160m")
        }
        "van" | "turkish van" | "angora" | "turkish angora" => {
            ("\x1b[38;2;81;190;240m", "\x1b[38;2;255;255;255m")
        }
        _ => ("\x1b[38;2;254;122;230m", "\x1b[38;2;255;255;255m"),
    }
}

/// Convert an ANSI escape sequence or color name/hex into a TrueColor or formatted escape.
pub fn parse_color_to_ansi(color_str: &str) -> String {
    let s = color_str.trim();
    if s.starts_with("\x1b[") {
        return s.to_string();
    }
    if let Some((r, g, b)) = crate::color::parse_color(s) {
        format!("\x1b[38;2;{r};{g};{b}m")
    } else {
        "\x1b[38;2;255;255;255m".to_string()
    }
}

#[derive(Clone, Copy)]
struct CellSample<'a> {
    mask: usize,
    braille: u8,
    n: usize,
    lsum: f32,
    color: Option<&'a str>,
    outer: bool,
}

impl Renderer3D {
    pub fn new(shading_mode: ShadingMode, custom_shading: Option<&str>) -> Self {
        let mut shading_chars: Vec<String> = custom_shading
            .map(|chars| chars.chars().map(|c| c.to_string()).collect())
            .unwrap_or_default();
        if shading_chars.is_empty() {
            shading_chars = default_ramp(shading_mode);
        }
        Self {
            shading_mode,
            shading_chars,
            ..Self::default()
        }
    }

    pub fn set_light_direction(&mut self, light: &str) {
        let (lx, ly, lz): (f32, f32, f32) = match light.trim().to_ascii_lowercase().as_str() {
            "top-right" | "topright" => (-0.4082, 0.8165, -0.4082),
            "front" | "center" => (0.0, 0.0, -1.0),
            "top" => (0.0, 0.8944, -0.4472),
            "bottom" => (0.0, -0.8944, -0.4472),
            _ => (0.4082, 0.8165, -0.4082),
        };
        let vx = 0.0f32;
        let vy = 0.0f32;
        let vz = -1.0f32;
        let hx = lx + vx;
        let hy = ly + vy;
        let hz = lz + vz;
        let hlen = (hx * hx + hy * hy + hz * hz).sqrt().max(1e-6);
        self.light_dir = (lx, ly, lz);
        self.half_dir = (hx / hlen, hy / hlen, hz / hlen);
    }

    /// Renders a single frame of the 3D model into an array of terminal strings.
    pub fn render_frame(
        &self,
        model: &Model3D,
        angle_x: f32,
        angle_y: f32,
        canvas_cols: usize,
        canvas_rows: usize,
    ) -> Vec<String> {
        if model.points.is_empty() || canvas_cols == 0 || canvas_rows == 0 {
            return vec![String::new(); canvas_rows];
        }

        let (sub_rows, sub_cols) = self.shading_mode.sub_grid();
        let total_sub_cols = canvas_cols * sub_cols;
        let total_sub_rows = canvas_rows * sub_rows;
        let total_cells = total_sub_rows * total_sub_cols;

        let mut zbuf = vec![0.0f32; total_cells];
        let mut lumbuf = vec![0.0f32; total_cells];
        let mut colorbuf: Vec<Option<&str>> = vec![None; total_cells];
        let mut is_outer_buf = vec![false; total_cells];

        let angle_x = sanitize_finite(angle_x, 0.0);
        let angle_y = sanitize_finite(angle_y, 0.0);
        let cos_a = angle_x.cos();
        let sin_a = angle_x.sin();
        let cos_b = angle_y.cos();
        let sin_b = angle_y.sin();

        let (lx, ly, lz) = self.light_dir;
        let (hx, hy, hz) = self.half_dir;

        let size_scale = sanitize_size_scale(self.size_scale);
        let k2 = 5.5f32;
        let logo_h = canvas_cols * 3 / 5;
        let k1 = 37.0f32 * (logo_h as f32 / 36.0f32) * size_scale;
        let k1x2 = k1 * 2.0;

        let half_w = canvas_cols as f32 * 0.5;

        let mut face_up = 0.0f32;
        for pt in &model.points {
            let zc = pt.z + k2;
            if zc > 0.1 {
                let ys = k1 * pt.y / zc;
                if ys > face_up {
                    face_up = ys;
                }
            }
        }
        let fixed_y_center = face_up + 1.0;
        let y_center = if fixed_y_center > 0.0 && fixed_y_center < canvas_rows as f32 {
            fixed_y_center
        } else {
            canvas_rows as f32 * 0.5
        };

        for pt in &model.points {
            let y1 = pt.y * cos_a - pt.z * sin_a;
            let z1 = pt.y * sin_a + pt.z * cos_a;

            let x2 = pt.x * cos_b + z1 * sin_b;
            let z2 = -pt.x * sin_b + z1 * cos_b;
            let y2 = y1;

            let ny1 = pt.ny * cos_a - pt.nz * sin_a;
            let nz1 = pt.ny * sin_a + pt.nz * cos_a;
            let nx2 = pt.nx * cos_b + nz1 * sin_b;
            let nz2 = -pt.nx * sin_b + nz1 * cos_b;
            let ny2 = ny1;

            let zc = z2 + k2;
            if zc < 0.1 {
                continue;
            }

            let ooz = 1.0 / zc;
            let xs = ((half_w + k1x2 * x2 * ooz) * sub_cols as f32).floor() as i32;
            let ys = ((y_center - k1 * y2 * ooz) * sub_rows as f32).floor() as i32;

            if xs < 0 || xs >= total_sub_cols as i32 || ys < 0 || ys >= total_sub_rows as i32 {
                continue;
            }

            let ux = xs as usize;
            let uy = ys as usize;
            let idx = uy * total_sub_cols + ux;

            if ooz > zbuf[idx] {
                let diff = (nx2 * lx + ny2 * ly + nz2 * lz).max(0.0);

                let spec_dot = (nx2 * hx + ny2 * hy + nz2 * hz).max(0.0);
                let mut spec = spec_dot * spec_dot;
                spec *= spec;
                spec *= spec;

                let l = (0.08 + 0.62 * diff + 0.30 * spec).min(1.0);

                zbuf[idx] = ooz;
                lumbuf[idx] = l;
                colorbuf[idx] = pt.color.as_deref();
                is_outer_buf[idx] = pt.is_outer_side;
            }
        }

        let mut cells: Vec<CellSample<'_>> = Vec::with_capacity(canvas_cols * canvas_rows);
        for row in 0..canvas_rows {
            let y0 = row * sub_rows;
            for col in 0..canvas_cols {
                let x0 = col * sub_cols;
                let mut mask = 0usize;
                let mut braille = 0u8;
                let mut bit = 0;
                let mut n = 0;
                let mut lsum = 0.0f32;
                let mut best_z = 0.0f32;
                let mut cell_color: Option<&str> = None;
                let mut cell_is_outer = false;

                for sr in 0..sub_rows {
                    let row_offset = (y0 + sr) * total_sub_cols;
                    for sc in 0..sub_cols {
                        let idx = row_offset + (x0 + sc);
                        let z = zbuf[idx];
                        if z > 0.0 {
                            mask |= 1 << bit;
                            if self.shading_mode == ShadingMode::Braille {
                                braille |= braille_bit(sr, sc);
                            }
                            lsum += lumbuf[idx];
                            n += 1;
                            if z > best_z {
                                best_z = z;
                                cell_color = colorbuf[idx];
                                cell_is_outer = is_outer_buf[idx];
                            }
                        }
                        bit += 1;
                    }
                }

                cells.push(CellSample {
                    mask,
                    braille,
                    n,
                    lsum,
                    color: cell_color,
                    outer: cell_is_outer,
                });
            }
        }

        let mut occupied = vec![false; cells.len()];
        for (index, cell) in cells.iter().enumerate() {
            occupied[index] = cell.n > 0;
        }

        let fallback_chars: Vec<String>;
        let shading_chars: &[String] = if self.shading_chars.is_empty() {
            fallback_chars = default_ramp(self.shading_mode);
            &fallback_chars
        } else {
            &self.shading_chars
        };
        let smax = shading_chars.len().saturating_sub(1);
        let total_sub = sub_rows * sub_cols;
        let full_mask = (1usize << total_sub) - 1;

        let mut output_lines = Vec::with_capacity(canvas_rows);

        for row in 0..canvas_rows {
            let mut line = String::with_capacity(canvas_cols * 16);
            let mut prev_color: Option<&str> = None;

            for col in 0..canvas_cols {
                let cell = &cells[row * canvas_cols + col];

                if cell.n == 0 {
                    if prev_color.is_some() {
                        line.push_str("\x1b[0m");
                        prev_color = None;
                    }
                    line.push(' ');
                    continue;
                }

                let coverage = cell.n as f32 / total_sub as f32;
                let ink = (cell.lsum / cell.n as f32) * coverage;
                let ci = ((ink * smax as f32) + 0.5) as usize;
                let ci = ci.min(smax);

                let mut buf = [0u8; 4];
                let glyph: &str = match self.shading_mode {
                    ShadingMode::Ascii => &shading_chars[ci],
                    ShadingMode::Dot => DOT_RAMP[ci.min(DOT_RAMP.len() - 1)],
                    ShadingMode::Solid => "█",
                    ShadingMode::Blocks => {
                        if cell.mask == full_mask {
                            &shading_chars[ci]
                        } else {
                            QUADRANT_GLYPHS[cell.mask.min(QUADRANT_GLYPHS.len() - 1)]
                        }
                    }
                    ShadingMode::Sextants => {
                        if cell.mask == full_mask || cell.mask >= self.sextant_table.len() {
                            &shading_chars[ci]
                        } else {
                            &self.sextant_table[cell.mask]
                        }
                    }
                    ShadingMode::Braille => char_str(braille_char(cell.braille), &mut buf),
                    ShadingMode::Wireframe => {
                        let up = row > 0 && occupied[(row - 1) * canvas_cols + col];
                        let down = row + 1 < canvas_rows && occupied[(row + 1) * canvas_cols + col];
                        let left = col > 0 && occupied[row * canvas_cols + col - 1];
                        let right = col + 1 < canvas_cols && occupied[row * canvas_cols + col + 1];
                        match wire_glyph(up, down, left, right) {
                            Some(ch) => char_str(ch, &mut buf),
                            None => " ",
                        }
                    }
                };

                let active_color = if cell.outer {
                    self.outer_color.as_str()
                } else if let Some(col) = cell.color {
                    col
                } else {
                    self.inner_color.as_str()
                };

                if prev_color != Some(active_color) {
                    line.push_str(active_color);
                    prev_color = Some(active_color);
                }

                line.push_str(glyph);
            }

            if prev_color.is_some() {
                line.push_str("\x1b[0m");
            }
            output_lines.push(line);
        }

        output_lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_model(mode: ShadingMode) -> Model3D {
        let ascii = vec![
            "  ###  ".to_string(),
            " ####### ".to_string(),
            "  #####  ".to_string(),
        ];
        Model3D::from_ascii_lines(&ascii, 1.0, 1.0, mode)
    }

    #[test]
    fn test_char_weight_utf8() {
        assert_eq!(char_weight_utf8(' '), 0.0);
        assert!(char_weight_utf8('M') > char_weight_utf8('.'));
        assert_eq!(char_weight_utf8('█'), 1.0);
        assert_eq!(char_weight_utf8('░'), 0.25);
    }

    #[test]
    fn test_model3d_creation() {
        let model = small_model(ShadingMode::Ascii);
        assert!(!model.points.is_empty());
        assert_eq!(model.orig_rows, 3);
    }

    #[test]
    fn test_renderer_frame() {
        let ascii = vec![
            "   /\\   ".to_string(),
            "  /  \\  ".to_string(),
            " /____\\ ".to_string(),
        ];
        let model = Model3D::from_ascii_lines(&ascii, 1.0, 1.0, ShadingMode::Ascii);
        let renderer = Renderer3D::default();
        let frame = renderer.render_frame(&model, 0.2, 0.4, 20, 10);
        assert_eq!(frame.len(), 10);
        assert!(frame.iter().any(|line| !line.trim().is_empty()));
    }

    #[test]
    fn shading_mode_parses_all_locked_names() {
        assert_eq!(ShadingMode::from_str_loose("ascii"), ShadingMode::Ascii);
        assert_eq!(ShadingMode::from_str_loose("blocks"), ShadingMode::Blocks);
        assert_eq!(
            ShadingMode::from_str_loose("sextants"),
            ShadingMode::Sextants
        );
        assert_eq!(ShadingMode::from_str_loose("braille"), ShadingMode::Braille);
        assert_eq!(ShadingMode::from_str_loose("dot"), ShadingMode::Dot);
        assert_eq!(
            ShadingMode::from_str_loose("wireframe"),
            ShadingMode::Wireframe
        );
        assert_eq!(ShadingMode::from_str_loose("wire"), ShadingMode::Wireframe);
        assert_eq!(ShadingMode::from_str_loose("solid"), ShadingMode::Solid);
        assert_eq!(ShadingMode::from_str_loose("nonsense"), ShadingMode::Ascii);
    }

    #[test]
    fn sub_grids_match_documented_shapes() {
        assert_eq!(ShadingMode::Ascii.sub_grid(), (1, 1));
        assert_eq!(ShadingMode::Blocks.sub_grid(), (2, 2));
        assert_eq!(ShadingMode::Sextants.sub_grid(), (3, 2));
        assert_eq!(ShadingMode::Braille.sub_grid(), (4, 2));
    }

    #[test]
    fn braille_bits_cover_both_columns() {
        assert_eq!(braille_bit(0, 0), 0x01);
        assert_eq!(braille_bit(1, 0), 0x02);
        assert_eq!(braille_bit(2, 0), 0x04);
        assert_eq!(braille_bit(3, 0), 0x40);
        assert_eq!(braille_bit(0, 1), 0x08);
        assert_eq!(braille_bit(1, 1), 0x10);
        assert_eq!(braille_bit(2, 1), 0x20);
        assert_eq!(braille_bit(3, 1), 0x80);
        assert_eq!(braille_char(0) as u32, 0x2800);
        assert_eq!(braille_char(0xFF) as u32, 0x28FF);
    }

    #[test]
    fn wire_glyph_draws_outlines() {
        assert_eq!(wire_glyph(true, true, true, true), Some('┼'));
        assert_eq!(wire_glyph(true, true, false, false), Some('│'));
        assert_eq!(wire_glyph(false, false, true, true), Some('─'));
        assert_eq!(wire_glyph(false, false, false, false), None);
    }

    #[test]
    fn empty_custom_shading_chars_never_panic() {
        let model = small_model(ShadingMode::Ascii);
        let renderer = Renderer3D::new(ShadingMode::Ascii, Some(""));
        assert!(!renderer.shading_chars.is_empty());
        let frame = renderer.render_frame(&model, 0.1, 0.2, 12, 6);
        assert_eq!(frame.len(), 6);
        assert!(frame.iter().any(|line| !line.trim().is_empty()));
    }

    #[test]
    fn every_shading_mode_renders_output() {
        for mode in [
            ShadingMode::Ascii,
            ShadingMode::Blocks,
            ShadingMode::Sextants,
            ShadingMode::Braille,
            ShadingMode::Dot,
            ShadingMode::Wireframe,
            ShadingMode::Solid,
        ] {
            let model = small_model(mode);
            let renderer = Renderer3D::new(mode, None);
            let frame = renderer.render_frame(&model, 0.0, 0.0, 16, 8);
            assert_eq!(frame.len(), 8, "mode {mode:?}");
            assert!(
                frame.iter().any(|line| !line.trim().is_empty()),
                "mode {mode:?} produced no visible output"
            );
        }
    }

    #[test]
    fn braille_mode_emits_braille_codepoints() {
        let model = small_model(ShadingMode::Braille);
        let renderer = Renderer3D::new(ShadingMode::Braille, None);
        let frame = renderer.render_frame(&model, 0.0, 0.0, 24, 10);
        let braille_count = frame
            .iter()
            .flat_map(|line| line.chars())
            .filter(|ch| ('\u{2800}'..='\u{28ff}').contains(ch))
            .count();
        assert!(braille_count > 0, "expected braille glyphs in output");
    }

    #[test]
    fn extreme_size_and_depth_are_sanitized() {
        let ascii = vec![
            "  ###  ".to_string(),
            " #   # ".to_string(),
            "  ###  ".to_string(),
        ];
        for scale in [f32::INFINITY, f32::NEG_INFINITY, f32::NAN, 1e30, 100.0] {
            let model = Model3D::from_ascii_lines(&ascii, scale, scale, ShadingMode::Ascii);
            assert!(
                model.points.len() < 250_000,
                "mesh must stay bounded for scale {scale}"
            );
            assert!(
                model
                    .points
                    .iter()
                    .all(|p| p.x.is_finite() && p.y.is_finite()),
                "coordinates must be finite for scale {scale}"
            );
        }
    }

    #[test]
    fn sanitize_helpers_have_defaults() {
        assert_eq!(sanitize_size_scale(f32::NAN), 1.0);
        assert_eq!(sanitize_size_scale(f32::INFINITY), 1.0);
        assert_eq!(sanitize_size_scale(100.0), 3.0);
        assert_eq!(sanitize_size_scale(0.0), 0.1);
        assert_eq!(sanitize_depth_scale(f32::NAN), 1.0);
        assert_eq!(sanitize_depth_scale(f32::INFINITY), 1.0);
        assert_eq!(sanitize_speed(f32::NAN), 1.0);
        assert_eq!(sanitize_speed(f32::INFINITY), 1.0);
    }

    #[test]
    fn distro_colors_use_exact_matching() {
        let van = ("\x1b[38;2;81;190;240m", "\x1b[38;2;255;255;255m");
        let default = ("\x1b[38;2;254;122;230m", "\x1b[38;2;255;255;255m");
        assert_eq!(distro_3d_colors("van"), van);
        assert_eq!(distro_3d_colors("Van"), van);
        assert_eq!(distro_3d_colors("Vanilla OS"), default);
        assert_eq!(distro_3d_colors("Arch Linux"), distro_3d_colors("arch"));
        assert_ne!(distro_3d_colors("Arch Linux"), distro_3d_colors("Ubuntu"));
    }

    #[test]
    fn invalid_color_falls_back_to_white() {
        assert_eq!(parse_color_to_ansi("not-a-color"), "\x1b[38;2;255;255;255m");
        assert_eq!(parse_color_to_ansi("#ff0000"), "\x1b[38;2;255;0;0m");
        assert_eq!(parse_color_to_ansi("\x1b[31m"), "\x1b[31m");
    }
}
