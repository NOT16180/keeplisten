use std::process::{Command, Stdio};
use std::path::Path;

pub fn download_audio(link: &str, output_dir: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Crée le dossier s'il n'existe pas
    std::fs::create_dir_all(output_dir)?;

    // yt-dlp arguments
    let output_template = format!("{}/%(title)s.%(ext)s", output_dir);
    let status = Command::new("yt-dlp")
        .arg("-x") // extract audio
        .arg("--audio-format")
        .arg("mp3") // convert to mp3
        .arg("-o")
        .arg(&output_template)
        .arg(link)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if status.success() {
        // Liste les fichiers mp3 créés dans le dossier
        let mut mp3s: Vec<_> = std::fs::read_dir(output_dir)?
            .filter_map(Result::ok)
            .filter(|entry| {
                entry.path().extension().map(|ext| ext == "mp3").unwrap_or(false)
            })
            .collect();

        // Prend le fichier le plus récent (le dernier téléchargé)
        mp3s.sort_by_key(|entry| entry.metadata().and_then(|m| m.modified()).ok());
        if let Some(last) = mp3s.last() {
            let path = last.path();
            println!("✅ Téléchargement terminé : {}", path.display());
            Ok(path.display().to_string())
        } else {
            Err("Aucun fichier mp3 trouvé après téléchargement".into())
        }
    } else {
        Err("❌ yt-dlp a échoué".into())
    }
}
