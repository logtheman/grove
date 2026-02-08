use anyhow::Result;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use tokio::sync::mpsc;

pub struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    shutdown_tx: mpsc::Sender<()>,
}

impl PtySession {
    pub fn spawn(
        cwd: &str,
        rows: u16,
        cols: u16,
        on_data: impl Fn(Vec<u8>) + Send + 'static,
    ) -> Result<Self> {
        let pty_system = native_pty_system();

        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        let mut cmd = CommandBuilder::new(&shell);
        cmd.cwd(cwd);

        // Set TERM for proper terminal emulation
        cmd.env("TERM", "xterm-256color");

        pair.slave.spawn_command(cmd)?;
        // Drop slave after spawning - not needed anymore
        drop(pair.slave);

        let writer = pair.master.take_writer()?;
        let mut reader = pair.master.try_clone_reader()?;

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        // Spawn reader thread
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                // Check for shutdown
                if shutdown_rx.try_recv().is_ok() {
                    break;
                }

                match reader.read(&mut buf) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        on_data(buf[..n].to_vec());
                    }
                    Err(e) => {
                        if e.kind() != std::io::ErrorKind::WouldBlock {
                            break;
                        }
                    }
                }
            }
        });

        Ok(Self {
            master: pair.master,
            writer,
            shutdown_tx,
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
        let _ = self.shutdown_tx.try_send(());
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        self.shutdown();
    }
}
