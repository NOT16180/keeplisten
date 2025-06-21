use std::process::{Command, Stdio, Child};
use std::sync::{Mutex, Arc};
use std::thread;
use std::time::{Duration, Instant};
use lazy_static::lazy_static;

lazy_static! {
    static ref AUDIO_CHILD: Mutex<Option<Child>> = Mutex::new(None);
    static ref PLAYBACK_STATE: Mutex<PlaybackState> = Mutex::new(PlaybackState::default());
}

#[derive(Debug, Clone, Default)]
pub struct PlaybackState {
    pub is_playing: bool,
    pub is_paused: bool,
    pub position: Duration,
    pub duration: Option<Duration>,
    pub volume: u8,
    pub last_update: Option<Instant>,
}

pub fn play_audio(file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Stop any previous playback
    stop_audio();

    let child = Command::new("mpv")
        .arg("--no-video")
        .arg("--quiet")
        .arg("--no-terminal")
        .arg("--input-ipc-server=/tmp/mpvsocket") // Enable IPC for better control
        .arg("--idle=yes")
        .arg(file_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let mut audio_child = AUDIO_CHILD.lock().unwrap();
    *audio_child = Some(child);

    // Update playback state
    let mut state = PLAYBACK_STATE.lock().unwrap();
    state.is_playing = true;
    state.is_paused = false;
    state.position = Duration::from_secs(0);
    state.last_update = Some(Instant::now());

    Ok(())
}

#[cfg(target_family = "unix")]
pub fn pause_audio() -> Result<(), Box<dyn std::error::Error>> {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;
    
    let audio_child = AUDIO_CHILD.lock().unwrap();
    if let Some(child) = audio_child.as_ref() {
        kill(Pid::from_raw(child.id() as i32), Signal::SIGSTOP)?;
        
        let mut state = PLAYBACK_STATE.lock().unwrap();
        state.is_paused = true;
    }
    Ok(())
}

#[cfg(target_family = "unix")]
pub fn resume_audio() -> Result<(), Box<dyn std::error::Error>> {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;
    
    let audio_child = AUDIO_CHILD.lock().unwrap();
    if let Some(child) = audio_child.as_ref() {
        kill(Pid::from_raw(child.id() as i32), Signal::SIGCONT)?;
        
        let mut state = PLAYBACK_STATE.lock().unwrap();
        state.is_paused = false;
        state.last_update = Some(Instant::now());
    }
    Ok(())
}

#[cfg(target_family = "windows")]
pub fn pause_audio() -> Result<(), Box<dyn std::error::Error>> {
    // Windows implementation would use different approach
    // For now, we'll use a placeholder
    let mut state = PLAYBACK_STATE.lock().unwrap();
    state.is_paused = true;
    Ok(())
}

#[cfg(target_family = "windows")]
pub fn resume_audio() -> Result<(), Box<dyn std::error::Error>> {
    let mut state = PLAYBACK_STATE.lock().unwrap();
    state.is_paused = false;
    state.last_update = Some(Instant::now());
    Ok(())
}

pub fn stop_audio() {
    let mut audio_child = AUDIO_CHILD.lock().unwrap();
    if let Some(child) = audio_child.as_mut() {
        let _ = child.kill();
        let _ = child.wait();
    }
    *audio_child = None;

    // Reset playback state
    let mut state = PLAYBACK_STATE.lock().unwrap();
    *state = PlaybackState::default();
}

pub fn set_volume(volume: u8) -> Result<(), Box<dyn std::error::Error>> {
    // Using mpv IPC to set volume (requires --input-ipc-server)
    use std::os::unix::net::UnixStream;
    use std::io::Write;
    
    if let Ok(mut stream) = UnixStream::connect("/tmp/mpvsocket") {
        let command = format!("{{ \"command\": [\"set_property\", \"volume\", {}] }}\n", volume);
        let _ = stream.write_all(command.as_bytes());
    }
    
    let mut state = PLAYBACK_STATE.lock().unwrap();
    state.volume = volume;
    Ok(())
}

pub fn seek_to(position: Duration) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::net::UnixStream;
    use std::io::Write;
    
    if let Ok(mut stream) = UnixStream::connect("/tmp/mpvsocket") {
        let seconds = position.as_secs_f64();
        let command = format!("{{ \"command\": [\"seek\", {}, \"absolute\"] }}\n", seconds);
        let _ = stream.write_all(command.as_bytes());
    }
    
    let mut state = PLAYBACK_STATE.lock().unwrap();
    state.position = position;
    Ok(())
}

pub fn get_playback_state() -> PlaybackState {
    PLAYBACK_STATE.lock().unwrap().clone()
}

pub fn update_position() {
    let mut state = PLAYBACK_STATE.lock().unwrap();
    if state.is_playing && !state.is_paused {
        if let Some(last_update) = state.last_update {
            let elapsed = last_update.elapsed();
            state.position += elapsed;
            state.last_update = Some(Instant::now());
        }
    }
}

pub fn is_process_running() -> bool {
    let audio_child = AUDIO_CHILD.lock().unwrap();
    if let Some(child) = audio_child.as_ref() {
        // Check if process is still running
        match child.try_wait() {
            Ok(Some(_)) => false, // Process has finished
            Ok(None) => true,     // Process is still running
            Err(_) => false,      // Error checking process
        }
    } else {
        false
    }
}
