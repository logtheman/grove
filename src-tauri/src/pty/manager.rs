use anyhow::Result;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::collections::HashMap;

pub struct CommandExecution {
    pub output: String,
    pub complete: bool,
}

pub struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    shutdown: Arc<AtomicBool>,
    pub current_cwd: Arc<Mutex<String>>,
    pub pending_commands: Arc<Mutex<HashMap<String, CommandExecution>>>,
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
        // Enable OSC 7 for CWD tracking
        cmd.env("PROMPT_COMMAND", "printf '\\e]7;file://%s%s\\e\\\\' \"$HOSTNAME\" \"$PWD\"");

        let mut child = pair.slave.spawn_command(cmd)?;
        eprintln!("[grove-pty] spawn_command succeeded");

        // Drop slave after spawning - master keeps the PTY open
        drop(pair.slave);
        eprintln!("[grove-pty] slave dropped");

        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_reader = shutdown.clone();
        let current_cwd = Arc::new(Mutex::new(cwd.to_string()));
        let cwd_for_reader = current_cwd.clone();
        let pending_commands = Arc::new(Mutex::new(HashMap::new()));
        let pending_commands_reader = pending_commands.clone();

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
                        let data = buf[..n].to_vec();
                        let mut should_display = true;

                        // Parse OSC 7 for CWD tracking: \e]7;file://host/path\e\\
                        if let Ok(s) = String::from_utf8(data.clone()) {
                            if let Some(start) = s.find("\x1b]7;file://") {
                                if let Some(end) = s[start..].find("\x1b\\") {
                                    let osc = &s[start + 12..start + end];
                                    if let Some(slash_pos) = osc.find('/') {
                                        let path = &osc[slash_pos..];
                                        if let Ok(mut cwd) = cwd_for_reader.lock() {
                                            *cwd = path.to_string();
                                            eprintln!("[grove-pty] CWD updated: {}", path);
                                        }
                                    }
                                }
                            }

                            // Check for command execution markers
                            if let Ok(mut cmds) = pending_commands_reader.lock() {
                                // Hide command lines that contain our wrapped command pattern
                                // e.g., "echo 'GROVE_EXEC_START_uuid' && git worktree list..."
                                if s.contains("echo 'GROVE_EXEC_START_") ||
                                   s.contains("echo \"GROVE_EXEC_START_") ||
                                   (s.contains("GROVE_EXEC_END_") && s.contains(" && echo")) {
                                    should_display = false;
                                    eprintln!("[grove-pty] Hiding command wrapper from display");
                                }

                                // Hide git command lines (but only the command itself, not all output)
                                if s.trim().starts_with("git ") && s.contains("2>/dev/null") {
                                    should_display = false;
                                    eprintln!("[grove-pty] Hiding git command line from display");
                                }

                                // Check for start markers
                                if s.contains("GROVE_EXEC_START_") {
                                    should_display = false;  // Hide marker line
                                    for line in s.lines() {
                                        if let Some(uuid_start) = line.find("GROVE_EXEC_START_") {
                                            // Extract UUID - take only valid UUID chars (alphanumeric and hyphens)
                                            let rest = &line[uuid_start + 17..];
                                            let uuid: String = rest.chars()
                                                .take_while(|c| c.is_alphanumeric() || *c == '-')
                                                .collect();
                                            if uuid.len() == 36 {  // Valid UUID length
                                                eprintln!("[grove-pty] Command execution started: {}", uuid);
                                                cmds.insert(uuid.clone(), CommandExecution {
                                                    output: String::new(),
                                                    complete: false,
                                                });
                                            }
                                        }
                                    }
                                }

                                // Accumulate output for active commands and check for end markers
                                let active_commands: Vec<String> = cmds.keys()
                                    .filter(|k| !cmds[*k].complete)
                                    .cloned()
                                    .collect();

                                for cmd_id in active_commands {
                                    if let Some(cmd) = cmds.get_mut(&cmd_id) {
                                        // Check if this chunk contains the end marker
                                        let end_marker = format!("GROVE_EXEC_END_{}", cmd_id);
                                        let start_marker = format!("GROVE_EXEC_START_{}", cmd_id);

                                        if s.contains(&end_marker) {
                                            // Extract everything before the end marker, but after start marker
                                            let output_chunk = if let Some(end_pos) = s.find(&end_marker) {
                                                let before_end = &s[..end_pos];
                                                // Also filter out the start marker if it's in this chunk
                                                if let Some(start_pos) = before_end.find(&start_marker) {
                                                    &before_end[start_pos + start_marker.len()..]
                                                } else {
                                                    before_end
                                                }
                                            } else {
                                                ""
                                            };

                                            cmd.output.push_str(output_chunk);
                                            cmd.complete = true;
                                            eprintln!("[grove-pty] Command execution complete: {} ({} bytes)", cmd_id, cmd.output.len());
                                        } else if !s.contains(&start_marker) {
                                            // Only accumulate output that doesn't contain start marker
                                            cmd.output.push_str(&s);
                                        }
                                    }
                                }

                                // Clean up completed commands after a delay to avoid accumulation
                                // Commands are kept briefly so execute_in_terminal can read them
                                let completed_commands: Vec<String> = cmds.keys()
                                    .filter(|k| cmds[*k].complete && cmds[*k].output.len() > 0)
                                    .cloned()
                                    .collect();

                                // For now, don't remove them immediately - execute_in_terminal needs to read them
                                // They'll be cleaned up by execute_in_terminal after reading
                            }
                        }

                        // Only send data to terminal if it should be displayed
                        if should_display {
                            on_data(data);
                        }
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
            current_cwd,
            pending_commands,
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

    pub fn get_cwd(&self) -> String {
        self.current_cwd.lock().unwrap().clone()
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        self.shutdown();
    }
}
