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
    widgets::{Block, Borders, Paragraph},
    prelude::Alignment,
    Frame, Terminal,
};
use regex::Regex;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::PathBuf;

mod gguf;
mod elf;
mod macho;

// --- Data Structures ---

#[derive(PartialEq)]
enum View {
    Dashboard,
    Hexdump,
    URLs,
}

struct AppState {
    view: View,
    hex_offset: u64,
    file_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DataFileType {
    GGUF(gguf::GgufFile),
    ELF(elf::ElfFile),
    MachO(macho::MachHeader64),
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataFile {
    pub data_type: DataFileType,
}

impl DataFile {
    pub fn from_file(path: &PathBuf) -> io::Result<Self> {
        // Get correct file type
        if let Ok(gguf_file) = Self::from_gguf(path) {
            return Ok(gguf_file);
        } else if let Ok(elf_file) = Self::from_elf(path) {
            return Ok(elf_file);
        } else if let Ok(macho_file) = Self::from_macho(path) {
            return Ok(macho_file);
        } else {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Unsupported file format"));
        }
    }

    pub fn from_gguf(path: &PathBuf) -> io::Result<Self> {
        let gguf_file = gguf::GgufFile::parse(path)?;
        Ok(DataFile {
            data_type: DataFileType::GGUF(gguf_file),
        })
    }

    pub fn from_elf(path: &PathBuf) -> io::Result<Self> {
        let elf_file = elf::ElfFile::parse(path)?;
        Ok(DataFile {
            data_type: DataFileType::ELF(elf_file),
        })
    }

    pub fn from_macho(path: &PathBuf) -> io::Result<Self> {
        let file = File::open(path)?;
        let macho_file = macho::MachHeader64::parse(file)?;
        Ok(DataFile {
            data_type: DataFileType::MachO(macho_file),
        })
    }
}

// --- Helpers ---

fn get_urls_from_file(path: &PathBuf) -> Vec<String> {
    let mut file = File::open(path).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    let content = String::from_utf8_lossy(&buffer);
    let url_regex = Regex::new(r"https?://[^\s]+").unwrap();
    url_regex.find_iter(&content).map(|m| m.as_str().to_string()).collect()
}

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

fn render_dashboard(f: &mut Frame, area: Rect, data: &DataFile) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(0)])
        .split(area);

    let rows = match &data.data_type {
        DataFileType::GGUF(gguf_file) => {
            vec![
                Line::from(vec![Span::raw(format!("Magic Number: 0x{:04x}", gguf_file.magic))]),
                Line::from(vec![Span::raw(format!("GGUF version: {:?}", gguf_file.version))]),
                Line::from(vec![Span::raw(format!("Tensor count: {:?}", gguf_file.tensor_count))]),
                Line::from(vec![Span::raw(format!("Metadata KV count: {:?}", gguf_file.metadata_kv_count))]),
            ]
        }
        DataFileType::ELF(elf_file) => {
            vec![
                Line::from(vec![Span::raw(format!("Magic Number: 0x{:04x}", elf_file.magic))]),
                Line::from(vec![Span::raw(format!("ELF file detected, size: {} bytes", elf_file.data.len()))]),
            ]
        }
        DataFileType::MachO(macho_file) => {
            vec![
                Line::from(vec![Span::raw(format!("Magic Number: 0x{:04x}", macho_file.magic))]),
                Line::from(vec![Span::raw(format!("CPU Type: {}", macho_file.cputype))]),
                Line::from(vec![Span::raw(format!("CPU Subtype: {}", macho_file.cpusubtype))]),
                Line::from(vec![Span::raw(format!("File Type: {}", macho_file.filetype))]),
                Line::from(vec![Span::raw(format!("Number of Commands: {}", macho_file.ncmds))]),
                Line::from(vec![Span::raw(format!("Size of Commands: {}", macho_file.sizeofcmds))]),
                Line::from(vec![Span::raw(format!("Flags: 0x{:04x}", macho_file.flags))]),
            ]
        }
        DataFileType::Unknown => {
            vec![
                Line::from(vec![Span::raw("Unknown file format".to_string())]),
                Line::from(vec![Span::raw("Press [H] to open hexdump view".to_string())]),
            ]
        }
    };

    f.render_widget(Paragraph::new(rows).block(Block::default().title(" Header ").borders(Borders::ALL)), chunks[0]);
}

fn render_hexdump(f: &mut Frame, area: Rect, app: &AppState) {
    let lines = get_hexdump(&app.file_path, app.hex_offset, area.height - 2);
    f.render_widget(Paragraph::new(lines).block(Block::default().title(" Hex Viewer ").borders(Borders::ALL)), area);
}

fn render_urls(f: &mut Frame, area: Rect, app: &AppState) {
    let urls = get_urls_from_file(&app.file_path);
    let lines: Vec<Line> = urls.into_iter().map(|url| Line::from(vec![Span::raw(url)])).collect();
    f.render_widget(Paragraph::new(lines).block(Block::default().title(" URLs ").borders(Borders::ALL)), area);
}

#[derive(Parser)]
struct Args { path: PathBuf }

const BANNER: &str = r#"
██████╗ ██╗███╗   ██╗██████╗  █████╗ ██████╗ ███████╗███████╗
██╔══██╗██║████╗  ██║██╔══██╗██╔══██╗██╔══██╗██╔════╝██╔════╝
██████╔╝██║██╔██╗ ██║██████╔╝███████║██████╔╝███████╗█████╗  
██╔══██╗██║██║╚██╗██║██╔═══╝ ██╔══██║██╔══██╗╚════██║██╔══╝  
██████╔╝██║██║ ╚████║██║     ██║  ██║██║  ██║███████║███████╗
╚═════╝ ╚═╝╚═╝  ╚═══╝╚═╝     ╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝╚══════╝"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let data = DataFile::from_file(&args.path).unwrap_or(DataFile { data_type: DataFileType::Unknown });
    let mut app = AppState { view: View::Dashboard, hex_offset: 0, file_path: args.path };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    loop {
        terminal.draw(|f| {
            let main_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(10),
                    Constraint::Length(3),
                    Constraint::Min(0),
                ])
                .split(f.size());

            // Banner
            let banner = Paragraph::new(BANNER)
                .style(Style::default().fg(Color::Blue))
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Center);
            f.render_widget(banner, main_layout[0]);

            // Help bar
            let help = "[Q]uit | [H]exdump | [U]RLs | [M]ain Dashboard | [Up/Down] Scroll Hex";
            let help_widget = Paragraph::new(help)
                .style(Style::default().fg(Color::Cyan))
                .block(Block::default().borders(Borders::ALL).title("Help"));
            f.render_widget(help_widget, main_layout[1]);

            // Main content
            match app.view {
                View::Dashboard => render_dashboard(f, main_layout[2], &data),
                View::Hexdump => render_hexdump(f, main_layout[2], &app),
                View::URLs => render_urls(f, main_layout[2], &app),
            }
        })?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('h') => app.view = View::Hexdump,
                    KeyCode::Char('u') => app.view = View::URLs,
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
