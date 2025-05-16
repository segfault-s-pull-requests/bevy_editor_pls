use std::io::{self, BufRead};
/// yes chatgpt wrote this shit.
use std::process::{Command, Stdio};
use std::path::{PathBuf};
use bevy::utils::tracing;
use tracing::Metadata;

/// Tries to open the file and line number from a `tracing::Metadata`
/// in the user's default editor (using `$EDITOR`, `code`, or `open`/`xdg-open`).
pub fn open_file_at_line(metadata: &Metadata) -> std::io::Result<()> {
    let file = metadata.file().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "No file info in metadata")
    })?;
    let line = metadata.line().unwrap_or(1);

    // Resolve the path — try to canonicalize, falling back if that fails
    let path = resolve_source_path(file).unwrap_or_else(|| PathBuf::from(file));

    open_in_editor(&path, line)
}

/// Resolves a source file path to an absolute one.
///
/// This handles:
/// - Workspace-local paths (like "src/lib.rs")
/// - Library crate paths (e.g. in `.cargo/registry/src/`)
fn resolve_source_path(file: &str) -> Option<PathBuf> {
    let path = PathBuf::from(file);

    // Already absolute
    if path.is_absolute() {
        return Some(path);
    }

    // Try canonicalizing relative to current project root
    if let Ok(p) = std::env::current_dir().and_then(|cwd| cwd.join(&path).canonicalize()) {
        return Some(p);
    }

    // Try finding it in the cargo registry cache
    if let Some(cargo_home) = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".cargo")))
    {
        let registry_src = cargo_home.join("registry").join("src");
        if let Ok(entries) = std::fs::read_dir(&registry_src) {
            for entry in entries.flatten() {
                let full = entry.path().join(&path);
                if full.exists() {
                    return Some(full);
                }
            }
        }
    }

    None
}

/// Attempts to open the given file at the specified line using the best available editor.
fn open_in_editor(path: &PathBuf, line: u32) -> std::io::Result<()> {
    // First try $EDITOR with +line if possible
    // disabled bc EDITOR is normally nongraphical ie vim
    // if let Ok(editor) = std::env::var("EDITOR") {
    //     // Assume `$EDITOR +line file`
    //     let status = Command::new(editor)
    //         .arg(format!("+{}", line))
    //         .arg(path)
    //         .status()?;

    //     if status.success() {
    //         return Ok(());
    //     }
    // }

    // VSCode (code) — prefers --goto
    if is_vscode_running().is_ok_and(|a| a)
        && Command::new("which")
            .arg("code")
            .output()
            .map_or(false, |o| o.status.success())
    {
        return Command::new("code")
            .arg("--goto")
            .arg(format!("{}:{}", path.display(), line))
            .status()
            .map(|_| ());
    }

    // Fallback: use xdg-open (Linux) or open (macOS)
    #[cfg(target_os = "macos")]
    let fallback = Command::new("open").arg(path).status();
    #[cfg(target_os = "linux")]
    let fallback = Command::new("xdg-open").arg(path).status();

    fallback.map(|_| ())
}

fn is_vscode_running() -> io::Result<bool> {
    // Use `ps aux` and look for "code" process (Linux/macOS)
    // This is a simple heuristic, might need tweaking per platform

    let ps = Command::new("ps")
        .arg("aux")
        .stdout(Stdio::piped())
        .spawn()?;

    let stdout = ps
        .stdout
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to get ps output"))?;
    let reader = io::BufReader::new(stdout);

    for line in reader.lines() {
        let line = line?;
        if line.contains("code") && !line.contains("code-helper") {
            // "code-helper" is a VSCode helper process, ignore it
            return Ok(true);
        }
    }

    Ok(false)
}
