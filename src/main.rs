use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::PathBuf;

mod gguf;

// --- Data Structures ---

#[derive(PartialEq)]
enum View {
    Dashboard,
    Hexdump,
}

struct AppState {
    view: View,
    hex_offset: u64,
    file_path: PathBuf,
}

// --- Hexdump Helper ---

fn get_hexdump(path: &PathBuf, offset: u64, lines: u16) -> Vec<Line<'_>> {
    let mut file = File::open(path).unwrap();
    let mut buffer = vec![0u8; (lines * 16) as usize];
    file.seek(SeekFrom::Start(offset)).unwrap();
    let n = file.read(&mut buffer).unwrap();
    
    buffer[..n].chunks(16).enumerate().map(|(i, chunk)| {
        let current_offset = offset + (i as u64 * 16);
        let hex_part = chunk.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ");
        let ascii_part: String = chunk.iter().map(|&b| if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' }).collect();
        
        Line::from(vec![
            Span::styled(format!("{:08x}: ", current_offset), Style::default().fg(Color::Yellow)),
            Span::raw(format!("{:48}  ", hex_part)),
            Span::styled(ascii_part, Style::default().fg(Color::Cyan)),
        ])
    }).collect()
}

// --- UI Logic ---

fn render_dashboard(f: &mut Frame, area: Rect, data: &gguf::GgufFile) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(0)])
        .split(area);

    let rows = vec![
        Line::from(vec![Span::raw(format!("GGUF version: {}", data.version))]),
        Line::from(vec![Span::raw(format!("Tensor count: {}", data.tensor_count))]),
        Line::from(vec![Span::raw(format!("Metadata KV count: {}", data.metadata_kv_count))]),
        Line::from(vec![Span::raw("Press [H] to open hexdump view".to_string())]),
    ];

    f.render_widget(Paragraph::new(rows).block(Block::default().title(" Header ").borders(Borders::ALL)), chunks[0]);
}

fn render_hexdump(f: &mut Frame, area: Rect, app: &AppState) {
    let lines = get_hexdump(&app.file_path, app.hex_offset, area.height - 2);
    f.render_widget(Paragraph::new(lines).block(Block::default().title(" Hex Viewer ").borders(Borders::ALL)), area);
}

#[derive(Parser)]
struct Args { path: PathBuf }

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let data = gguf::GgufFile::parse(&args.path)?;
    let mut app = AppState { view: View::Dashboard, hex_offset: 0, file_path: args.path };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    loop {
        terminal.draw(|f| {
            let main_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(f.size());

            let help = "[Q]uit | [H]exdump | [M]ain Dashboard | [Up/Down] Scroll Hex";
            f.render_widget(Paragraph::new(help).block(Block::default().borders(Borders::ALL)), main_layout[0]);

            match app.view {
                View::Dashboard => render_dashboard(f, main_layout[1], &data),
                View::Hexdump => render_hexdump(f, main_layout[1], &app),
            }
        })?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('h') => app.view = View::Hexdump,
                    KeyCode::Char('m') => app.view = View::Dashboard,
                    KeyCode::Down if app.view == View::Hexdump => app.hex_offset += 16,
                    KeyCode::Up if app.view == View::Hexdump && app.hex_offset >= 16 => app.hex_offset -= 16,
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
