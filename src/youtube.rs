use std::process::Command;
use std::error::Error;
use std::path::Path;

/// Recherche le premier résultat YouTube pour une requête `query`.
/// Retourne (url, titre) ou None en cas d'échec.
pub fn search_first_video(query: &str) -> Option<(String, String)> {
    if query.trim().is_empty() {
        return None;
    }

    let output = Command::new("yt-dlp")
        .arg(format!("ytsearch1:{}", query))
        .arg("--get-id")
        .arg("--get-title")
        .arg("--no-warnings")
        .output()
        .ok()?;

    if !output.status.success() {
        eprintln!("yt-dlp search failed: {}", String::from_utf8_lossy(&output.stderr));
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
    // Validation des entrées
    if link.trim().is_empty() {
        return Err("URL vide fournie".into());
    }
    
    if output_dir.trim().is_empty() {
        return Err("Répertoire de sortie vide".into());
    }

    // Créer le répertoire de sortie
    std::fs::create_dir_all(output_dir)?;
    
    // Template de sortie sécurisé (évite les caractères problématiques)
    let output_template = format!("{}/%(title).100s.%(ext)s", output_dir);
    
    // Compter les fichiers MP3 existants avant téléchargement
    let files_before = count_mp3_files(output_dir)?;
    
    let status = Command::new("yt-dlp")
        .arg("-x")
        .arg("--audio-format")
        .arg("mp3")
        .arg("--audio-quality")
        .arg("0") // Meilleure qualité
        .arg("-o")
        .arg(&output_template)
        .arg("--no-warnings")
        .arg("--restrict-filenames") // Évite les caractères spéciaux
        .arg(link)
        .status()?;

    if !status.success() {
        return Err("❌ yt-dlp a échoué lors du téléchargement".into());
    }

    // Trouver le nouveau fichier téléchargé
    find_newest_mp3(output_dir, files_before)
}

/// Compte le nombre de fichiers MP3 dans un répertoire
fn count_mp3_files(dir: &str) -> Result<usize, Box<dyn Error>> {
    let count = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.path().extension()
                .map(|ext| ext.to_ascii_lowercase() == "mp3")
                .unwrap_or(false)
        })
        .count();
    Ok(count)
}

/// Trouve le fichier MP3 le plus récent dans un répertoire
fn find_newest_mp3(dir: &str, files_before: usize) -> Result<String, Box<dyn Error>> {
    let mut mp3_files: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.path().extension()
                .map(|ext| ext.to_ascii_lowercase() == "mp3")
                .unwrap_or(false)
        })
        .collect();

    if mp3_files.len() <= files_before {
        return Err("Aucun nouveau fichier MP3 trouvé après téléchargement".into());
    }

    // Trier par date de modification (le plus récent en dernier)
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

/// Fonction utilitaire pour télécharger directement depuis une requête de recherche
pub fn search_and_download(query: &str, output_dir: &str) -> Result<(String, String), Box<dyn Error>> {
    let (url, title) = search_first_video(query)
        .ok_or("Aucun résultat trouvé pour la recherche")?;
    
    println!("🔍 Trouvé: {}", title);
    println!("📥 Téléchargement depuis: {}", url);
    
    let file_path = download_audio(&url, output_dir)?;
    
    println!("✅ Téléchargé: {}", file_path);
    
    Ok((file_path, title))
}

/// Vérifie si yt-dlp est disponible sur le système
pub fn check_yt_dlp_available() -> bool {
    Command::new("yt-dlp")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_search_empty_query() {
        assert!(search_first_video("").is_none());
        assert!(search_first_video("   ").is_none());
    }

    #[test]
    fn test_download_invalid_inputs() {
        let temp_dir = env::temp_dir().join("test_youtube_dl");
        let temp_path = temp_dir.to_str().unwrap();
        
        assert!(download_audio("", temp_path).is_err());
        assert!(download_audio("https://youtube.com/watch?v=invalid", "").is_err());
    }

    #[test]
    fn test_yt_dlp_check() {
        // Ce test pourrait échouer si yt-dlp n'est pas installé
        let available = check_yt_dlp_available();
        println!("yt-dlp disponible: {}", available);
    }
}
