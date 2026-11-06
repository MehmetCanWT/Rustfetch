use base64::prelude::*;
use std::fmt::Write as _;
use std::io::Write;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

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

/// Allocates a unique, non-zero kitty image id and registers it as ours.
fn allocate_image_id() -> u32 {
    let counter = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    // Mix the pid into the high bits so concurrent rustfetch processes do not
    // collide, and keep the per-process counter in the low bits.
    let id = ((pid & 0xFFFF) << 16) | (counter & 0xFFFF);
    let id = id.max(1);
    lock_tracked_ids().push(id);
    id
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
    if rgba_data.is_empty() || width == 0 || height == 0 || cols == 0 || rows == 0 {
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
    let _ = std::io::stdout().flush();
}

pub fn print_kitty(rgba_data: &[u8], width: u32, height: u32, cols: u32, rows: u32) {
    print_kitty_ex(rgba_data, width, height, cols, rows, 0, false);
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
    let mut sequences = Vec::new();
    if frames.is_empty() {
        return sequences;
    }

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
                        "\x1b_Ga=T,c={cols},r={rows},q=2,i={image_id},f=32{compression},s={width},v={height},m={m};{chunk}\x1b\\"
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
            "\x1b_Ga=a,i={image_id},r={},z={}\x1b\\",
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
    let _ = std::io::stdout().flush();
}

/// Builds deletion sequences for the given kitty image ids only.
pub fn build_clear_sequences(ids: &[u32]) -> Vec<String> {
    ids.iter()
        .filter(|id| **id != 0)
        .map(|id| format!("\x1b_Ga=d,d=i,i={id}\x1b\\"))
        .collect()
}

/// Deletes only the images rustfetch itself created in this process.
pub fn clear_kitty_images() {
    if !crate::terminal::supports_kitty() {
        return;
    }
    let ids = drain_tracked_ids();
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
        assert!(ids.iter().all(|id| *id != 0));
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
    fn print_kitty_ex_does_not_panic_for_empty_input() {
        print_kitty_ex(&[], 10, 10, 4, 2, 0, false);
        print_kitty(&[1, 2, 3, 4], 1, 1, 0, 0);
    }
}
