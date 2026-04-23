use std::process::Command;
use std::sync::OnceLock;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Escape a string for use in a bash -c command
#[cfg(windows)]
fn shell_escape(s: &str) -> String {
    // Wrap in single quotes, escaping any embedded single quotes
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Build a WSL command with hidden console window (Windows only)
#[cfg(windows)]
fn wsl_command() -> Command {
    let mut cmd = Command::new("wsl");
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// Public accessor for the resolved tmux binary path
pub fn tmux_bin_pub() -> &'static str {
    tmux_bin()
}

/// Resolve tmux binary path — macOS GUI apps have stripped PATH
fn tmux_bin() -> &'static str {
    static BIN: OnceLock<String> = OnceLock::new();
    BIN.get_or_init(|| {
        #[cfg(windows)]
        {
            // On Windows, tmux lives inside WSL — check via `wsl -- bash -c 'which tmux'`
            if let Ok(output) = wsl_command().args(["--", "bash", "-c", "which tmux"]).output() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return path;
                }
            }
            return "tmux".to_string();
        }
        #[cfg(not(windows))]
        {
            // Try common locations first (macOS GUI apps don't have full PATH)
            let candidates = [
                "/opt/homebrew/bin/tmux",  // Apple Silicon homebrew
                "/usr/local/bin/tmux",     // Intel homebrew
                "/usr/bin/tmux",           // System
            ];
            for path in &candidates {
                if std::path::Path::new(path).exists() {
                    return path.to_string();
                }
            }
            // Fallback: try shell resolution
            if let Ok(output) = Command::new("/bin/sh").args(["-c", "which tmux"]).output() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return path;
                }
            }
            "tmux".to_string()
        }
    })
}

/// Run a tmux command and return stdout
fn tmux(args: &[&str]) -> Result<String, String> {
    #[cfg(windows)]
    let output = {
        // On Windows, run tmux through WSL via bash -c to prevent
        // argument mangling (WSL eats #{} format strings otherwise)
        let mut parts = vec![tmux_bin().to_string()];
        parts.extend(args.iter().map(|a| shell_escape(a)));
        let bash_cmd = parts.join(" ");
        wsl_command()
            .args(["--", "bash", "-c", &bash_cmd])
            .output()
            .map_err(|e| format!("wsl tmux exec: {}", e))?
    };
    #[cfg(not(windows))]
    let output = {
        Command::new(tmux_bin())
            .args(args)
            .output()
            .map_err(|e| format!("tmux exec: {}", e))?
    };

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("tmux: {}", stderr))
    }
}

/// Check if tmux server is running
pub fn is_running() -> bool {
    tmux(&["info"]).is_ok()
}

/// Check if a session exists
pub fn session_exists(name: &str) -> bool {
    tmux(&["has-session", "-t", name]).is_ok()
}

/// Create a new detached session with a working directory
pub fn create_session(name: &str, cwd: &str) -> Result<(), String> {
    if session_exists(name) {
        return Ok(());
    }
    tmux(&["new-session", "-d", "-s", name, "-c", cwd])?;
    Ok(())
}

/// Create a named window inside a session
pub fn create_window(session: &str, window: &str, cwd: &str) -> Result<(), String> {
    tmux(&["new-window", "-t", session, "-n", window, "-c", cwd])?;
    Ok(())
}

/// Send text to a session:window target, then press Enter.
/// Split into two tmux calls because Claude Code's paste bracketing
/// treats text+Enter in a single call as one paste event (Enter stays literal).
pub fn send_keys(target: &str, text: &str) -> Result<(), String> {
    tmux(&["send-keys", "-t", target, text])?;
    std::thread::sleep(std::time::Duration::from_millis(120));
    tmux(&["send-keys", "-t", target, "Enter"])?;
    Ok(())
}

/// Send text without pressing Enter
pub fn send_keys_raw(target: &str, text: &str) -> Result<(), String> {
    tmux(&["send-keys", "-t", target, text])?;
    Ok(())
}

/// Capture pane content (last N lines)
pub fn capture_pane(target: &str, lines: u32) -> Result<String, String> {
    let start = format!("-{}", lines);
    tmux(&["capture-pane", "-t", target, "-p", "-S", &start])
}

const SEP: &str = "|||";

/// List all sessions
pub fn list_sessions() -> Result<Vec<SessionInfo>, String> {
    let fmt = format!("#{{session_name}}{}#{{session_created}}{}#{{session_windows}}", SEP, SEP);
    let output = tmux(&["list-sessions", "-F", &fmt])?;
    Ok(output
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(SEP).collect();
            if parts.is_empty() { return None; }
            Some(SessionInfo {
                name: parts[0].to_string(),
                created: parts.get(1).unwrap_or(&"").to_string(),
                windows: parts.get(2).and_then(|w| w.parse().ok()).unwrap_or(0),
            })
        })
        .collect())
}

/// List windows in a session
pub fn list_windows(session: &str) -> Result<Vec<WindowInfo>, String> {
    let fmt = format!("#{{window_index}}{}#{{window_name}}{}#{{window_active}}", SEP, SEP);
    let output = tmux(&["list-windows", "-t", session, "-F", &fmt])?;
    Ok(output
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(SEP).collect();
            if parts.len() < 3 { return None; }
            Some(WindowInfo {
                index: parts[0].parse().unwrap_or(0),
                name: parts[1].to_string(),
                active: parts[2] == "1",
            })
        })
        .collect())
}

/// Kill a specific session
pub fn kill_session(name: &str) -> Result<(), String> {
    tmux(&["kill-session", "-t", name])?;
    Ok(())
}

/// Kill a specific window
pub fn kill_window(target: &str) -> Result<(), String> {
    tmux(&["kill-window", "-t", target])?;
    Ok(())
}

/// Rename a window
pub fn rename_window(target: &str, new_name: &str) -> Result<(), String> {
    tmux(&["rename-window", "-t", target, new_name])?;
    Ok(())
}

/// Check if Claude Code is ready in a session (past startup, at the prompt)
pub fn is_claude_ready(target: &str) -> bool {
    let Ok(content) = capture_pane(target, 15) else { return false };
    // Claude Code shows the ">" input prompt when ready for input
    // Also check it's not stuck on the trust dialog
    let at_prompt = content.contains("\u{276f}") || content.lines().any(|l| l.trim().starts_with('>'));
    let has_claude = content.contains("Claude Code");
    let at_trust = content.contains("I trust this folder");
    (at_prompt && has_claude) && !at_trust
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionInfo {
    pub name: String,
    pub created: String,
    pub windows: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WindowInfo {
    pub index: u32,
    pub name: String,
    pub active: bool,
}
