
use std::process::{Command, Stdio};

pub fn play_audio(file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut child = Command::new("mpv")
        .arg("--no-video")
        .arg("--quiet")
        .arg("--no-terminal")
        .arg(file_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    Ok(())
}

