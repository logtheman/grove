# Grove Debug Handoff

## What is this

Grove is a Tauri v2 desktop app (Rust backend + React frontend) that embeds terminal emulators via xterm.js + portable-pty. It's a worktree-aware terminal manager.

## Current Problem

The terminal renders in the UI but **nothing displays and typing doesn't work**. The PTY backend is confirmed working -- shell spawns, data flows through the reader thread. The bug is on the frontend: Tauri events aren't reaching xterm.js, or xterm.js isn't rendering.

## What's Been Verified (Rust side works)

`cargo tauri dev` stderr shows the full PTY pipeline working:

```
[grove-cmd] create_terminal: id=..., cwd=/Users/loganmurdock
[grove-pty] spawn: cwd=/Users/loganmurdock, rows=24, cols=80
[grove-pty] openpty succeeded
[grove-pty] got reader
[grove-pty] got writer
[grove-pty] using shell: /bin/zsh
[grove-pty] spawn_command succeeded
[grove-pty] slave dropped
[grove-pty] reader thread started
[grove-pty] reader thread: read 136 bytes
[grove-pty] reader thread: read 42 bytes
... (many reads - shell prompt output)
```

The shell spawns, outputs its prompt, and the reader thread delivers data to the `on_data` callback which calls `app_handle.emit()`. No emit errors are logged.

## What Was Just Fixed (may or may not resolve it)

The most recent commit (`15afe05`) fixed a critical bug where `onExit` was in the useEffect dependency array but was a new arrow function on every render, causing the terminal to be **disposed and recreated on every App re-render**. This has been fixed (onExit stored in a ref). Also fixed CSS positioning and visibility.

## Your Job

1. Run `cargo tauri dev`
2. Open browser devtools in the Tauri window (right-click -> Inspect Element, or Cmd+Option+I)
3. Check the browser console for `[grove-term]` and `[grove-ts]` log lines
4. Determine where the pipeline breaks:

### Expected console output if working

```
[grove-ts] createTerminal: cwd= undefined
[grove-ts] createTerminal result: {id: "...", cwd: "/Users/..."}
[grove-term] useEffect: container= true initialized= false terminalId= ...
[grove-term] initializing terminal: ...
[grove-term] xterm opened in DOM
[grove-term] fitAddon.fit() done, rows= 24 cols= 80
[grove-ts] subscribing to terminal-data: ...
[grove-ts] terminal-data received: ... bytes: 136
[grove-term] writing 136 bytes to xterm, terminalRef= true
```

When typing:
```
[grove-term] onData from xterm: "a"
[grove-ts] writeToTerminal: ... bytes: 1
```

### Likely failure points to check

| Symptom | Cause | Fix |
|---------|-------|-----|
| No `[grove-term]` logs at all | useEffect not firing, component not mounting | Check React rendering, check if tab is created |
| `useEffect` fires but `container= false` | DOM ref not attached | Check TerminalTab render, containerRef |
| xterm opens but no `terminal-data received` | Tauri event listener not connecting, or event name mismatch | Check event names match between Rust and TS |
| Data received but nothing visible | xterm container has zero dimensions | Inspect element, check computed height/width |
| No `onData from xterm` when typing | xterm not focused, or container intercepting events | Check focus, check z-index, check pointer-events |

## Key Files

| File | Role |
|------|------|
| `src-tauri/src/pty/manager.rs` | PTY spawn, read thread, write |
| `src-tauri/src/ipc/commands.rs` | Tauri commands, emit events to frontend |
| `src-tauri/src/ipc/events.rs` | Event name constants (`terminal-data`, `terminal-exit`) |
| `src/hooks/useTerminal.ts` | xterm.js lifecycle, Tauri event subscription |
| `src/components/terminal/TerminalTab.tsx` | Component wrapper, visibility/focus |
| `src/services/tauri.ts` | Typed wrappers around invoke/listen |
| `src/App.tsx` | Creates terminals, manages tabs |
| `src/styles/globals.css` | xterm-container CSS |

## How to Run

```bash
cargo tauri dev
```

This starts both the Rust backend and the Vite dev server with hot reload. Rust `eprintln!` goes to stderr (visible in the terminal). Frontend `console.log` goes to the webview devtools console.

## Debug Logging Already In Place

All debug logging uses `[grove-*]` prefixes:
- `[grove-pty]` - Rust PTY operations (stderr)
- `[grove-cmd]` - Rust Tauri commands (stderr)
- `[grove-term]` - Frontend useTerminal hook (browser console)
- `[grove-ts]` - Frontend tauri service layer (browser console)

## If You Need to Make Changes

The event flow is: **PTY reader thread** -> `on_data` callback -> `app_handle.emit("terminal-data:ID", data)` -> frontend `listen("terminal-data:ID")` -> `terminal.write(bytes)` -> xterm renders.

Input flow: **xterm.onData** -> `writeToTerminal(id, encoded)` -> Rust `write_to_terminal` command -> `session.write(data)` -> PTY master writer -> shell receives input -> shell echoes -> reader thread picks up echo -> back to output flow.
