use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};
use tokio::time::{sleep, Duration};

/// One pending command waiting for its response.
struct Pending {
    tx: oneshot::Sender<Result<String, String>>,
}

/// Shared mutable state for the daemon (stdin + pending map).
struct DaemonState {
    stdin: ChildStdin,
    pending: HashMap<u32, Pending>,
}

/// A persistent ExifTool `-stay_open` daemon.
///
/// Commands are serialised through a sequential queue; each call gets a unique
/// sequence ID so the response reader can route `{ready####}` tokens back to
/// the correct caller.
pub struct ExifToolDaemon {
    state: Arc<Mutex<DaemonState>>,
    seq: Arc<AtomicU32>,
    /// Keep the child handle alive so it isn't dropped/killed prematurely.
    _child: Arc<Mutex<Child>>,
}

impl ExifToolDaemon {
    /// Spawn ExifTool in `-stay_open` mode.  Returns an error if the binary
    /// cannot be found on `PATH`.
    pub fn spawn() -> std::io::Result<Self> {
        Self::spawn_from("exiftool")
    }

    /// Like `spawn` but accepts a custom path to the ExifTool binary.
    pub fn spawn_from(binary: &str) -> std::io::Result<Self> {
        let mut child = Command::new(binary)
            .args(["-stay_open", "True", "-@", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let stdin = child.stdin.take().expect("stdin piped");
        let stdout = child.stdout.take().expect("stdout piped");

        let pending: HashMap<u32, Pending> = HashMap::new();
        let state = Arc::new(Mutex::new(DaemonState { stdin, pending }));
        let seq = Arc::new(AtomicU32::new(1));
        let child_arc = Arc::new(Mutex::new(child));

        // Spawn a background task to read stdout and resolve pending oneshots.
        let state_clone = Arc::clone(&state);
        tokio::spawn(async move {
            read_loop(stdout, state_clone).await;
        });

        Ok(Self {
            state,
            seq,
            _child: child_arc,
        })
    }

    /// Send a list of ExifTool arguments for a single file and return the raw
    /// stdout text that ExifTool emits before the `{ready####}` marker.
    ///
    /// `args` should NOT include `-execute`; the sequence suffix is appended
    /// automatically.
    pub async fn run(&self, args: &[&str]) -> Result<String, String> {
        let id = self.seq.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();

        {
            let mut guard = self.state.lock().await;
            guard.pending.insert(id, Pending { tx });

            // Write arguments to ExifTool stdin, one per line, then
            // -execute<id> to flush.
            for arg in args {
                guard
                    .stdin
                    .write_all(arg.as_bytes())
                    .map_err(|e| e.to_string())?;
                guard
                    .stdin
                    .write_all(b"\n")
                    .map_err(|e| e.to_string())?;
            }
            write!(guard.stdin, "-execute{:04}\n", id).map_err(|e| e.to_string())?;
            guard.stdin.flush().map_err(|e| e.to_string())?;
        }

        // Wait for the response reader to deliver our output.
        rx.await.map_err(|_| "ExifTool daemon shut down".to_string())?
    }

    /// Extract metadata from `path` as a JSON string.
    pub async fn extract_json(&self, path: &str) -> Result<String, String> {
        self.run(&[path, "-json", "-all:all", "-struct"]).await
    }

    /// Write metadata tags to `path`.  `tag_args` should be pairs like
    /// `["-EXIF:ImageDescription=My caption"]`.
    pub async fn write_tags(&self, path: &str, tag_args: &[String]) -> Result<String, String> {
        let mut args: Vec<&str> = Vec::with_capacity(tag_args.len() + 3);
        args.push(path);
        args.push("-overwrite_original_in_place");
        args.push("-preserve");
        for s in tag_args {
            args.push(s.as_str());
        }
        self.run(&args).await
    }
}

/// Background task: read stdout lines from ExifTool, accumulate output per
/// sequence ID, resolve the waiting oneshot when `{ready####}` is seen.
async fn read_loop(stdout: ChildStdout, state: Arc<Mutex<DaemonState>>) {
    let reader = BufReader::new(stdout);
    let mut buf = String::new();

    for line in reader.lines() {
        match line {
            Ok(line) => {
                // ExifTool marks the end of each response with {ready####}
                if let Some(id_str) = line.strip_prefix("{ready") {
                    let id_str = id_str.trim_end_matches('}');
                    if let Ok(id) = id_str.parse::<u32>() {
                        let output = std::mem::take(&mut buf);
                        let mut guard = state.lock().await;
                        if let Some(pending) = guard.pending.remove(&id) {
                            let _ = pending.tx.send(Ok(output));
                        }
                    }
                } else {
                    buf.push_str(&line);
                    buf.push('\n');
                }
            }
            Err(_) => {
                // ExifTool process died; fail all pending commands.
                let mut guard = state.lock().await;
                for (_, p) in guard.pending.drain() {
                    let _ = p.tx.send(Err("ExifTool process exited unexpectedly".to_string()));
                }
                break;
            }
        }
    }

    // If we exit the loop without draining, fail remaining pending.
    let mut guard = state.lock().await;
    for (_, p) in guard.pending.drain() {
        let _ = p.tx.send(Err("ExifTool process exited".to_string()));
    }
}

/// A self-healing wrapper around `ExifToolDaemon` that restarts after crashes
/// with exponential back-off.
pub struct ResilientExifToolDaemon {
    binary: String,
    inner: Arc<Mutex<Option<ExifToolDaemon>>>,
}

impl ResilientExifToolDaemon {
    pub fn new(binary: impl Into<String>) -> Self {
        let binary = binary.into();
        let inner = match ExifToolDaemon::spawn_from(&binary) {
            Ok(d) => Some(d),
            Err(_) => None,
        };
        Self {
            binary,
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    /// Send args to ExifTool, restarting the daemon if it has died.
    pub async fn run(&self, args: &[&str]) -> Result<String, String> {
        let mut delay_ms: u64 = 100;
        for attempt in 0..5u32 {
            let result = {
                let guard = self.inner.lock().await;
                match guard.as_ref() {
                    Some(d) => d.run(args).await,
                    None => Err("ExifTool daemon not available".to_string()),
                }
            };

            match result {
                Ok(output) => return Ok(output),
                Err(e) if attempt < 4 => {
                    // Restart the daemon and retry.
                    let mut guard = self.inner.lock().await;
                    *guard = ExifToolDaemon::spawn_from(&self.binary).ok();
                    sleep(Duration::from_millis(delay_ms)).await;
                    delay_ms = (delay_ms * 2).min(5000);
                    let _ = e;
                }
                Err(e) => return Err(e),
            }
        }
        Err("ExifTool daemon failed after 5 restart attempts".to_string())
    }

    pub async fn extract_json(&self, path: &str) -> Result<String, String> {
        self.run(&[path, "-json", "-all:all", "-struct"]).await
    }

    pub async fn write_tags(&self, path: &str, tag_args: &[String]) -> Result<String, String> {
        let mut args: Vec<&str> = Vec::with_capacity(tag_args.len() + 3);
        args.push(path);
        args.push("-overwrite_original_in_place");
        args.push("-preserve");
        for s in tag_args {
            args.push(s.as_str());
        }
        self.run(&args).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spawn ExifTool only if it exists; skip the test otherwise.
    fn exiftool_available() -> bool {
        Command::new("exiftool").arg("-ver").output().is_ok()
    }

    #[tokio::test]
    async fn sequential_queries_all_resolve() {
        if !exiftool_available() {
            return;
        }
        let daemon = ExifToolDaemon::spawn().unwrap();
        let mut handles = Vec::new();
        for _ in 0..10 {
            // -ver just prints the ExifTool version; fast and side-effect-free.
            let out = daemon.run(&["-ver"]).await;
            handles.push(out);
        }
        for result in handles {
            assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        }
    }
}
