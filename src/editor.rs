use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn configured_editor() -> String {
    env::var("NIFFLER_EDITOR")
        .or_else(|_| env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string())
}

pub fn edit_card(path: &Path) -> io::Result<()> {
    let original = fs::read_to_string(path)?;
    let temp_path = temp_card_path(path)?;
    fs::write(&temp_path, original)?;

    let status = run_editor(&configured_editor(), &temp_path)?;
    if status.success() {
        let edited = fs::read_to_string(&temp_path)?;
        fs::write(path, edited)?;
    }

    let _ = fs::remove_file(temp_path);
    Ok(())
}

fn run_editor(editor: &str, path: &Path) -> io::Result<std::process::ExitStatus> {
    let mut parts = editor.split_whitespace();
    let executable = parts.next().unwrap_or("vi");
    let mut command = Command::new(executable);
    command.args(parts).arg(path).status()
}

fn temp_card_path(path: &Path) -> io::Result<PathBuf> {
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let filename = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "card.md".to_string());
    Ok(env::temp_dir().join(format!("niffler-{id}-{filename}")))
}
