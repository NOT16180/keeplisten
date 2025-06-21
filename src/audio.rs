use std::process::{Command, Stdio, Child};
use std::sync::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref AUDIO_CHILD: Mutex<Option<Child>> = Mutex::new(None);
}

pub fn play_audio(file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Stop any previous playback
    stop_audio();

    let mut child = Command::new("mpv")
        .arg("--no-video")
        .arg("--quiet")
        .arg("--no-terminal")
        .arg(file_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let mut audio_child = AUDIO_CHILD.lock().unwrap();
    *audio_child = Some(child);

    Ok(())
}

#[cfg(target_family = "unix")]
pub fn pause_audio() {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;
    let audio_child = AUDIO_CHILD.lock().unwrap();
    if let Some(child) = audio_child.as_ref() {
        let _ = kill(Pid::from_raw(child.id() as i32), Signal::SIGSTOP);
    }
}

#[cfg(target_family = "unix")]
pub fn resume_audio() {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;
    let audio_child = AUDIO_CHILD.lock().unwrap();
    if let Some(child) = audio_child.as_ref() {
        let _ = kill(Pid::from_raw(child.id() as i32), Signal::SIGCONT);
    }
}

pub fn stop_audio() {
    let mut audio_child = AUDIO_CHILD.lock().unwrap();
    if let Some(child) = audio_child.as_mut() {
        let _ = child.kill();
        let _ = child.wait();
    }
    *audio_child = None;
}
