use std::process::{Command, Stdio};
use std::path::Path;

pub fn download_audio(link: &str, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Crée le dossier s'il n'existe pas
    std::fs::create_dir_all(output_dir)?;

    // yt-dlp arguments
    let status = Command::new("yt-dlp")
        .arg("-x") // extract audio
        .arg("--audio-format")
        .arg("mp3") // convert to mp3
        .arg("-o")
        .arg(format!("{}/%(title)s.%(ext)s", output_dir))
        .arg(link)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if status.success() {
        println!("✅ Téléchargement terminé !");
        Ok(())
    } else {
        Err("❌ yt-dlp a échoué".into())
    }
}

