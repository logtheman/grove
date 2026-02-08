use anyhow::Result;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    shutdown: Arc<AtomicBool>,
}

impl PtySession {
    pub fn spawn(
        cwd: &str,
        rows: u16,
        cols: u16,
        on_data: impl Fn(Vec<u8>) + Send + 'static,
        on_exit: impl Fn() + Send + 'static,
    ) -> Result<Self> {
        eprintln!("[grove-pty] spawn: cwd={}, rows={}, cols={}", cwd, rows, cols);

        let pty_system = native_pty_system();

        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        eprintln!("[grove-pty] openpty succeeded");

        // IMPORTANT: On macOS, reader and writer must be obtained from master
        // BEFORE spawning the command and BEFORE dropping the slave.
        // Otherwise the reader FD may not receive PTY output.
        let mut reader = pair.master.try_clone_reader()?;
        eprintln!("[grove-pty] got reader");
        let writer = pair.master.take_writer()?;
        eprintln!("[grove-pty] got writer");

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        eprintln!("[grove-pty] using shell: {}", shell);

        let mut cmd = CommandBuilder::new(&shell);
        // Login shell flag to source profile on macOS
        cmd.arg("-l");
        cmd.cwd(cwd);
        cmd.env("TERM", "xterm-256color");

        let mut child = pair.slave.spawn_command(cmd)?;
        eprintln!("[grove-pty] spawn_command succeeded");

        // Drop slave after spawning - master keeps the PTY open
        drop(pair.slave);
        eprintln!("[grove-pty] slave dropped");

        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_reader = shutdown.clone();

        // Spawn reader thread - reads PTY output and sends to frontend
        std::thread::spawn(move || {
            eprintln!("[grove-pty] reader thread started");
            let mut buf = [0u8; 4096];
            loop {
                if shutdown_reader.load(Ordering::Relaxed) {
                    eprintln!("[grove-pty] reader thread: shutdown requested");
                    break;
                }

                match reader.read(&mut buf) {
                    Ok(0) => {
                        eprintln!("[grove-pty] reader thread: EOF");
                        break;
                    }
                    Ok(n) => {
                        eprintln!("[grove-pty] reader thread: read {} bytes", n);
                        on_data(buf[..n].to_vec());
                    }
                    Err(e) => {
                        eprintln!("[grove-pty] reader thread: error: {} (kind={:?})", e, e.kind());
                        if e.kind() != std::io::ErrorKind::WouldBlock {
                            break;
                        }
                    }
                }
            }
            eprintln!("[grove-pty] reader thread exiting");
        });

        // Spawn child-wait thread - detects when shell process exits
        std::thread::spawn(move || {
            eprintln!("[grove-pty] child-wait thread started");
            match child.wait() {
                Ok(status) => {
                    eprintln!("[grove-pty] child exited: {:?}", status);
                }
                Err(e) => {
                    eprintln!("[grove-pty] child wait error: {}", e);
                }
            }
            on_exit();
        });

        Ok(Self {
            master: pair.master,
            writer,
            shutdown,
        })
    }

    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        self.writer.write_all(data)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn resize(&self, rows: u16, cols: u16) -> Result<()> {
        self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }

    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        self.shutdown();
    }
}
