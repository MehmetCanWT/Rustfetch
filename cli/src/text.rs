//! Terminal-safe text helpers shared by every output path.
//!
//! `strip_ansi`/`visible_width` are the single source of truth for width
//! measurement: widths must be computed with `UnicodeWidthStr` (which handles
//! emoji/ZWJ sequences) instead of summing per-code-point widths.

use unicode_width::UnicodeWidthStr;

/// Length in bytes of the escape sequence starting at `bytes[start]` (an ESC 0x1b).
fn escape_len_bytes(bytes: &[u8], start: usize) -> usize {
    let n = bytes.len();
    if start + 1 >= n {
        return 1;
    }
    match bytes[start + 1] {
        // CSI: ESC [ params final
        b'[' => {
            let mut j = start + 2;
            while j < n {
                let b = bytes[j];
                if (0x40..=0x7e).contains(&b) {
                    return j - start + 1;
                }
                if b == 0x1b {
                    return j - start;
                }
                j += 1;
            }
            n - start
        }
        // OSC: ESC ] ... BEL | ST
        b']' => {
            let mut j = start + 2;
            while j < n {
                let b = bytes[j];
                if b == 0x07 {
                    return j - start + 1;
                }
                if b == 0x1b {
                    if j + 1 < n && bytes[j + 1] == b'\\' {
                        return j - start + 2;
                    }
                    return j - start;
                }
                j += 1;
            }
            n - start
        }
        // DCS / SOS / PM / APC: ... ST
        b'P' | b'^' | b'_' | b'X' => {
            let mut j = start + 2;
            while j < n {
                if bytes[j] == 0x1b && j + 1 < n && bytes[j + 1] == b'\\' {
                    return j - start + 2;
                }
                j += 1;
            }
            n - start
        }
        _ => {
            // Two-or-more byte escape: intermediates then a final byte.
            let mut j = start + 1;
            while j < n && (0x20..=0x2f).contains(&bytes[j]) {
                j += 1;
            }
            if j < n && (0x30..=0x7e).contains(&bytes[j]) {
                j += 1;
            }
            j - start
        }
    }
}

/// Removes ANSI escape sequences (CSI/OSC/DCS/APC and two-byte escapes).
///
/// Avoids heap allocations for intermediate code-point buffers by scanning
/// UTF-8 bytes directly and slicing contiguous non-escape text runs.
pub fn strip_ansi(input: &str) -> String {
    let bytes = input.as_bytes();
    if !bytes.contains(&0x1b) {
        return input.to_string();
    }
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    let mut start = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b {
            if i > start {
                out.push_str(&input[start..i]);
            }
            let len = escape_len_bytes(bytes, i).max(1);
            i += len;
            start = i;
        } else {
            i += 1;
        }
    }
    if start < bytes.len() {
        out.push_str(&input[start..]);
    }
    out
}

/// Visible column width of `input` after stripping ANSI sequences.
///
/// Uses `UnicodeWidthStr` so emoji/ZWJ/VS16 sequences count as their real
/// terminal width instead of one column per code point.
///
/// Skips all string allocations when `input` does not contain escape characters.
pub fn visible_width(input: &str) -> usize {
    if !input.as_bytes().contains(&0x1b) {
        return UnicodeWidthStr::width(input);
    }
    UnicodeWidthStr::width(strip_ansi(input).as_str())
}

/// Removes ANSI sequences and every remaining control character.
///
/// Use this for any value that originates from the system or the network
/// (Wi-Fi SSID, hostname, process names, release titles, ...) before printing
/// it to a terminal.
pub fn sanitize_terminal(input: &str) -> String {
    strip_ansi(input)
        .chars()
        .filter(|c| !c.is_control())
        .collect()
}

/// Removes control characters but keeps intentional ANSI colour/OSC sequences.
#[allow(dead_code)]
pub fn sanitize_keep_ansi(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b {
            let len = escape_len_bytes(bytes, i).max(1);
            let end = (i + len).min(bytes.len());
            out.push_str(&input[i..end]);
            i = end;
            continue;
        }
        let rest = &input[i..];
        if let Some(c) = rest.chars().next() {
            if !c.is_control() {
                out.push(c);
            }
            i += c.len_utf8();
        } else {
            break;
        }
    }
    out
}

/// Writes `text` to stdout, returning false on any I/O failure.
///
/// Never panics: printing to a full disk or a closed pipe must not abort.
pub fn write_stdout(text: &str) -> bool {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    lock.write_all(text.as_bytes()).is_ok() && lock.flush().is_ok()
}

/// Writes `text` plus a trailing newline to stdout (false on I/O failure).
///
/// Avoids extra heap allocation by streaming both the text and the newline
/// through the locked stdout buffer.
pub fn write_stdout_line(text: &str) -> bool {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    lock.write_all(text.as_bytes()).is_ok() && lock.write_all(b"\n").is_ok() && lock.flush().is_ok()
}

/// Expand `{value}` and `{}` placeholders in a single pass so placeholders
/// inside `value` itself are not re-expanded.
pub(crate) fn expand_value_template(template: &str, value: &str) -> String {
    let mut out = String::with_capacity(template.len() + value.len());
    let mut rest = template;
    while let Some(idx) = rest.find('{') {
        out.push_str(&rest[..idx]);
        let after = &rest[idx..];
        if let Some(tail) = after.strip_prefix("{value}") {
            out.push_str(value);
            rest = tail;
        } else if let Some(tail) = after.strip_prefix("{}") {
            out.push_str(value);
            rest = tail;
        } else {
            out.push('{');
            rest = &after[1..];
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_expand_value_template_avoids_double_substitution() {
        assert_eq!(
            super::expand_value_template("{value} / {}", "x{}"),
            "x{} / x{}"
        );
        assert_eq!(
            super::expand_value_template("Pre {value} {unknown}", "{value}"),
            "Pre {value} {unknown}"
        );
    }
    use super::*;

    #[test]
    fn strips_sgr_and_unicode_wide_sequences() {
        assert_eq!(strip_ansi("\x1b[31mred\x1b[0m"), "red");
        assert_eq!(
            strip_ansi("\x1b]8;;https://example.com\x1b\\link\x1b]8;;\x1b\\"),
            "link"
        );
        assert_eq!(strip_ansi("\x1b(Bhi"), "hi");
        assert_eq!(strip_ansi("plain"), "plain");
    }

    #[test]
    fn visible_width_uses_unicode_width_str() {
        assert_eq!(visible_width("\x1b[1mabc\x1b[0m"), 3);
        assert_eq!(visible_width("👨‍👩‍👧"), 2);
        assert_eq!(visible_width("❤️"), 2);
    }

    #[test]
    fn sanitize_terminal_drops_control_characters() {
        assert_eq!(sanitize_terminal("a\x1b]0;pwn\x07b\x01c"), "abc");
        assert_eq!(sanitize_terminal("SSID\x1b[2J"), "SSID");
    }

    #[test]
    fn sanitize_keep_ansi_keeps_intentional_sequences() {
        assert_eq!(
            sanitize_keep_ansi("\x1b[31mred\x1b[0m\x01"),
            "\x1b[31mred\x1b[0m"
        );
    }

    #[test]
    fn write_stdout_line_is_non_panicking() {
        // Writing to the normal stdout keeps returning true in tests.
        assert!(write_stdout_line("rustfetch-test"));
    }
}
