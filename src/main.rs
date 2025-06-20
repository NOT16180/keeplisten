

mod youtube;
mod audio;

use std::io;
use std::fs;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let url = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
    let music_dir = "music";
    let audio_path = format!("{}/ton_fichier.mp3", music_dir);

    // Crée le dossier si besoin
    fs::create_dir_all(music_dir)?;

    // Télécharge seulement si le fichier n'existe pas
    if !std::path::Path::new(&audio_path).exists() {
        match youtube::download_audio(url, music_dir) {
            Ok(_) => (),
            Err(e) => eprintln!("Erreur téléchargement audio : {}", e),
        }
    }

    let res = run_app(&mut terminal, &audio_path);

    // Nettoyage terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Erreur app : {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    audio_path: &str,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| {
            let size = f.size();
            let block = Block::default()
                .title("🎵 Mon Lecteur TUI (q: quitter, p: play)")
                .borders(Borders::ALL);
            f.render_widget(block, size);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('p') => {
                        if let Err(e) = audio::play_audio(audio_path) {
                            eprintln!("Erreur lecture audio : {}", e);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
