use std::process::Command;
use std::error::Error;

/// Recherche le premier résultat YouTube pour une requête `query`.
/// Retourne (url, titre) ou None en cas d'échec.
pub fn search_first_video(query: &str) -> Option<(String, String)> {
    if query.trim().is_empty() {
        return None;
    }

    let output = Command::new("yt-dlp")
        .arg(format!("ytsearch1:{}", query))
        .arg("--get-title")
        .arg("--get-id")
        .arg("--no-warnings")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let out = String::from_utf8_lossy(&output.stdout);
    let mut lines = out.lines();
    let title = lines.next()?.trim().to_string();
    let id = lines.next()?.trim().to_string();

    if title.is_empty() || id.is_empty() {
        return None;
    }

    let url = format!("https://www.youtube.com/watch?v={}", id);
    Some((url, title))
}

/// Télécharge l'audio d'une vidéo YouTube via yt-dlp.
/// Retourne le chemin du fichier MP3 téléchargé.
pub fn download_audio(link: &str, output_dir: &str) -> Result<String, Box<dyn Error>> {
    if link.trim().is_empty() {
        return Err("URL vide fournie".into());
    }
    if output_dir.trim().is_empty() {
        return Err("Répertoire de sortie vide".into());
    }

    std::fs::create_dir_all(output_dir)?;

    let output_template = format!("{}/%(title).100s.%(ext)s", output_dir);
    let files_before = count_mp3_files(output_dir)?;

    let status = Command::new("yt-dlp")
        .arg("-x")
        .arg("--audio-format")
        .arg("mp3")
        .arg("--audio-quality")
        .arg("0")
        .arg("-o")
        .arg(&output_template)
        .arg("--no-warnings")
        .arg("--restrict-filenames")
        .arg(link)
        .status()?;

    if !status.success() {
        return Err(format!("❌ yt-dlp a échoué lors du téléchargement (code: {:?})", status.code()).into());
    }

    let file = find_newest_mp3(output_dir, files_before)?;
    // Vérification du fichier
    if !std::path::Path::new(&file).exists() {
        return Err("❌ Fichier téléchargé introuvable après yt-dlp".into());
    }
    Ok(file)
}

fn count_mp3_files(dir: &str) -> Result<usize, Box<dyn Error>> {
    let count = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.path().extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("mp3"))
                .unwrap_or(false)
        })
        .count();
    Ok(count)
}

fn find_newest_mp3(dir: &str, files_before: usize) -> Result<String, Box<dyn Error>> {
    let mut mp3_files: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.path().extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("mp3"))
                .unwrap_or(false)
        })
        .collect();

    if mp3_files.len() <= files_before {
        return Err("Aucun nouveau fichier MP3 trouvé après téléchargement".into());
    }

    mp3_files.sort_by_key(|entry| {
        entry.metadata()
            .and_then(|m| m.modified())
            .unwrap_or_else(|_| std::time::SystemTime::UNIX_EPOCH)
    });

    if let Some(newest) = mp3_files.last() {
        Ok(newest.path().display().to_string())
    } else {
        Err("Impossible de trouver le fichier MP3 téléchargé".into())
    }
}

pub fn search_and_download(query: &str, output_dir: &str) -> Result<(String, String), Box<dyn Error>> {
    let (url, title) = search_first_video(query)
        .ok_or("Aucun résultat trouvé pour la recherche")?;


    let file_path = download_audio(&url, output_dir)?;


    Ok((file_path, title))
}

pub fn check_yt_dlp_available() -> bool {
    Command::new("yt-dlp")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
