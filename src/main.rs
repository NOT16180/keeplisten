mod youtube;
mod audio;

use std::io::{self, Write};
use std::fs;
use std::time::{Duration, Instant};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};

#[derive(Clone)]
struct Track {
    title: String,
    url: String,
    file_path: Option<String>,
    duration: Option<String>,
}

struct AppState {
    tracks: Vec<Track>,
    current_track: usize,
    is_playing: bool,
    is_paused: bool,
    volume: u8,
    progress: f64,
    status_message: String,
    show_help: bool,
    last_update: Instant,
    search_mode: bool,
    search_input: String,
}

impl AppState {
    fn new() -> Self {
        Self {
            tracks: Vec::new(),
            current_track: 0,
            is_playing: false,
            is_paused: false,
            volume: 70,
            progress: 0.0,
            status_message: "Prêt - Utilisez 's' pour rechercher une musique".to_string(),
            show_help: false,
            last_update: Instant::now(),
            search_mode: false,
            search_input: String::new(),
        }
    }

    fn add_track(&mut self, track: Track) {
        self.tracks.push(track);
        self.status_message = format!("Ajouté: {} pistes au total", self.tracks.len());
    }

    fn next_track(&mut self) {
        if !self.tracks.is_empty() {
            self.current_track = (self.current_track + 1) % self.tracks.len();
            self.progress = 0.0;
            self.status_message = format!("Piste suivante: {}", 
                self.tracks[self.current_track].title);
        }
    }

    fn previous_track(&mut self) {
        if !self.tracks.is_empty() {
            self.current_track = if self.current_track == 0 {
                self.tracks.len() - 1
            } else {
                self.current_track - 1
            };
            self.progress = 0.0;
            self.status_message = format!("Piste précédente: {}", 
                self.tracks[self.current_track].title);
        }
    }

    fn toggle_play_pause(&mut self) {
        if self.tracks.is_empty() {
            self.status_message = "Aucune piste chargée".to_string();
            return;
        }

        if self.is_playing {
            self.is_paused = !self.is_paused;
            self.status_message = if self.is_paused { 
                "⏸️ Pause".to_string() 
            } else { 
                "▶️ Lecture".to_string() 
            };
        } else {
            self.play_current_track();
        }
    }

    fn play_current_track(&mut self) {
        if let Some(track) = self.tracks.get(self.current_track) {
            if let Some(file_path) = &track.file_path {
                match audio::play_audio(file_path) {
                    Ok(_) => {
                        self.is_playing = true;
                        self.is_paused = false;
                        self.status_message = format!("▶️ Lecture: {}", track.title);
                    }
                    Err(e) => {
                        self.status_message = format!("❌ Erreur lecture: {}", e);
                    }
                }
            } else {
                self.status_message = "Fichier audio non disponible".to_string();
            }
        }
    }

    fn adjust_volume(&mut self, delta: i8) {
        let new_volume = (self.volume as i8 + delta).clamp(0, 100) as u8;
        self.volume = new_volume;
        self.status_message = format!("🔊 Volume: {}%", self.volume);
    }

    fn update_progress(&mut self) {
        if self.is_playing && !self.is_paused {
            // Simulation du progrès (dans une vraie app, ceci viendrait du lecteur audio)
            self.progress += 0.01;
            if self.progress >= 1.0 {
                self.progress = 0.0;
                self.next_track();
                if !self.tracks.is_empty() {
                    self.play_current_track();
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Vérifier que yt-dlp est disponible
    if !youtube::check_yt_dlp_available() {
        eprintln!("❌ yt-dlp n'est pas installé ou accessible.");
        eprintln!("Veuillez installer yt-dlp: pip install yt-dlp");
        return Ok(());
    }

    println!("🎵 Lecteur Musical TUI");
    println!("===================");
    
    // Préparer le dossier de musique
    let music_dir = "music";
    fs::create_dir_all(music_dir)?;

    // Charger les pistes existantes
    let mut app_state = AppState::new();
    load_existing_tracks(&mut app_state, music_dir)?;

    // Demande initiale si aucune piste n'est chargée
    if app_state.tracks.is_empty() {
        if let Some(track) = search_and_download_interactive(music_dir)? {
            app_state.add_track(track);
        }
    }

    // Initialisation de la TUI
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, &mut app_state, music_dir);

    // Nettoyage du terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("❌ Erreur application: {:?}", err);
    }

    Ok(())
}

fn load_existing_tracks(app_state: &mut AppState, music_dir: &str) -> io::Result<()> {
    if let Ok(entries) = fs::read_dir(music_dir) {
        for entry in entries.flatten() {
            if let Some(ext) = entry.path().extension() {
                if ext == "mp3" {
                    let title = entry.path()
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Titre inconnu")
                        .to_string();
                    
                    let track = Track {
                        title,
                        url: String::new(),
                        file_path: Some(entry.path().display().to_string()),
                        duration: None,
                    };
                    
                    app_state.tracks.push(track);
                }
            }
        }
    }
    
    if !app_state.tracks.is_empty() {
        app_state.status_message = format!("Chargé {} piste(s) existante(s)", app_state.tracks.len());
    }
    
    Ok(())
}

fn search_and_download_interactive(music_dir: &str) -> Result<Option<Track>, Box<dyn std::error::Error>> {
    print!("🔍 Entrez le nom de la musique à rechercher: ");
    io::stdout().flush()?;
    
    let mut song_name = String::new();
    io::stdin().read_line(&mut song_name)?;
    let song_name = song_name.trim();
    
    if song_name.is_empty() {
        return Ok(None);
    }

    println!("🔎 Recherche en cours...");
    
    let (video_url, video_title) = match youtube::search_first_video(song_name) {
        Some(result) => result,
        None => {
            println!("❌ Aucun résultat trouvé pour '{}'", song_name);
            return Ok(None);
        }
    };

    println!("✅ Trouvé: {}", video_title);
    println!("🔗 URL: {}", video_url);
    print!("📥 Télécharger? (o/n): ");
    io::stdout().flush()?;

    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    
    if !["o", "y", "oui", "yes"].contains(&answer.trim().to_lowercase().as_str()) {
        println!("❌ Téléchargement annulé");
        return Ok(None);
    }

    println!("⬇️ Téléchargement en cours...");
    
    match youtube::download_audio(&video_url, music_dir) {
        Ok(file_path) => {
            println!("✅ Téléchargé: {}", file_path);
            Ok(Some(Track {
                title: video_title,
                url: video_url,
                file_path: Some(file_path),
                duration: None,
            }))
        }
        Err(e) => {
            println!("❌ Erreur téléchargement: {}", e);
            Ok(None)
        }
    }
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app_state: &mut AppState,
    music_dir: &str,
) -> io::Result<()> {
    loop {
        // Mettre à jour le progrès
        if app_state.last_update.elapsed() >= Duration::from_millis(100) {
            app_state.update_progress();
            app_state.last_update = Instant::now();
        }

        terminal.draw(|f| {
            if app_state.search_mode {
                draw_search_popup(f, app_state);
            } else {
                draw_main_ui(f, app_state);
            }
            
            if app_state.show_help {
                draw_help_popup(f);
            }
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if app_state.search_mode {
                    handle_search_input(app_state, key, music_dir)?;
                } else {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Char('p') | KeyCode::Char(' ') => app_state.toggle_play_pause(),
                        KeyCode::Char('n') | KeyCode::Right => app_state.next_track(),
                        KeyCode::Char('b') | KeyCode::Left => app_state.previous_track(),
                        KeyCode::Char('s') => {
                            app_state.search_mode = true;
                            app_state.search_input.clear();
                        }
                        KeyCode::Char('h') => app_state.show_help = !app_state.show_help,
                        KeyCode::Char('+') | KeyCode::Up => app_state.adjust_volume(5),
                        KeyCode::Char('-') | KeyCode::Down => app_state.adjust_volume(-5),
                        KeyCode::Char('r') => {
                            app_state.progress = 0.0;
                            app_state.status_message = "⏮️ Remis au début".to_string();
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}

fn handle_search_input(
    app_state: &mut AppState, 
    key: crossterm::event::KeyEvent, 
    music_dir: &str
) -> io::Result<()> {
    match key.code {
        KeyCode::Enter => {
            if !app_state.search_input.trim().is_empty() {
                let query = app_state.search_input.clone();
                app_state.search_mode = false;
                app_state.status_message = format!("🔎 Recherche: {}", query);
                
                // Ici, dans une vraie app, on ferait la recherche en arrière-plan
                // Pour cet exemple, on simule juste l'ajout
                let track = Track {
                    title: format!("Recherche: {}", query),
                    url: String::new(),
                    file_path: None,
                    duration: None,
                };
                app_state.add_track(track);
            } else {
                app_state.search_mode = false;
            }
        }
        KeyCode::Esc => {
            app_state.search_mode = false;
            app_state.search_input.clear();
        }
        KeyCode::Backspace => {
            app_state.search_input.pop();
        }
        KeyCode::Char(c) => {
            app_state.search_input.push(c);
        }
        _ => {}
    }
    Ok(())
}

fn draw_main_ui(f: &mut Frame, app_state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(8),     // Playlist
            Constraint::Length(5),  // Player controls
            Constraint::Length(3),  // Status
        ])
        .split(f.size());

    // Header
    let header = Paragraph::new("🎵 Lecteur Musical TUI")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    // Playlist
    let items: Vec<ListItem> = app_state.tracks
        .iter()
        .enumerate()
        .map(|(i, track)| {
            let symbol = if i == app_state.current_track {
                if app_state.is_playing && !app_state.is_paused { "▶️" }
                else if app_state.is_paused { "⏸️" }
                else { "⏹️" }
            } else { "  " };
            
            let style = if i == app_state.current_track {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            
            ListItem::new(format!("{} {}", symbol, track.title)).style(style)
        })
        .collect();

    let playlist = List::new(items)
        .block(Block::default()
            .title("📋 Playlist")
            .borders(Borders::ALL))
        .style(Style::default().fg(Color::White));
    f.render_widget(playlist, chunks[1]);

    // Player controls
    draw_player_controls(f, chunks[2], app_state);

    // Status bar
    let status = Paragraph::new(app_state.status_message.as_str())
        .style(Style::default().fg(Color::Green))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("📊 Status"));
    f.render_widget(status, chunks[3]);
}

fn draw_player_controls(f: &mut Frame, area: Rect, app_state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(Block::default().borders(Borders::ALL).title("🎮 Contrôles").inner(area));

    // Progress bar
    let progress = Gauge::default()
        .block(Block::default())
        .gauge_style(Style::default().fg(Color::Cyan))
        .ratio(app_state.progress);
    f.render_widget(progress, chunks[0]);

    // Controls info
    let controls = Paragraph::new("Space: Play/Pause | ←→: Piste | ↑↓: Volume | S: Recherche | H: Aide | Q: Quitter")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    f.render_widget(controls, chunks[1]);

    // Volume
    let volume = Paragraph::new(format!("🔊 Volume: {}%", app_state.volume))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center);
    f.render_widget(volume, chunks[2]);
}

fn draw_search_popup(f: &mut Frame, app_state: &AppState) {
    draw_main_ui(f, app_state);
    
    let popup_area = centered_rect(50, 20, f.size());
    f.render_widget(Clear, popup_area);
    
    let input = Paragraph::new(app_state.search_input.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default()
            .borders(Borders::ALL)
            .title("🔍 Rechercher une musique"));
    f.render_widget(input, popup_area);
}

fn draw_help_popup(f: &mut Frame) {
    let popup_area = centered_rect(60, 70, f.size());
    f.render_widget(Clear, popup_area);
    
    let help_text = vec![
        Line::from("🎵 Aide du Lecteur Musical"),
        Line::from(""),
        Line::from("Contrôles:"),
        Line::from("  Space/P  - Play/Pause"),
        Line::from("  ←/→ N/B  - Piste précédente/suivante"),
        Line::from("  ↑/↓ +/-  - Volume +/-"),
        Line::from("  R        - Remettre au début"),
        Line::from("  S        - Rechercher une musique"),
        Line::from("  H/F1     - Afficher/masquer cette aide"),
        Line::from("  Q/Esc    - Quitter"),
        Line::from(""),
        Line::from("Appuyez sur H ou F1 pour fermer"),
    ];
    
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::White))
        .block(Block::default()
            .borders(Borders::ALL)
            .title("❓ Aide"))
        .wrap(Wrap { trim: true });
    f.render_widget(help, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
