mod youtube;
mod audio;
mod playlist;

use std::io::{self, Write};
use std::fs;
use std::time::{Duration, Instant};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    event::{self, Event, KeyCode, EnableMouseCapture, DisableMouseCapture},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, List, ListItem, Gauge, Clear, Wrap},
    text:: Line,
    Frame,
};
use playlist::{Track, PlaylistManager};

struct AppState {
    playlist_manager: PlaylistManager,
    current_playlist: String,
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
        let mut playlist_manager = PlaylistManager::new();
        playlist_manager.create_playlist("default");
        Self {
            playlist_manager,
            current_playlist: "default".to_string(),
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

    fn current_tracks(&self) -> Vec<Track> {
        self.playlist_manager
            .playlists
            .get(&self.current_playlist)
            .map(|p| p.tracks.clone())
            .unwrap_or_default()
    }

    fn add_track_to_current(&mut self, track: Track) {
        self.playlist_manager
            .add_track_to_playlist(&self.current_playlist, track);
        self.status_message =
            format!("Ajouté à '{}' ({} pistes)", self.current_playlist, self.current_tracks().len());
    }

    fn next_track(&mut self) {
        let tracks = self.current_tracks();
        if !tracks.is_empty() {
            audio::stop_audio();
            self.current_track = (self.current_track + 1) % tracks.len();
            self.progress = 0.0;
            self.is_playing = false;
            self.is_paused = false;
            self.status_message = format!("Piste suivante: {}", tracks[self.current_track].title);
            self.play_current_track();
        }
    }

    fn previous_track(&mut self) {
        let tracks = self.current_tracks();
        if !tracks.is_empty() {
            audio::stop_audio();
            self.current_track = if self.current_track == 0 {
                tracks.len() - 1
            } else {
                self.current_track - 1
            };
            self.progress = 0.0;
            self.is_playing = false;
            self.is_paused = false;
            self.status_message = format!("Piste précédente: {}", tracks[self.current_track].title);
            self.play_current_track();
        }
    }

    fn toggle_play_pause(&mut self) {
        let tracks = self.current_tracks();
        if tracks.is_empty() {
            self.status_message = "Aucune piste chargée".to_string();
            return;
        }

        if self.is_playing {
            self.is_paused = !self.is_paused;
            if self.is_paused {
                audio::pause_audio();
                self.status_message = "⏸️ Pause".to_string();
            } else {
                audio::resume_audio();
                self.status_message = "▶️ Lecture".to_string();
            }
        } else {
            self.play_current_track();
        }
    }

    fn play_current_track(&mut self) {
        let tracks = self.current_tracks();
        if let Some(track) = tracks.get(self.current_track) {
            audio::stop_audio();
            match audio::play_audio(&track.file_path) {
                Ok(_) => {
                    self.is_playing = true;
                    self.is_paused = false;
                    self.status_message = format!("▶️ Lecture: {}", track.title);
                }
                Err(e) => {
                    self.is_playing = false;
                    self.is_paused = false;
                    self.status_message = format!("❌ Erreur lecture: {}", e);
                }
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
            self.progress += 0.001;
            let tracks = self.current_tracks();
            if self.progress >= 1.0 && !tracks.is_empty() {
                self.progress = 0.0;
                self.next_track();
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !youtube::check_yt_dlp_available() {
        eprintln!("❌ yt-dlp n'est pas installé ou accessible.");
        eprintln!("Veuillez installer yt-dlp: pip install yt-dlp");
        return Ok(());
    }

    println!("-keeplisten-");
    println!("===================");

    let music_dir = "music";
    fs::create_dir_all(music_dir)?;

    let mut app_state = AppState::new();
    let _ = app_state.playlist_manager.load_all_from_dir("playlists");
    if app_state.current_tracks().is_empty() {
        load_existing_tracks(&mut app_state, music_dir)?;
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, &mut app_state, music_dir);

    let _ = app_state.playlist_manager.save_all_to_dir("playlists");
    audio::stop_audio();
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
    let mut any = false;
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
                        file_path: entry.path().display().to_string(),
                        url: None,
                        duration: None,
                    };
                    app_state.add_track_to_current(track);
                    any = true;
                }
            }
        }
    }
    if any {
        app_state.status_message = "Musique locale chargée dans la playlist par défaut".to_string();
    }
    Ok(())
}

fn prompt(question: &str) -> io::Result<String> {
    print!("{}", question);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app_state: &mut AppState,
    music_dir: &str,
) -> io::Result<()> {
    loop {
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
                        // Playlists shortcuts
                        KeyCode::Char('P') => {
                            let name = prompt("Nom de la nouvelle playlist : ")?;
                            if app_state.playlist_manager.create_playlist(&name) {
                                app_state.status_message = format!("Playlist '{}' créée", name);
                            } else {
                                app_state.status_message = format!("Playlist '{}' existe déjà", name);
                            }
                        }
                        KeyCode::Char('D') => {
                            let pl = app_state.current_playlist.clone();
                            if pl == "default" {
                                app_state.status_message = "Impossible de supprimer la playlist par défaut".to_string();
                            } else if app_state.playlist_manager.delete_playlist(&pl) {
                                app_state.status_message = format!("Playlist '{}' supprimée", pl);
                                app_state.current_playlist = "default".into();
                                app_state.current_track = 0;
                            } else {
                                app_state.status_message = format!("Impossible de supprimer '{}'", pl);
                            }
                        }
                        KeyCode::Char('A') => {
                            let target = prompt("Ajouter la piste courante à quelle playlist ? ")?;
                            let tracks = app_state.current_tracks();
                            if let Some(track) = tracks.get(app_state.current_track).cloned() {
                                if app_state.playlist_manager.add_track_to_playlist(&target, track) {
                                    app_state.status_message = format!("Ajouté à '{}'", target);
                                } else {
                                    app_state.status_message = "Playlist introuvable".to_string();
                                }
                            }
                        }
                        KeyCode::Char('S') => {
                            let pl = app_state.current_playlist.clone();
                            if app_state
                                .playlist_manager
                                .remove_track_from_playlist_by_index(&pl, app_state.current_track)
                            {
                                app_state.status_message = "Piste retirée".to_string();
                                app_state.current_track = 0;
                            } else {
                                app_state.status_message = "Erreur suppression".to_string();
                            }
                        }
                        KeyCode::Char('L') => {
                            let mut msg = "Playlists : ".to_string();
                            let list = app_state
                                .playlist_manager
                                .playlists
                                .keys()
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(", ");
                            msg.push_str(&list);
                            app_state.status_message = msg;
                        }
                        KeyCode::Char('C') => {
                            let name = prompt("Aller à la playlist : ")?;
                            if app_state.playlist_manager.playlists.contains_key(&name) {
                                app_state.current_playlist = name;
                                app_state.current_track = 0;
                                app_state.progress = 0.0;
                                app_state.status_message = "Changement de playlist".to_string();
                            } else {
                                app_state.status_message = "Playlist introuvable".to_string();
                            }
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
                if let Some((url, title)) = youtube::search_first_video(&query) {
                    if let Ok(file_path) = youtube::download_audio(&url, music_dir) {
                        let track = Track {
                            title,
                            file_path,
                            url: Some(url),
                            duration: None,
                        };
                        app_state.add_track_to_current(track);
                    } else {
                        app_state.status_message = "Erreur lors du téléchargement".to_string();
                    }
                } else {
                    app_state.status_message = "Aucun résultat trouvé".to_string();
                }
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
    let tracks = app_state.current_tracks();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(8),     // Playlist
            Constraint::Length(5),  // Player controls
            Constraint::Length(3),  // Status
        ])
        .split(f.area());

    let header = Paragraph::new(format!(
        "- Keeplisten -  [Playlist: {}]",
        app_state.current_playlist
    ))
    .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    let items: Vec<ListItem> = tracks
        .iter()
        .enumerate()
        .map(|(i, track)| {
            let symbol = if i == app_state.current_track {
                if app_state.is_playing && !app_state.is_paused {
                    "▶️"
                } else if app_state.is_paused {
                    "⏸️"
                } else {
                    "⏹️"
                }
            } else {
                "  "
            };
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

    draw_player_controls(f, chunks[2], app_state);

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

    let progress = Gauge::default()
        .block(Block::default())
        .gauge_style(Style::default().fg(Color::Cyan))
        .ratio(app_state.progress);
    f.render_widget(progress, chunks[0]);

    let controls = Paragraph::new("Space: Play/Pause | ←→: Piste | ↑↓: Volume | S: Recherche | P: Nouvelle Playlist | D: Suppr Playlist | A: Ajout piste | S: Suppr piste | L: Lister | C: Changer | H: Aide | Q: Quitter")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    f.render_widget(controls, chunks[1]);

    let volume = Paragraph::new(format!("🔊 Volume: {}%", app_state.volume))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center);
    f.render_widget(volume, chunks[2]);
}

fn draw_search_popup(f: &mut Frame, app_state: &AppState) {
    draw_main_ui(f, app_state);
    let popup_area = centered_rect(50, 20, f.area());
    f.render_widget(Clear, popup_area);
    let input = Paragraph::new(app_state.search_input.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default()
            .borders(Borders::ALL)
            .title("🔍 Rechercher une musique"));
    f.render_widget(input, popup_area);
}

fn draw_help_popup(f: &mut Frame) {
    let popup_area = centered_rect(60, 70, f.area());
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
        Line::from("  P        - Nouvelle playlist"),
        Line::from("  D        - Supprimer playlist"),
        Line::from("  A        - Ajouter piste à playlist"),
        Line::from("  S        - Supprimer piste"),
        Line::from("  L        - Lister playlists"),
        Line::from("  C        - Changer de playlist"),
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
