//! Timeout-aware subprocess execution helpers.
//!
//! Every external tool invocation in RustFetch must be bounded: a stuck
//! `bluetoothctl`, `nmcli`, `rpm` or `curl` must never hang the fetch.
//! `run_capture_with_timeout` spawns the child, drains stdout/stderr on helper
//! threads (so a large output can never deadlock against a full pipe buffer)
//! and kills the process once the deadline passes.

use std::io::Read;
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

/// Result of a bounded child-process run.
#[derive(Debug, Clone)]
pub struct CapturedOutput {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// True when the child was killed because it exceeded the deadline.
    pub timed_out: bool,
}

impl CapturedOutput {
    pub fn stdout_string(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }

    pub fn stderr_string(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }

    /// True only when the process exited successfully inside its deadline.
    pub fn success(&self) -> bool {
        !self.timed_out && self.status.success()
    }
}

const MAX_CAPTURE_BYTES: u64 = 1024 * 1024;

/// Spawns `command` with piped stdout/stderr and kills it after `timeout`.
///
/// Returns `None` only when the process could not be spawned at all.
pub fn run_capture_with_timeout(mut command: Command, timeout: Duration) -> Option<CapturedOutput> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;

    let stdout_pipe = child.stdout.take()?;
    let stderr_pipe = child.stderr.take()?;

    let stdout_reader = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = stdout_pipe.take(MAX_CAPTURE_BYTES).read_to_end(&mut buffer);
        buffer
    });
    let stderr_reader = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = stderr_pipe.take(MAX_CAPTURE_BYTES).read_to_end(&mut buffer);
        buffer
    });

    let deadline = Instant::now() + timeout;
    let mut timed_out = false;
    let status: Option<ExitStatus>;

    loop {
        match child.try_wait() {
            Ok(Some(exit)) => {
                status = Some(exit);
                break;
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    timed_out = true;
                    #[cfg(unix)]
                    {
                        let pgid = child.id() as libc::pid_t;
                        if pgid > 0 {
                            unsafe {
                                libc::kill(-pgid, libc::SIGKILL);
                            }
                        }
                    }
                    let _ = child.kill();
                    status = child.wait().ok();
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }

    let stdout = stdout_reader.join().unwrap_or_default();
    let stderr = stderr_reader.join().unwrap_or_default();

    Some(CapturedOutput {
        status: status?,
        stdout,
        stderr,
        timed_out,
    })
}

/// Convenience wrapper for an argv-style invocation.
pub fn run_with_timeout(program: &str, args: &[&str], timeout: Duration) -> Option<CapturedOutput> {
    let mut command = Command::new(program);
    command.args(args);
    run_capture_with_timeout(command, timeout)
}

/// Raised warning message for a timed-out optional source (for `--doctor`/debug use).
pub fn timeout_note(program: &str, timeout: Duration) -> String {
    format!(
        "warning: '{program}' did not answer within {} ms; value skipped",
        timeout.as_millis()
    )
}

/// Default deadline for optional information sources (sound, media, bluetooth, wifi).
pub const OPTIONAL_SOURCE_TIMEOUT: Duration = Duration::from_millis(750);
/// Deadline for package-manager fallbacks such as `rpm -qa`.
pub const PACKAGE_TOOL_TIMEOUT: Duration = Duration::from_millis(2_000);
/// Deadline for network metadata fetches (update checker / self-update).
pub const NETWORK_TIMEOUT: Duration = Duration::from_millis(4_000);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_stdout_and_success() {
        let output = run_with_timeout("sh", &["-c", "printf hello"], Duration::from_secs(5))
            .expect("sh must be spawnable");
        assert!(output.success());
        assert_eq!(output.stdout_string(), "hello");
        assert!(!output.timed_out);
    }

    #[test]
    fn captures_stderr_and_failure() {
        let output = run_with_timeout(
            "sh",
            &["-c", "printf oops >&2; exit 3"],
            Duration::from_secs(5),
        )
        .expect("sh must be spawnable");
        assert!(!output.success());
        assert_eq!(output.status.code(), Some(3));
        assert_eq!(output.stderr_string(), "oops");
    }

    #[test]
    fn kills_process_after_deadline() {
        let started = Instant::now();
        let output = run_with_timeout("sleep", &["30"], Duration::from_millis(80))
            .expect("sleep must be spawnable");
        assert!(output.timed_out);
        assert!(!output.success());
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "timed-out child was not killed promptly"
        );
    }

    #[test]
    fn missing_program_returns_none() {
        assert!(
            run_with_timeout(
                "definitely-not-a-real-program-rustfetch-test",
                &[],
                Duration::from_millis(100)
            )
            .is_none()
        );
    }
}
