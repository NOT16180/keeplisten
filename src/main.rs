

mod youtube;
mod audio;

use std::io;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders},
    Terminal,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Démarrage TUI
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Par exemple, tu peux télécharger la musique avant la boucle
    youtube::download_audio("https://www.youtube.com/watch?v=dQw4w9WgXcQ", "music")?;

    let res = run_app(&mut terminal);

    // Nettoyage terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
    loop {
        terminal.draw(|f| {
            let size = f.size();
            let block = Block::default()
                .title("🎵 Mon Lecteur TUI")
                .borders(Borders::ALL);
            f.render_widget(block, size);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('p') => {
                        // Exemple : lancer la lecture d'un fichier hardcodé
                        if let Err(e) = audio::play_audio("music/ton_fichier.mp3") {
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
