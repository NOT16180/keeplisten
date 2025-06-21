use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Track {
    pub title: String,
    pub file_path: String,
    pub url: Option<String>,
    pub duration: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Playlist {
    pub name: String,
    pub tracks: Vec<Track>,
}

impl Playlist {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tracks: Vec::new(),
        }
    }

    pub fn add_track(&mut self, track: Track) {
        self.tracks.push(track);
    }

    pub fn remove_track_by_index(&mut self, index: usize) -> Option<Track> {
        if index < self.tracks.len() {
            Some(self.tracks.remove(index))
        } else {
            None
        }
    }

    pub fn remove_track_by_title(&mut self, title: &str) -> Option<Track> {
        if let Some(pos) = self.tracks.iter().position(|t| t.title == title) {
            Some(self.tracks.remove(pos))
        } else {
            None
        }
    }

    pub fn list_tracks(&self) {
        for (i, track) in self.tracks.iter().enumerate() {
            println!(
                "{}. {} ({})",
                i + 1,
                track.title,
                track.file_path
            );
        }
    }
}

#[derive(Debug, Default)]
pub struct PlaylistManager {
    pub playlists: HashMap<String, Playlist>,
}

impl PlaylistManager {
    pub fn new() -> Self {
        Self {
            playlists: HashMap::new(),
        }
    }

    pub fn create_playlist(&mut self, name: &str) -> bool {
        if self.playlists.contains_key(name) {
            false
        } else {
            self.playlists.insert(name.to_string(), Playlist::new(name));
            true
        }
    }

    pub fn delete_playlist(&mut self, name: &str) -> bool {
        self.playlists.remove(name).is_some()
    }

    pub fn add_track_to_playlist(&mut self, playlist_name: &str, track: Track) -> bool {
        if let Some(pl) = self.playlists.get_mut(playlist_name) {
            pl.add_track(track);
            true
        } else {
            false
        }
    }

    pub fn remove_track_from_playlist_by_title(
        &mut self,
        playlist_name: &str,
        track_title: &str,
    ) -> bool {
        if let Some(pl) = self.playlists.get_mut(playlist_name) {
            pl.remove_track_by_title(track_title).is_some()
        } else {
            false
        }
    }

    pub fn remove_track_from_playlist_by_index(
        &mut self,
        playlist_name: &str,
        index: usize,
    ) -> bool {
        if let Some(pl) = self.playlists.get_mut(playlist_name) {
            pl.remove_track_by_index(index).is_some()
        } else {
            false
        }
    }

    pub fn list_playlists(&self) {
        if self.playlists.is_empty() {
            println!("Aucune playlist existante.");
        } else {
            for name in self.playlists.keys() {
                println!("- {}", name);
            }
        }
    }

    pub fn list_tracks_in_playlist(&self, playlist_name: &str) {
        if let Some(pl) = self.playlists.get(playlist_name) {
            pl.list_tracks();
        } else {
            println!("Playlist '{}' introuvable.", playlist_name);
        }
    }

    /// Sauvegarde toutes les playlists dans un dossier sous forme de fichiers .m3u simples.
    pub fn save_all_to_dir(&self, dir: &str) -> io::Result<()> {
        fs::create_dir_all(dir)?;
        for (name, playlist) in &self.playlists {
            let filename = format!("{}/{}.m3u", dir, name);
            let mut file = fs::File::create(&filename)?;
            for track in &playlist.tracks {
                writeln!(file, "{}", track.file_path)?;
            }
        }
        Ok(())
    }

    /// Charge toutes les playlists depuis un dossier (format .m3u simple).
    pub fn load_all_from_dir(&mut self, dir: &str) -> io::Result<()> {
        self.playlists.clear();
        if Path::new(dir).is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("m3u") {
                    let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("playlist");
                    let lines = fs::read_to_string(&path)?;
                    let mut playlist = Playlist::new(name);
                    for line in lines.lines() {
                        // On ne connaît pas le titre, juste le chemin
                        let file_path = line.trim();
                        if !file_path.is_empty() {
                            let title = Path::new(file_path)
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or(file_path)
                                .to_string();
                            playlist.add_track(Track {
                                title,
                                file_path: file_path.to_string(),
                                url: None,
                                duration: None,
                            });
                        }
                    }
                    self.playlists.insert(name.to_string(), playlist);
                }
            }
        }
        Ok(())
    }
}
