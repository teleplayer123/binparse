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
use regex::bytes::Regex;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::OnceLock;

mod gguf;
mod elf;
mod macho;

static URL_REGEX: OnceLock<Regex> = OnceLock::new();

// --- Data Structures ---

#[derive(PartialEq)]
enum View {
    Dashboard,
    Hexdump,
    URLs,
}

struct AppState {
    view: View,
    offset: u64,
    file_path: PathBuf,
    url_cache: Option<Vec<String>>,
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

fn scan_urls_from_file(path: &PathBuf) -> Vec<String> {
    let re = URL_REGEX.get_or_init(|| Regex::new(r"https?://[\x21-\x7e]+").unwrap());
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    const CHUNK_SIZE: usize = 1024 * 1024; // 1MB per read
    const OVERLAP: usize = 4096;           // carry-over to catch URLs spanning chunk boundaries
    let mut urls = Vec::new();
    let mut tail: Vec<u8> = Vec::new();
    let mut chunk = vec![0u8; CHUNK_SIZE];

    loop {
        let n = match file.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };

        let mut combined = tail;
        combined.extend_from_slice(&chunk[..n]);
        let boundary = combined.len() - n; // byte offset where new data starts

        for m in re.find_iter(&combined) {
            if m.start() >= boundary {
                if let Ok(s) = std::str::from_utf8(m.as_bytes()) {
                    urls.push(s.to_string());
                }
            }
        }

        let keep_from = combined.len().saturating_sub(OVERLAP);
        tail = combined[keep_from..].to_vec();
        if n < CHUNK_SIZE {
            break;
        }
    }

    urls
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

fn render_dashboard(f: &mut Frame, area: Rect, data: &DataFile, offset: usize) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(0)])
        .split(area);

    let rows = match &data.data_type {
        DataFileType::GGUF(gguf_file) => {
            vec![
                Line::from(vec![Span::raw(format!("Magic Number: 0x{:08x}", gguf_file.magic))]),
                Line::from(vec![Span::raw(format!("GGUF Version: {}", gguf_file.version))]),
                Line::from(vec![Span::raw(format!("Tensor Count: {}", gguf_file.tensor_count))]),
                Line::from(vec![Span::raw(format!("Metadata KV Count: {}", gguf_file.metadata_kv_count))]),
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

    if let DataFileType::GGUF(gguf_file) = &data.data_type {
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(vec![Span::styled(" Metadata ", Style::default().fg(Color::Yellow))]));
        for entry in &gguf_file.metadata {
            let raw = entry.value.to_string();
            let value_str = if raw.len() > 80 { format!("{}…", &raw[..80]) } else { raw };
            lines.push(Line::from(vec![
                Span::styled(format!("  {}: ", entry.key), Style::default().fg(Color::Cyan)),
                Span::raw(value_str),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(" Tensors ", Style::default().fg(Color::Yellow))]));
        for tensor in &gguf_file.tensors {
            let dims: Vec<String> = tensor.dimensions.iter().map(|d| d.to_string()).collect();
            lines.push(Line::from(vec![
                Span::styled(format!("  {}: ", tensor.name), Style::default().fg(Color::Cyan)),
                Span::raw(format!("{} [{}] @ offset {}", tensor.type_name(), dims.join("×"), tensor.offset)),
            ]));
        }

        let visible_height = chunks[1].height.saturating_sub(2) as usize;
        let visible: Vec<Line> = lines.into_iter().skip(offset).take(visible_height).collect();
        f.render_widget(
            Paragraph::new(visible).block(Block::default().title(" Metadata & Tensors ").borders(Borders::ALL)),
            chunks[1],
        );
    }
}

fn render_hexdump(f: &mut Frame, area: Rect, app: &AppState) {
    let lines = get_hexdump(&app.file_path, app.offset, area.height - 2);
    f.render_widget(Paragraph::new(lines).block(Block::default().title(" Hex Viewer ").borders(Borders::ALL)), area);
}

fn render_urls(f: &mut Frame, area: Rect, app: &AppState) {
    let empty = Vec::new();
    let all_urls = app.url_cache.as_ref().unwrap_or(&empty);
    let lines_to_show = area.height.saturating_sub(2) as usize;
    let lines: Vec<Line> = all_urls
        .iter()
        .skip(app.offset as usize)
        .take(lines_to_show)
        .map(|url| Line::from(vec![Span::raw(url.as_str())]))
        .collect();
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
    let mut app = AppState { view: View::Dashboard, offset: 0, file_path: args.path, url_cache: None };

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
            let help = "[Q]uit | [H]exdump | [U]RLs | [M]ain Dashboard | [Up/Down] Scroll";
            let help_widget = Paragraph::new(help)
                .style(Style::default().fg(Color::Cyan))
                .block(Block::default().borders(Borders::ALL).title("Help"));
            f.render_widget(help_widget, main_layout[1]);

            // Main content
            match app.view {
                View::Dashboard => render_dashboard(f, main_layout[2], &data, app.offset as usize),
                View::Hexdump => render_hexdump(f, main_layout[2], &app),
                View::URLs => render_urls(f, main_layout[2], &app),
            }
        })?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('h') => {
                        app.view = View::Hexdump;
                        app.offset = 0;
                    },
                    KeyCode::Char('u') => {
                        if app.url_cache.is_none() {
                            app.url_cache = Some(scan_urls_from_file(&app.file_path));
                        }
                        app.view = View::URLs;
                        app.offset = 0;
                    },
                    KeyCode::Char('m') => app.view = View::Dashboard,
                    KeyCode::Down => {
                        match app.view {
                            View::Hexdump => app.offset += 16,
                            View::URLs => app.offset += 1,
                            View::Dashboard => app.offset += 1,
                        }
                    },
                    KeyCode::Up => {
                        match app.view {
                            View::Hexdump if app.offset >= 16 => app.offset -= 16,
                            View::URLs if app.offset >= 1 => app.offset -= 1,
                            View::Dashboard if app.offset >= 1 => app.offset -= 1,
                            View::Hexdump | View::URLs | View::Dashboard => app.offset = 0,
                        }
                    },
                    KeyCode::Char(' ') => {
                        match app.view {
                            View::Hexdump => app.offset += 160,
                            View::URLs => app.offset += 10,
                            View::Dashboard => app.offset += 10,
                        }
                    },
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
