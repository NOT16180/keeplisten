use std::process::Command;
use std::error::Error;
use std::time::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    pub id: String,
    pub title: String,
    pub duration: Option<Duration>,
    pub uploader: Option<String>,
    pub view_count: Option<u64>,
}

/// Enhanced search that returns multiple results with metadata
pub fn search_videos(query: &str, max_results: usize) -> Option<Vec<VideoInfo>> {
    if query.trim().is_empty() {
        return None;
    }

    let search_query = format!("ytsearch{}:{}", max_results, query);
    let output = Command::new("yt-dlp")
        .arg(&search_query)
        .arg("--dump-json")
        .arg("--no-warnings")
        .arg("--no-playlist")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut videos = Vec::new();

    for line in output_str.lines() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            let id = json["id"].as_str()?.to_string();
            let title = json["title"].as_str()?.to_string();
            let duration = json["duration"].as_f64()
                .map(|d| Duration::from_secs_f64(d));
            let uploader = json["uploader"].as_str().map(|s| s.to_string());
            let view_count = json["view_count"].as_u64();

            videos.push(VideoInfo {
                id,
                title,
                duration,
                uploader,
                view_count,
            });
        }
    }

    if videos.is_empty() {
        None
    } else {
        Some(videos)
    }
}

/// Backward compatibility: search first video
pub fn search_first_video(query: &str) -> Option<(String, String)> {
    let results = search_videos(query, 1)?;
    let video = results.into_iter().next()?;
    let url = format!("https://www.youtube.com/watch?v={}", video.id);
    Some((url, video.title))
}

/// Enhanced download with progress callback
pub fn download_audio_with_progress<F>(
    link: &str, 
    output_dir: &str,
    progress_callback: Option<F>
) -> Result<String, Box<dyn Error>> 
where 
    F: Fn(f32) + Send + 'static,
{
    if link.trim().is_empty() {
        return Err("URL vide fournie".into());
    }
    if output_dir.trim().is_empty() {
        return Err("Répertoire de sortie vide".into());
    }

    std::fs::create_dir_all(output_dir)?;

    let output_template = format!("{}/%(title).100s.%(ext)s", output_dir);
    let files_before = count_mp3_files(output_dir)?;

    let mut cmd = Command::new("yt-dlp");
    cmd.arg("-x")
        .arg("--audio-format")
        .arg("mp3")
        .arg("--audio-quality")
        .arg("0")
        .arg("-o")
        .arg(&output_template)
        .arg("--no-warnings")
        .arg("--restrict-filenames");

    // Add progress hooks if callback is provided
    if progress_callback.is_some() {
        cmd.arg("--newline");
    }

    cmd.arg(link);

    let status = if let Some(callback) = progress_callback {
        // Run with progress monitoring
        let output = cmd.output()?;
        
        // Parse progress from stderr (yt-dlp outputs progress there)
        let stderr = String::from_utf8_lossy(&output.stderr);
        for line in stderr.lines() {
            if line.contains('%') {
                if let Some(percent_str) = extract_percentage(line) {
                    if let Ok(percent) = percent_str.parse::<f32>() {
                        callback(percent / 100.0);
                    }
                }
            }
        }
        
        output.status
    } else {
        cmd.status()?
    };

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

/// Original download function for backward compatibility
pub fn download_audio(link: &str, output_dir: &str) -> Result<String, Box<dyn Error>> {
    download_audio_with_progress(link, output_dir, None::<fn(f32)>)
}

fn extract_percentage(line: &str) -> Option<String> {
    // Look for patterns like "[download] 45.2% of 3.45MiB at 1.23MiB/s ETA 00:02"
    if let Some(start) = line.find("] ") {
        let after_bracket = &line[start + 2..];
        if let Some(percent_pos) = after_bracket.find('%') {
            let before_percent = &after_bracket[..percent_pos];
            if let Some(space_pos) = before_percent.rfind(' ') {
                return Some(before_percent[space_pos + 1..].to_string());
            } else {
                return Some(before_percent.to_string());
            }
        }
    }
    None
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

pub fn search_and_download_with_progress<F>(
    query: &str, 
    output_dir: &str,
    progress_callback: F
) -> Result<(String, String), Box<dyn Error>> 
where 
    F: Fn(f32) + Send + 'static,
{
    let (url, title) = search_first_video(query)
        .ok_or("Aucun résultat trouvé pour la recherche")?;

    let file_path = download_audio_with_progress(&url, output_dir, Some(progress_callback))?;

    Ok((file_path, title))
}

/// Get video info without downloading
pub fn get_video_info(url: &str) -> Result<VideoInfo, Box<dyn Error>> {
    let output = Command::new("yt-dlp")
        .arg(url)
        .arg("--dump-json")
        .arg("--no-warnings")
        .output()?;

    if !output.status.success() {
        return Err("Failed to get video info".into());
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&output_str)?;

    let id = json["id"].as_str()
        .ok_or("No video ID found")?
        .to_string();
    let title = json["title"].as_str()
        .ok_or("No video title found")?
        .to_string();
    let duration = json["duration"].as_f64()
        .map(|d| Duration::from_secs_f64(d));
    let uploader = json["uploader"].as_str().map(|s| s.to_string());
    let view_count = json["view_count"].as_u64();

    Ok(VideoInfo {
        id,
        title,
        duration,
        uploader,
        view_count,
    })
}

pub fn check_yt_dlp_available() -> bool {
    Command::new("yt-dlp")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Check if URL is a valid YouTube URL
pub fn is_youtube_url(url: &str) -> bool {
    url.contains("youtube.com/watch") || 
    url.contains("youtu.be/") || 
    url.contains("youtube.com/playlist") ||
    url.contains("music.youtube.com")
}
