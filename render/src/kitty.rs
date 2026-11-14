use base64::prelude::*;
use std::fmt::Write as _;
use std::io::Write;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::terminal::wrap_tmux_passthrough;

/// Maximum number of base64 bytes per kitty graphics chunk.
const KITTY_CHUNK_SIZE: usize = 4096;

/// Counter used to build unique image ids within this process.
static ID_COUNTER: AtomicU32 = AtomicU32::new(1);

/// Image ids created by this process, so cleanup only removes our own images.
static TRACKED_IDS: Mutex<Vec<u32>> = Mutex::new(Vec::new());

fn lock_tracked_ids() -> std::sync::MutexGuard<'static, Vec<u32>> {
    TRACKED_IDS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Kitty Unicode placeholder base character (Private Use Area).
pub const PLACEHOLDER_CHAR: char = '\u{10EEEE}';

/// Kitty row/column combining diacritics in exact index order (0..297).
///
/// Generated from Kitty's `rowcolumn-diacritics.txt`.
#[rustfmt::skip]
pub const DIACRITICS: [char; 297] = [
    '\u{0305}', '\u{030D}', '\u{030E}', '\u{0310}', '\u{0312}', '\u{033D}', '\u{033E}', '\u{033F}', '\u{0346}', '\u{034A}', '\u{034B}', '\u{034C}',
    '\u{0350}', '\u{0351}', '\u{0352}', '\u{0357}', '\u{035B}', '\u{0363}', '\u{0364}', '\u{0365}', '\u{0366}', '\u{0367}', '\u{0368}', '\u{0369}',
    '\u{036A}', '\u{036B}', '\u{036C}', '\u{036D}', '\u{036E}', '\u{036F}', '\u{0483}', '\u{0484}', '\u{0485}', '\u{0486}', '\u{0487}', '\u{0592}',
    '\u{0593}', '\u{0594}', '\u{0595}', '\u{0597}', '\u{0598}', '\u{0599}', '\u{059C}', '\u{059D}', '\u{059E}', '\u{059F}', '\u{05A0}', '\u{05A1}',
    '\u{05A8}', '\u{05A9}', '\u{05AB}', '\u{05AC}', '\u{05AF}', '\u{05C4}', '\u{0610}', '\u{0611}', '\u{0612}', '\u{0613}', '\u{0614}', '\u{0615}',
    '\u{0616}', '\u{0617}', '\u{0657}', '\u{0658}', '\u{0659}', '\u{065A}', '\u{065B}', '\u{065D}', '\u{065E}', '\u{06D6}', '\u{06D7}', '\u{06D8}',
    '\u{06D9}', '\u{06DA}', '\u{06DB}', '\u{06DC}', '\u{06DF}', '\u{06E0}', '\u{06E1}', '\u{06E2}', '\u{06E4}', '\u{06E7}', '\u{06E8}', '\u{06EB}',
    '\u{06EC}', '\u{0730}', '\u{0732}', '\u{0733}', '\u{0735}', '\u{0736}', '\u{073A}', '\u{073D}', '\u{073F}', '\u{0740}', '\u{0741}', '\u{0743}',
    '\u{0745}', '\u{0747}', '\u{0749}', '\u{074A}', '\u{07EB}', '\u{07EC}', '\u{07ED}', '\u{07EE}', '\u{07EF}', '\u{07F0}', '\u{07F1}', '\u{07F3}',
    '\u{0816}', '\u{0817}', '\u{0818}', '\u{0819}', '\u{081B}', '\u{081C}', '\u{081D}', '\u{081E}', '\u{081F}', '\u{0820}', '\u{0821}', '\u{0822}',
    '\u{0823}', '\u{0825}', '\u{0826}', '\u{0827}', '\u{0829}', '\u{082A}', '\u{082B}', '\u{082C}', '\u{082D}', '\u{0951}', '\u{0953}', '\u{0954}',
    '\u{0F82}', '\u{0F83}', '\u{0F86}', '\u{0F87}', '\u{135D}', '\u{135E}', '\u{135F}', '\u{17DD}', '\u{193A}', '\u{1A17}', '\u{1A75}', '\u{1A76}',
    '\u{1A77}', '\u{1A78}', '\u{1A79}', '\u{1A7A}', '\u{1A7B}', '\u{1A7C}', '\u{1B6B}', '\u{1B6D}', '\u{1B6E}', '\u{1B6F}', '\u{1B70}', '\u{1B71}',
    '\u{1B72}', '\u{1B73}', '\u{1CD0}', '\u{1CD1}', '\u{1CD2}', '\u{1CDA}', '\u{1CDB}', '\u{1CE0}', '\u{1DC0}', '\u{1DC1}', '\u{1DC3}', '\u{1DC4}',
    '\u{1DC5}', '\u{1DC6}', '\u{1DC7}', '\u{1DC8}', '\u{1DC9}', '\u{1DCB}', '\u{1DCC}', '\u{1DD1}', '\u{1DD2}', '\u{1DD3}', '\u{1DD4}', '\u{1DD5}',
    '\u{1DD6}', '\u{1DD7}', '\u{1DD8}', '\u{1DD9}', '\u{1DDA}', '\u{1DDB}', '\u{1DDC}', '\u{1DDD}', '\u{1DDE}', '\u{1DDF}', '\u{1DE0}', '\u{1DE1}',
    '\u{1DE2}', '\u{1DE3}', '\u{1DE4}', '\u{1DE5}', '\u{1DE6}', '\u{1DFE}', '\u{20D0}', '\u{20D1}', '\u{20D4}', '\u{20D5}', '\u{20D6}', '\u{20D7}',
    '\u{20DB}', '\u{20DC}', '\u{20E1}', '\u{20E7}', '\u{20E9}', '\u{20F0}', '\u{2CEF}', '\u{2CF0}', '\u{2CF1}', '\u{2DE0}', '\u{2DE1}', '\u{2DE2}',
    '\u{2DE3}', '\u{2DE4}', '\u{2DE5}', '\u{2DE6}', '\u{2DE7}', '\u{2DE8}', '\u{2DE9}', '\u{2DEA}', '\u{2DEB}', '\u{2DEC}', '\u{2DED}', '\u{2DEE}',
    '\u{2DEF}', '\u{2DF0}', '\u{2DF1}', '\u{2DF2}', '\u{2DF3}', '\u{2DF4}', '\u{2DF5}', '\u{2DF6}', '\u{2DF7}', '\u{2DF8}', '\u{2DF9}', '\u{2DFA}',
    '\u{2DFB}', '\u{2DFC}', '\u{2DFD}', '\u{2DFE}', '\u{2DFF}', '\u{A66F}', '\u{A67C}', '\u{A67D}', '\u{A6F0}', '\u{A6F1}', '\u{A8E0}', '\u{A8E1}',
    '\u{A8E2}', '\u{A8E3}', '\u{A8E4}', '\u{A8E5}', '\u{A8E6}', '\u{A8E7}', '\u{A8E8}', '\u{A8E9}', '\u{A8EA}', '\u{A8EB}', '\u{A8EC}', '\u{A8ED}',
    '\u{A8EE}', '\u{A8EF}', '\u{A8F0}', '\u{A8F1}', '\u{AAB0}', '\u{AAB2}', '\u{AAB3}', '\u{AAB7}', '\u{AAB8}', '\u{AABE}', '\u{AABF}', '\u{AAC1}',
    '\u{FE20}', '\u{FE21}', '\u{FE22}', '\u{FE23}', '\u{FE24}', '\u{FE25}', '\u{FE26}', '\u{10A0F}', '\u{10A38}', '\u{1D185}', '\u{1D186}', '\u{1D187}',
    '\u{1D188}', '\u{1D189}', '\u{1D1AA}', '\u{1D1AB}', '\u{1D1AC}', '\u{1D1AD}', '\u{1D242}', '\u{1D243}', '\u{1D244}',
];

/// Allocates a unique, non-zero 24-bit kitty image id (`1..=0x00FF_FFFF`) and
/// registers it as owned by this process.
fn allocate_image_id() -> u32 {
    let counter = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    // Mix the low PID byte into bits 16..24 so concurrent rustfetch processes
    // avoid collisions while keeping every ID representable in 24-bit RGB
    // foreground escapes for Kitty Unicode placeholders.
    let pid_byte = pid & 0xFF;
    let seq_low = (counter.wrapping_sub(1) % 0xFFFE) + 1;
    let id = (pid_byte << 16) | seq_low;
    lock_tracked_ids().push(id);
    id
}

fn window_id_tag() -> String {
    let raw = std::env::var("KITTY_WINDOW_ID")
        .or_else(|_| std::env::var("WINDOWID"))
        .or_else(|_| std::env::var("WEZTERM_PANE"))
        .unwrap_or_else(|_| "default".to_string());
    let cleaned: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .take(32)
        .collect();
    if cleaned.is_empty() {
        "default".to_string()
    } else {
        cleaned
    }
}

fn persisted_ids_path() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_dir())
        .or_else(|| {
            std::env::var_os("XDG_CACHE_HOME")
                .map(std::path::PathBuf::from)
                .filter(|p| !p.as_os_str().is_empty())
        })
        .or_else(|| {
            std::env::var_os("HOME")
                .map(std::path::PathBuf::from)
                .map(|h| h.join(".cache"))
        })?;
    Some(
        base.join("rustfetch")
            .join(format!("kitty-ids-{}.txt", window_id_tag())),
    )
}

fn persist_tracked_id(id: u32) {
    if id == 0 {
        return;
    }
    let Some(path) = persisted_ids_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut ids: Vec<u32> = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|tok| tok.parse::<u32>().ok())
        .filter(|v| *v != 0 && *v != id)
        .collect();
    ids.push(id);
    if ids.len() > 64 {
        let drain_to = ids.len() - 64;
        ids.drain(0..drain_to);
    }
    let body = ids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    let _ = std::fs::write(&path, body);
}

fn take_persisted_ids() -> Vec<u32> {
    let Some(path) = persisted_ids_path() else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let _ = std::fs::remove_file(&path);
    text.split_whitespace()
        .filter_map(|tok| tok.parse::<u32>().ok())
        .filter(|v| *v != 0)
        .collect()
}

/// Encodes a 24-bit Kitty image id into `(red, green, blue)` foreground bytes.
pub fn encode_image_id_rgb(image_id: u32) -> (u8, u8, u8) {
    (
        ((image_id >> 16) & 0xFF) as u8,
        ((image_id >> 8) & 0xFF) as u8,
        (image_id & 0xFF) as u8,
    )
}

/// Builds per-row Kitty Unicode placeholder strings (`U+10EEEE` + row/col diacritics)
/// for a virtual placement created with `U=1`.
pub fn build_placeholder_rows(image_id: u32, cols: usize, rows: usize) -> Vec<String> {
    if image_id == 0 || cols == 0 || rows == 0 {
        return Vec::new();
    }
    let max_dim = DIACRITICS.len();
    let cols = cols.min(max_dim);
    let rows = rows.min(max_dim);
    let (r, g, b) = encode_image_id_rgb(image_id);
    let mut out = Vec::with_capacity(rows);
    for &row_diacritic in &DIACRITICS[..rows] {
        let mut line = String::with_capacity(cols * 8 + 24);
        let _ = write!(line, "\x1b[38;2;{r};{g};{b}m");
        for &col_diacritic in &DIACRITICS[..cols] {
            line.push(PLACEHOLDER_CHAR);
            line.push(row_diacritic);
            line.push(col_diacritic);
        }
        line.push_str("\x1b[39m");
        out.push(line);
    }
    out
}

/// Removes and returns every image id created by this process.
fn drain_tracked_ids() -> Vec<u32> {
    let mut guard = lock_tracked_ids();
    std::mem::take(&mut *guard)
}

fn compress_and_encode(rgba_data: &[u8]) -> (bool, String) {
    let compressed = miniz_oxide::deflate::compress_to_vec_zlib(rgba_data, 6);
    let use_compression = compressed.len() < rgba_data.len();
    let data = if use_compression {
        &compressed[..]
    } else {
        rgba_data
    };
    (use_compression, BASE64_STANDARD.encode(data))
}

/// Splits a base64 payload into protocol-sized chunks.
fn split_chunks(encoded: &str) -> Vec<&str> {
    let mut chunks = Vec::with_capacity(encoded.len() / KITTY_CHUNK_SIZE + 1);
    for raw in encoded.as_bytes().chunks(KITTY_CHUNK_SIZE) {
        // Base64 output is always ASCII, so this can never fail in practice.
        if let Ok(chunk) = std::str::from_utf8(raw) {
            chunks.push(chunk);
        }
    }
    chunks
}

pub fn print_kitty_ex(
    rgba_data: &[u8],
    width: u32,
    height: u32,
    cols: u32,
    rows: u32,
    z_index: i32,
    prevent_cursor_move: bool,
) {
    let Some(expected) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|px| px.checked_mul(4))
    else {
        return;
    };
    if width == 0 || height == 0 || cols == 0 || rows == 0 || rgba_data.len() < expected {
        return;
    }

    let image_id = allocate_image_id();
    let (use_compression, encoded) = compress_and_encode(rgba_data);
    let chunks = split_chunks(&encoded);

    for (i, chunk) in chunks.iter().enumerate() {
        let m = if i == chunks.len() - 1 { 0 } else { 1 };
        let seq = if i == 0 {
            let mut cmd =
                format!("\x1b_Ga=T,c={cols},r={rows},q=2,i={image_id},f=32,s={width},v={height}");
            if use_compression {
                cmd.push_str(",o=z");
            }
            if z_index != 0 {
                let _ = write!(cmd, ",z={z_index}");
            }
            if prevent_cursor_move {
                cmd.push_str(",C=1");
            }
            let _ = write!(cmd, ",m={m};{chunk}\x1b\\");
            cmd
        } else {
            format!("\x1b_Gm={m},q=2;{chunk}\x1b\\")
        };
        print!("{}", wrap_tmux_passthrough(&seq));
    }
    persist_tracked_id(image_id);
    let _ = std::io::stdout().flush();
}

pub fn print_kitty(rgba_data: &[u8], width: u32, height: u32, cols: u32, rows: u32) {
    print_kitty_ex(rgba_data, width, height, cols, rows, 0, false);
}

/// Builds chunks for transmitting an image as a Kitty Unicode-placeholder virtual
/// placement (`a=T,U=1`).
pub fn build_virtual_transmit_sequences(
    rgba_data: &[u8],
    width: u32,
    height: u32,
    cols: u32,
    rows: u32,
    image_id: u32,
) -> Vec<String> {
    let Some(expected) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|px| px.checked_mul(4))
    else {
        return Vec::new();
    };
    if image_id == 0
        || width == 0
        || height == 0
        || cols == 0
        || rows == 0
        || rgba_data.len() < expected
    {
        return Vec::new();
    }
    let (use_compression, encoded) = compress_and_encode(rgba_data);
    let chunks = split_chunks(&encoded);
    let mut sequences = Vec::with_capacity(chunks.len());
    for (i, chunk) in chunks.iter().enumerate() {
        let m = if i == chunks.len() - 1 { 0 } else { 1 };
        let seq = if i == 0 {
            let mut cmd = format!(
                "\x1b_Ga=T,U=1,c={cols},r={rows},q=2,i={image_id},f=32,s={width},v={height}"
            );
            if use_compression {
                cmd.push_str(",o=z");
            }
            let _ = write!(cmd, ",m={m};{chunk}\x1b\\");
            cmd
        } else {
            format!("\x1b_Gm={m},q=2;{chunk}\x1b\\")
        };
        sequences.push(seq);
    }
    sequences
}

/// Transmits an RGBA image as a Kitty virtual placement (`U=1`) and returns
/// `(image_id, placeholder_rows)` for inline grid placement.
pub fn print_kitty_virtual(
    rgba_data: &[u8],
    width: u32,
    height: u32,
    cols: u32,
    rows: u32,
) -> Option<(u32, Vec<String>)> {
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|px| px.checked_mul(4))?;
    if width == 0 || height == 0 || cols == 0 || rows == 0 || rgba_data.len() < expected {
        return None;
    }
    let image_id = allocate_image_id();
    let seqs = build_virtual_transmit_sequences(rgba_data, width, height, cols, rows, image_id);
    if seqs.is_empty() {
        return None;
    }
    for seq in &seqs {
        print!("{}", wrap_tmux_passthrough(seq));
    }
    persist_tracked_id(image_id);
    let _ = std::io::stdout().flush();
    Some((
        image_id,
        build_placeholder_rows(image_id, cols as usize, rows as usize),
    ))
}

pub struct KittyFrame<'a> {
    pub rgba_data: &'a [u8],
    pub delay_ms: u32,
}

/// Builds every control sequence for an animated kitty image.
///
/// Protocol rules honoured here:
/// * the root image is transmitted exactly once with a=T and c/r (cell
///   dimensions) and WITHOUT a z key (in a=T, z means z-index);
/// * additional frames are transmitted with a=f and NEVER carry c/r, because
///   for animation frame loading those keys are frame numbers;
/// * per-frame delays are sent as separate a=a,r=<frame>,z=<delay> control
///   packets (frame numbers are 1-based, root frame is 1);
/// * the animation is started with a=a,s=3,v=1 (loop forever).
///
/// Returns the complete escape sequences, each already terminated with ST.
pub fn build_animation_sequences(
    frames: &[KittyFrame],
    width: u32,
    height: u32,
    cols: u32,
    rows: u32,
    image_id: u32,
) -> Vec<String> {
    build_animation_sequences_ex(frames, width, height, cols, rows, image_id, false)
}

/// Builds every control sequence for an animated kitty image, optionally marking
/// the root transmission as a Unicode-placeholder virtual placement (`U=1`).
pub fn build_animation_sequences_ex(
    frames: &[KittyFrame],
    width: u32,
    height: u32,
    cols: u32,
    rows: u32,
    image_id: u32,
    virtual_placeholder: bool,
) -> Vec<String> {
    let mut sequences = Vec::new();
    if frames.is_empty() {
        return sequences;
    }
    let virtual_flag = if virtual_placeholder { ",U=1" } else { "" };

    for (frame_idx, frame) in frames.iter().enumerate() {
        let (use_compression, encoded) = compress_and_encode(frame.rgba_data);
        let chunks = split_chunks(&encoded);
        if chunks.is_empty() {
            continue;
        }

        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let m = if chunk_idx == chunks.len() - 1 { 0 } else { 1 };
            let compression = if use_compression { ",o=z" } else { "" };
            let seq = if chunk_idx == 0 {
                if frame_idx == 0 {
                    format!(
                        "\x1b_Ga=T{virtual_flag},c={cols},r={rows},q=2,i={image_id},f=32{compression},s={width},v={height},m={m};{chunk}\x1b\\"
                    )
                } else {
                    format!(
                        "\x1b_Ga=f,q=2,i={image_id},f=32{compression},s={width},v={height},m={m};{chunk}\x1b\\"
                    )
                }
            } else if frame_idx == 0 {
                format!("\x1b_Gm={m},q=2;{chunk}\x1b\\")
            } else {
                // Continuation chunks of animation frame data must repeat a=f.
                format!("\x1b_Ga=f,q=2,m={m};{chunk}\x1b\\")
            };
            sequences.push(seq);
        }

        sequences.push(format!(
            "\x1b_Ga=a,q=2,i={image_id},r={},z={}\x1b\\",
            frame_idx + 1,
            frame.delay_ms
        ));
    }

    sequences.push(format!("\x1b_Ga=a,q=2,i={image_id},s=3,v=1\x1b\\"));
    sequences
}

pub fn print_kitty_animation(frames: &[KittyFrame], width: u32, height: u32, cols: u32, rows: u32) {
    if frames.is_empty() || width == 0 || height == 0 || cols == 0 || rows == 0 {
        return;
    }

    let image_id = allocate_image_id();
    for seq in build_animation_sequences(frames, width, height, cols, rows, image_id) {
        print!("{}", wrap_tmux_passthrough(&seq));
    }
    persist_tracked_id(image_id);
    let _ = std::io::stdout().flush();
}

/// Transmits an animated GIF/image as a Kitty virtual placement (`U=1`) and
/// returns `(image_id, placeholder_rows)`.
pub fn print_kitty_animation_virtual(
    frames: &[KittyFrame],
    width: u32,
    height: u32,
    cols: u32,
    rows: u32,
) -> Option<(u32, Vec<String>)> {
    if frames.is_empty() || width == 0 || height == 0 || cols == 0 || rows == 0 {
        return None;
    }
    let image_id = allocate_image_id();
    let seqs = build_animation_sequences_ex(frames, width, height, cols, rows, image_id, true);
    if seqs.is_empty() {
        return None;
    }
    for seq in &seqs {
        print!("{}", wrap_tmux_passthrough(seq));
    }
    persist_tracked_id(image_id);
    let _ = std::io::stdout().flush();
    Some((
        image_id,
        build_placeholder_rows(image_id, cols as usize, rows as usize),
    ))
}

/// Builds deletion sequences for the given kitty image ids only.
pub fn build_clear_sequences(ids: &[u32]) -> Vec<String> {
    ids.iter()
        .filter(|id| **id != 0)
        .map(|id| format!("\x1b_Ga=d,d=i,i={id},q=2\x1b\\"))
        .collect()
}

/// Deletes images created by rustfetch in this process or a prior run on the
/// same terminal window.
pub fn clear_kitty_images() {
    let mut ids = drain_tracked_ids();
    for pid_id in take_persisted_ids() {
        if !ids.contains(&pid_id) {
            ids.push(pid_id);
        }
    }
    if !crate::terminal::supports_kitty() {
        return;
    }
    for seq in build_clear_sequences(&ids) {
        print!("{}", wrap_tmux_passthrough(&seq));
    }
    let _ = std::io::stdout().flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames() -> Vec<KittyFrame<'static>> {
        vec![
            KittyFrame {
                rgba_data: &[255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 9, 9, 9, 255],
                delay_ms: 100,
            },
            KittyFrame {
                rgba_data: &[
                    10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255, 1, 2, 3, 255,
                ],
                delay_ms: 500,
            },
            KittyFrame {
                rgba_data: &[1, 1, 1, 255, 2, 2, 2, 255, 3, 3, 3, 255, 4, 4, 4, 255],
                delay_ms: 1000,
            },
        ]
    }

    #[test]
    fn animation_sequences_use_protocol_correct_keys() {
        let frames = frames();
        let seqs = build_animation_sequences(&frames, 2, 2, 4, 3, 42);
        assert!(!seqs.is_empty());

        let root: Vec<&String> = seqs.iter().filter(|s| s.contains("a=T")).collect();
        assert_eq!(root.len(), 1, "root image must be transmitted exactly once");
        assert!(root[0].contains("c=4"));
        assert!(root[0].contains("r=3"));
        assert!(
            !root[0].contains("z="),
            "root transmit must not carry a z-index key"
        );
        assert!(root[0].contains("i=42"));

        let frame_data: Vec<&String> = seqs.iter().filter(|s| s.contains("a=f")).collect();
        assert!(!frame_data.is_empty(), "additional frames must use a=f");
        for seq in &frame_data {
            assert!(
                !seq.contains("c="),
                "a=f must not carry cell columns: {seq}"
            );
            assert!(!seq.contains("r="), "a=f must not carry cell rows: {seq}");
        }

        let gaps: Vec<&String> = seqs
            .iter()
            .filter(|s| s.contains("a=a") && s.contains("z="))
            .collect();
        assert_eq!(gaps.len(), frames.len(), "one gap packet per frame");
        assert!(gaps[0].contains("r=1") && gaps[0].contains("z=100"));
        assert!(gaps[1].contains("r=2") && gaps[1].contains("z=500"));
        assert!(gaps[2].contains("r=3") && gaps[2].contains("z=1000"));

        let last = seqs.last().expect("non-empty sequence list");
        assert!(last.contains("a=a"));
        assert!(last.contains("s=3"));
        assert!(last.contains("v=1"));
        assert!(last.contains("i=42"));
    }

    #[test]
    fn animation_frame_chunks_repeat_a_equals_f() {
        // Incompressible (xorshift) data forces multiple base64 chunks.
        let mut state: u32 = 0x1234_5678;
        let data: Vec<u8> = (0..(256 * 64 * 4))
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                (state & 0xFF) as u8
            })
            .collect();
        let frames = [KittyFrame {
            rgba_data: &data,
            delay_ms: 40,
        }];
        let seqs = build_animation_sequences(&frames, 256, 64, 8, 8, 7);
        let continuations: Vec<&String> = seqs
            .iter()
            .filter(|s| s.contains("m=1") && !s.contains("a=T") && !s.contains("a=a"))
            .collect();
        assert!(
            !continuations.is_empty(),
            "expected chunked animation frame data"
        );
        for seq in continuations {
            assert!(seq.contains("a=f") || seq.starts_with("\x1b_Gm="));
        }
    }

    #[test]
    fn clear_sequences_only_target_our_ids() {
        let seqs = build_clear_sequences(&[7, 9]);
        assert_eq!(seqs.len(), 2);
        assert!(seqs[0].contains("a=d,d=i,i=7"));
        assert!(seqs[1].contains("a=d,d=i,i=9"));
        for seq in &seqs {
            assert!(!seq.contains("d=a"), "must never delete every image: {seq}");
        }
        assert!(build_clear_sequences(&[0]).is_empty());
    }

    #[test]
    fn image_ids_are_unique_and_nonzero() {
        let mut ids = Vec::new();
        for _ in 0..64 {
            ids.push(allocate_image_id());
        }
        assert!(ids.iter().all(|id| *id != 0 && *id <= 0x00FF_FFFF));
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "image ids must be unique");
        let tracked = lock_tracked_ids();
        for id in &ids {
            assert!(tracked.contains(id), "allocated id must be tracked");
        }
    }

    #[test]
    fn placeholder_rows_encode_24bit_id_and_row_col_diacritics() {
        assert_eq!(DIACRITICS.len(), 297);
        assert_eq!(encode_image_id_rgb(0x12_34_56), (0x12, 0x34, 0x56));
        let rows = build_placeholder_rows(0x01_02_03, 2, 2);
        assert_eq!(rows.len(), 2);
        assert!(rows[0].starts_with("\x1b[38;2;1;2;3m"));
        assert!(rows[0].ends_with("\x1b[39m"));
        assert!(rows[0].contains(&format!(
            "{PLACEHOLDER_CHAR}{}{}",
            DIACRITICS[0], DIACRITICS[0]
        )));
        assert!(rows[0].contains(&format!(
            "{PLACEHOLDER_CHAR}{}{}",
            DIACRITICS[0], DIACRITICS[1]
        )));
        assert!(rows[1].contains(&format!(
            "{PLACEHOLDER_CHAR}{}{}",
            DIACRITICS[1], DIACRITICS[0]
        )));
        assert!(rows[1].contains(&format!(
            "{PLACEHOLDER_CHAR}{}{}",
            DIACRITICS[1], DIACRITICS[1]
        )));
        assert!(build_placeholder_rows(0, 2, 2).is_empty());
    }

    #[test]
    fn virtual_transmit_and_animation_set_u_equals_1() {
        let rgba = [255u8; 16];
        let v_seqs = build_virtual_transmit_sequences(&rgba, 2, 2, 4, 3, 99);
        assert!(!v_seqs.is_empty());
        assert!(v_seqs[0].contains("a=T,U=1,c=4,r=3"));
        assert!(v_seqs[0].contains("i=99"));

        let anim_frames = frames();
        let a_seqs = build_animation_sequences_ex(&anim_frames, 2, 2, 4, 3, 55, true);
        let root = a_seqs.iter().find(|s| s.contains("a=T")).unwrap();
        assert!(root.contains("U=1"));
        assert!(root.contains("i=55"));
    }

    #[test]
    fn print_kitty_ex_does_not_panic_for_empty_input() {
        print_kitty_ex(&[], 10, 10, 4, 2, 0, false);
        print_kitty(&[1, 2, 3, 4], 1, 1, 0, 0);
    }
}
