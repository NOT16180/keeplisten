use std::fmt;

#[derive(Debug)]
pub enum MusicPlayerError {
    Audio(AudioError),
    Youtube(YoutubeError),
    Playlist(PlaylistError),
    Io(std::io::Error),
    Network(String),
    Config(String),
}

#[derive(Debug)]
pub enum AudioError {
    PlaybackFailed(String),
    MpvNotFound,
    VolumeControlFailed,
    SeekFailed,
    ProcessTerminated,
}

#[derive(Debug)]
pub enum YoutubeError {
    YtDlpNotFound,
    SearchFailed(String),
    DownloadFailed(String),
    InvalidUrl(String),
    NoResults,
    ParseError(String),
}

#[derive(Debug)]
pub enum PlaylistError {
    PlaylistNotFound(String),
    TrackNotFound(String),
    SaveFailed(String),
    LoadFailed(String),
    InvalidFormat(String),
}

impl fmt::Display for MusicPlayerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MusicPlayerError::Audio(e) => write!(f, "Audio error: {}", e),
            MusicPlayerError::Youtube(e) => write!(f, "YouTube error: {}", e),
            MusicPlayerError::Playlist(e) => write!(f, "Playlist error: {}", e),
            MusicPlayerError::Io(e) => write!(f, "IO error: {}", e),
            MusicPlayerError::Network(e) => write!(f, "Network error: {}", e),
            MusicPlayerError::Config(e) => write!(f, "Configuration error: {}", e),
        }
    }
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AudioError::PlaybackFailed(msg) => write!(f, "Playback failed: {}", msg),
            AudioError::MpvNotFound => write!(f, "MPV player not found. Please install mpv."),
            AudioError::VolumeControlFailed => write!(f, "Failed to control volume"),
            AudioError::SeekFailed => write!(f, "Failed to seek in track"),
            AudioError::ProcessTerminated => write!(f, "Audio process was terminated"),
        }
    }
}

impl fmt::Display for YoutubeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            YoutubeError::YtDlpNotFound => write!(f, "yt-dlp not found. Please install yt-dlp."),
            YoutubeError::SearchFailed(query) => write!(f, "Search failed for: {}", query),
            YoutubeError::DownloadFailed(url) => write!(f, "Download failed for: {}", url),
            YoutubeError::InvalidUrl(url) => write!(f, "Invalid YouTube URL: {}", url),
            YoutubeError::NoResults => write!(f, "No search results found"),
            YoutubeError::ParseError(msg) => write!(f, "Failed to parse response: {}", msg),
        }
    }
}

impl fmt::Display for PlaylistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlaylistError::PlaylistNotFound(name) => write!(f, "Playlist not found: {}", name),
            PlaylistError::TrackNotFound(title) => write!(f, "Track not found: {}", title),
            PlaylistError::SaveFailed(msg) => write!(f, "Failed to save playlist: {}", msg),
            PlaylistError::LoadFailed(msg) => write!(f, "Failed to load playlist: {}", msg),
            PlaylistError::InvalidFormat(msg) => write!(f, "Invalid playlist format: {}", msg),
        }
    }
}

impl std::error::Error for MusicPlayerError {}
impl std::error::Error for AudioError {}
impl std::error::Error for YoutubeError {}
impl std::error::Error for PlaylistError {}

impl From<std::io::Error> for MusicPlayerError {
    fn from(error: std::io::Error) -> Self {
        MusicPlayerError::Io(error)
    }
}

impl From<AudioError> for MusicPlayerError {
    fn from(error: AudioError) -> Self {
        MusicPlayerError::Audio(error)
    }
}

impl From<YoutubeError> for MusicPlayerError {
    fn from(error: YoutubeError) -> Self {
        MusicPlayerError::Youtube(error)
    }
}

impl From<PlaylistError> for MusicPlayerError {
    fn from(error: PlaylistError) -> Self {
        MusicPlayerError::Playlist(error)
    }
}

pub type Result<T> = std::result::Result<T, MusicPlayerError>;

/// Helper function to create user-friendly error messages
pub fn user_friendly_error(error: &MusicPlayerError) -> String {
    match error {
        MusicPlayerError::Audio(AudioError::MpvNotFound) => {
            "🎵 Lecteur audio manquant. Installez MPV avec:\n• Ubuntu/Debian: sudo apt install mpv\n• macOS: brew install mpv\n• Windows: téléchargez depuis mpv.io".to_string()
        },
        MusicPlayerError::Youtube(YoutubeError::YtDlpNotFound) => {
            "📺 Téléchargeur YouTube manquant. Installez yt-dlp avec:\n• pip install yt-dlp\n• ou visitez github.com/yt-dlp/yt-dlp".to_string()
        },
        MusicPlayerError::Network(msg) => {
            format!("🌐 Problème de connexion: {}\nVérifiez votre connexion internet.", msg)
        },
        _ => format!("❌ {}", error)
    }
}
