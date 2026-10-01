use clap::Parser;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap,
    },
    Frame, Terminal,
};
use std::{
    error::Error,
    io::{self, Stdout},
    path::PathBuf,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::{Duration, Instant},
};
use sysinfo::Disks;

mod model;
use model::{format_size, DiskNode};

#[derive(Parser, Debug)]
#[command(
    name = "arch-disk-tui",
    author = "Emir <emirlkf@hotmail.com>",
    version = "0.1.0",
    about = "⚡ Ultra-fast, aesthetic terminal disk usage analyzer & treemap"
)]
struct CliArgs {
    /// Path to analyze (defaults to current directory or system root)
    #[arg(default_value = ".")]
    path: PathBuf,
}

const PALETTE: [Color; 7] = [
    Color::Rgb(125, 207, 255), // Arch Ice Blue
    Color::Rgb(120, 230, 160), // Mint Green
    Color::Rgb(255, 198, 109), // Amber Gold
    Color::Rgb(255, 120, 180), // Neon Rose
    Color::Rgb(180, 130, 255), // Cyber Violet
    Color::Rgb(255, 100, 100), // Crimson Warning
    Color::Rgb(100, 220, 240), // Cyan Glow
];

enum ScanMessage {
    Progress { files: usize, current_path: String },
    Item { path: PathBuf, size: u64, is_dir: bool },
    Finished,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortOrder {
    SizeDesc,
    NameAsc,
    FilesDesc,
}

struct App {
    root_path: PathBuf,
    current_path: PathBuf,
    root_node: DiskNode,
    disks: Disks,
    list_state: ListState,
    sort_order: SortOrder,
    scanning: bool,
    scanned_files_count: usize,
    current_scanning_item: String,
    scan_start_time: Instant,
    scan_duration: Option<Duration>,
    show_help: bool,
    filter_query: String,
    is_filtering: bool,
}

impl App {
    fn new(target_path: PathBuf) -> Self {
        let abs_path = std::fs::canonicalize(&target_path).unwrap_or(target_path);
        let name = abs_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| abs_path.to_string_lossy().to_string());

        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            root_path: abs_path.clone(),
            current_path: abs_path.clone(),
            root_node: DiskNode::new(name, abs_path.clone(), true),
            disks: Disks::new_with_refreshed_list(),
            list_state,
            sort_order: SortOrder::SizeDesc,
            scanning: true,
            scanned_files_count: 0,
            current_scanning_item: String::from("Starting scan..."),
            scan_start_time: Instant::now(),
            scan_duration: None,
            show_help: false,
            filter_query: String::new(),
            is_filtering: false,
        }
    }

    fn current_node(&self) -> Option<&DiskNode> {
        self.root_node.find_node(&self.current_path)
    }

    fn get_filtered_sorted_children(&self) -> Vec<DiskNode> {
        let Some(node) = self.current_node() else {
            return Vec::new();
        };

        let mut items: Vec<DiskNode> = node
            .children
            .values()
            .filter(|child| {
                if self.filter_query.is_empty() {
                    true
                } else {
                    child
                        .name
                        .to_lowercase()
                        .contains(&self.filter_query.to_lowercase())
                }
            })
            .cloned()
            .collect();

        match self.sort_order {
            SortOrder::SizeDesc => items.sort_by(|a, b| b.size.cmp(&a.size)),
            SortOrder::NameAsc => items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
            SortOrder::FilesDesc => items.sort_by(|a, b| (b.file_count + b.dir_count).cmp(&(a.file_count + a.dir_count))),
        }

        items
    }

    fn next_item(&mut self) {
        let items = self.get_filtered_sorted_children();
        if items.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i + 1 >= items.len() {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn previous_item(&mut self) {
        let items = self.get_filtered_sorted_children();
        if items.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn enter_dir(&mut self) {
        let items = self.get_filtered_sorted_children();
        if let Some(selected) = self.list_state.selected() {
            if let Some(target) = items.get(selected) {
                if target.is_dir && !target.children.is_empty() {
                    self.current_path = target.path.clone();
                    self.list_state.select(Some(0));
                    self.filter_query.clear();
                }
            }
        }
    }

    fn exit_dir(&mut self) {
        if self.current_path != self.root_path {
            if let Some(parent) = self.current_path.parent() {
                self.current_path = parent.to_path_buf();
                self.list_state.select(Some(0));
                self.filter_query.clear();
            }
        }
    }

    fn cycle_sort(&mut self) {
        self.sort_order = match self.sort_order {
            SortOrder::SizeDesc => SortOrder::NameAsc,
            SortOrder::NameAsc => SortOrder::FilesDesc,
            SortOrder::FilesDesc => SortOrder::SizeDesc,
        };
        self.list_state.select(Some(0));
    }
}

fn start_scanner(root: PathBuf, tx: mpsc::Sender<ScanMessage>) {
    thread::spawn(move || {
        let walker = jwalk::WalkDir::new(&root)
            .follow_links(false)
            .skip_hidden(false);

        let mut count = 0;
        for entry in walker {
            if let Ok(entry) = entry {
                let path = entry.path();
                let path_str = path.to_string_lossy();

                // Skip virtual filesystems if root is /
                if path_str.starts_with("/proc")
                    || path_str.starts_with("/sys")
                    || path_str.starts_with("/dev")
                    || path_str.starts_with("/run")
                {
                    continue;
                }

                let meta = entry.metadata().ok();
                let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                let size = if is_dir { 0 } else { meta.map(|m| m.len()).unwrap_or(0) };

                count += 1;
                if count % 200 == 0 {
                    let _ = tx.send(ScanMessage::Progress {
                        files: count,
                        current_path: path_str.to_string(),
                    });
                }

                let _ = tx.send(ScanMessage::Item {
                    path,
                    size,
                    is_dir,
                });
            }
        }
        let _ = tx.send(ScanMessage::Finished);
    });
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = CliArgs::parse();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(args.path);
    let (tx, rx): (mpsc::Sender<ScanMessage>, Receiver<ScanMessage>) = mpsc::channel();
    start_scanner(app.root_path.clone(), tx);

    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(50);

    let res = run_app(&mut terminal, &mut app, rx, tick_rate, &mut last_tick);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, cursor::Show)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Application Error: {err:?}");
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    rx: Receiver<ScanMessage>,
    tick_rate: Duration,
    last_tick: &mut Instant,
) -> Result<(), Box<dyn Error>> {
    loop {
        // Drain scanner messages up to a limit per frame to stay snappy
        let mut processed = 0;
        while processed < 1000 {
            match rx.try_recv() {
                Ok(ScanMessage::Item { path, size, is_dir }) => {
                    app.root_node.insert(&path, size, is_dir, &app.root_path);
                    processed += 1;
                }
                Ok(ScanMessage::Progress { files, current_path }) => {
                    app.scanned_files_count = files;
                    app.current_scanning_item = current_path;
                    processed += 1;
                }
                Ok(ScanMessage::Finished) => {
                    app.scanning = false;
                    app.scan_duration = Some(app.scan_start_time.elapsed());
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    app.scanning = false;
                    break;
                }
            }
        }

        terminal.draw(|f| render_ui(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if app.is_filtering {
                    match key.code {
                        KeyCode::Esc | KeyCode::Enter => {
                            app.is_filtering = false;
                        }
                        KeyCode::Backspace => {
                            app.filter_query.pop();
                            app.list_state.select(Some(0));
                        }
                        KeyCode::Char(c) => {
                            app.filter_query.push(c);
                            app.list_state.select(Some(0));
                        }
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(()),
                        KeyCode::Char('?') => app.show_help = !app.show_help,
                        KeyCode::Char('/') => {
                            app.is_filtering = true;
                        }
                        KeyCode::Char('s') => app.cycle_sort(),
                        KeyCode::Char('r') => {
                            app.disks.refresh(true);
                        }
                        KeyCode::Down | KeyCode::Char('j') => app.next_item(),
                        KeyCode::Up | KeyCode::Char('k') => app.previous_item(),
                        KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => app.enter_dir(),
                        KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace => app.exit_dir(),
                        KeyCode::Esc => {
                            if app.show_help {
                                app.show_help = false;
                            } else if !app.filter_query.is_empty() {
                                app.filter_query.clear();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            *last_tick = Instant::now();
        }
    }
}

fn render_ui(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Dark sleek background
    let bg_block = Block::default().style(Style::default().bg(Color::Rgb(15, 17, 26)));
    f.render_widget(bg_block, size);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Modern Header
            Constraint::Length(6), // Drive & System Storage Health
            Constraint::Min(10),   // Content: Tree Navigator + Treemap/Details
            Constraint::Length(1), // Minimal Bottom Status bar
        ])
        .split(size);

    render_header(f, app, chunks[0]);
    render_disks(f, app, chunks[1]);
    render_body(f, app, chunks[2]);
    render_footer(f, app, chunks[3]);

    if app.show_help {
        render_help_modal(f, size);
    }
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(20), Constraint::Length(32)])
        .split(area);

    // App Branding
    let logo_spans = vec![
        Span::styled(" 󰣇 ", Style::default().fg(Color::Rgb(23, 147, 209)).bold()),
        Span::styled("ARCH", Style::default().fg(Color::Cyan).bold()),
        Span::styled("·DISK·", Style::default().fg(Color::White).bold()),
        Span::styled("TUI", Style::default().fg(Color::Rgb(120, 230, 160)).bold()),
        Span::styled(" v0.1 ", Style::default().fg(Color::DarkGray)),
    ];
    let logo = Paragraph::new(Line::from(logo_spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(60, 70, 95))),
    );
    f.render_widget(logo, header_chunks[0]);

    // Current Breadcrumb / Path Info
    let path_display = app.current_path.to_string_lossy();
    let mut path_spans = vec![
        Span::styled("  Location: ", Style::default().fg(Color::DarkGray)),
        Span::styled(path_display, Style::default().fg(Color::Yellow).bold()),
    ];
    if !app.filter_query.is_empty() || app.is_filtering {
        path_spans.push(Span::styled("  [Search: ", Style::default().fg(Color::Cyan)));
        path_spans.push(Span::styled(&app.filter_query, Style::default().fg(Color::White).bold()));
        if app.is_filtering {
            path_spans.push(Span::styled("█", Style::default().fg(Color::Cyan).slow_blink()));
        }
        path_spans.push(Span::styled("]", Style::default().fg(Color::Cyan)));
    }

    let path_widget = Paragraph::new(Line::from(path_spans))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(60, 70, 95))),
        )
        .alignment(Alignment::Left);
    f.render_widget(path_widget, header_chunks[1]);

    // Live Scanner Status Indicator
    let status_widget = if app.scanning {
        let elapsed = app.scan_start_time.elapsed().as_secs_f32();
        let dots = match (app.scan_start_time.elapsed().as_millis() / 250) % 4 {
            0 => "⠋",
            1 => "⠙",
            2 => "⠹",
            _ => "⠸",
        };
        Paragraph::new(Line::from(vec![
            Span::styled(format!(" {dots} Indexing "), Style::default().fg(Color::Yellow).bold()),
            Span::styled(
                format!("({} items, {:.1}s) ", app.scanned_files_count, elapsed),
                Style::default().fg(Color::White),
            ),
        ]))
    } else {
        let secs = app.scan_duration.map(|d| d.as_secs_f32()).unwrap_or(0.0);
        Paragraph::new(Line::from(vec![
            Span::styled(" 󰄬 Ready ", Style::default().fg(Color::Green).bold()),
            Span::styled(format!("({} scanned in {:.2}s) ", app.scanned_files_count, secs), Style::default().fg(Color::DarkGray)),
        ]))
    }
    .alignment(Alignment::Right)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if app.scanning {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Rgb(60, 70, 95))
            }),
    );
    f.render_widget(status_widget, header_chunks[2]);
}

fn render_disks(f: &mut Frame, app: &App, area: Rect) {
    let mut disk_lines = Vec::new();

    for disk in &app.disks {
        let total = disk.total_space();
        let available = disk.available_space();
        let used = total.saturating_sub(available);
        let ratio = if total > 0 { used as f64 / total as f64 } else { 0.0 };
        let pct = ratio * 100.0;

        let mount = disk.mount_point().to_string_lossy();
        let name = disk.name().to_string_lossy();
        let fs = disk.file_system().to_string_lossy();

        let gauge_color = if pct >= 90.0 {
            Color::Rgb(255, 95, 95)
        } else if pct >= 75.0 {
            Color::Rgb(255, 185, 75)
        } else {
            Color::Rgb(100, 220, 160)
        };

        // Render sleek gradient/block meter
        let meter_width = 18;
        let filled = ((pct / 100.0) * meter_width as f64).round() as usize;
        let filled = filled.clamp(0, meter_width);
        let bar_filled = "━".repeat(filled);
        let bar_empty = "─".repeat(meter_width - filled);

        disk_lines.push(Line::from(vec![
            Span::styled("  ", Style::default().fg(Color::Cyan).bold()),
            Span::styled(format!("{:<8} ", name), Style::default().fg(Color::White).bold()),
            Span::styled(format!("({:<5}) ", fs), Style::default().fg(Color::DarkGray)),
            Span::styled("on ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{:<10} ", mount), Style::default().fg(Color::Yellow)),
            Span::styled("[", Style::default().fg(Color::DarkGray)),
            Span::styled(bar_filled, Style::default().fg(gauge_color).bold()),
            Span::styled(bar_empty, Style::default().fg(Color::Rgb(60, 65, 80))),
            Span::styled("] ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{:5.1}% ", pct), Style::default().fg(gauge_color).bold()),
            Span::styled(
                format!("({} / {})", format_size(used), format_size(total)),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }

    if disk_lines.is_empty() {
        disk_lines.push(Line::from(Span::styled(" No mounts detected", Style::default().fg(Color::DarkGray))));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(70, 80, 110)))
        .title(Span::styled("   Disks & Partitions Health ", Style::default().fg(Color::Cyan).bold()));

    let disk_widget = Paragraph::new(disk_lines).block(block);
    f.render_widget(disk_widget, area);
}

fn render_body(f: &mut Frame, app: &mut App, area: Rect) {
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    render_explorer(f, app, body_chunks[0]);
    render_analytics(f, app, body_chunks[1]);
}

fn render_explorer(f: &mut Frame, app: &mut App, area: Rect) {
    let items = app.get_filtered_sorted_children();
    let current_node = app.current_node();
    let current_total_size = current_node.map(|n| n.size).unwrap_or(1);

    let sort_label = match app.sort_order {
        SortOrder::SizeDesc => "Size 󰄼",
        SortOrder::NameAsc => "Name 󰄾",
        SortOrder::FilesDesc => "Items 󰄼",
    };

    let mut list_items = Vec::new();
    for (idx, child) in items.iter().enumerate() {
        let color = PALETTE[idx % PALETTE.len()];
        let pct = if current_total_size > 0 {
            (child.size as f64 / current_total_size as f64) * 100.0
        } else {
            0.0
        };

        let icon = if child.is_dir { " " } else { " " };

        // Relative visual proportion bar (mini bullet)
        let bullet_len = ((pct / 100.0) * 8.0).round() as usize;
        let bullet = "■".repeat(bullet_len.max(if pct > 0.5 { 1 } else { 0 }));

        let name_trimmed = if child.name.len() > 22 {
            format!("{}…", &child.name[..21])
        } else {
            format!("{:<22}", child.name)
        };

        let line = Line::from(vec![
            Span::styled(format!("{icon}"), Style::default().fg(color)),
            Span::styled(name_trimmed, Style::default().fg(if child.is_dir { Color::White } else { Color::Rgb(180, 190, 210) })),
            Span::styled(format!(" {:>9} ", format_size(child.size)), Style::default().fg(color).bold()),
            Span::styled(format!("{:>5.1}% ", pct), Style::default().fg(Color::DarkGray)),
            Span::styled(bullet, Style::default().fg(color)),
        ]);

        list_items.push(ListItem::new(line));
    }

    if list_items.is_empty() {
        let msg = if app.scanning {
            "  ⏳ Scanning directory in background..."
        } else {
            "  📭 Empty directory or no items matched"
        };
        list_items.push(ListItem::new(Line::from(Span::styled(msg, Style::default().fg(Color::DarkGray)))));
    }

    let explorer_title = format!(
        "  Explorer [{}] - Sort: [{}] ",
        format_size(current_total_size),
        sort_label
    );

    let list_widget = List::new(list_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(125, 207, 255)))
                .title(Span::styled(explorer_title, Style::default().fg(Color::Cyan).bold())),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 50, 75))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(" ❯ ");

    f.render_stateful_widget(list_widget, area, &mut app.list_state);
}

fn render_analytics(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(8)])
        .split(area);

    let items = app.get_filtered_sorted_children();
    let selected_node = app
        .list_state
        .selected()
        .and_then(|idx| items.get(idx))
        .or_else(|| items.first());

    let current_node = app.current_node();
    let current_total = current_node.map(|n| n.size).unwrap_or(1);

    // Selected Item Detail Card
    let mut details_lines = Vec::new();
    if let Some(target) = selected_node {
        let pct = if current_total > 0 {
            (target.size as f64 / current_total as f64) * 100.0
        } else {
            0.0
        };

        details_lines.push(Line::from(vec![
            Span::styled(" Name:      ", Style::default().fg(Color::DarkGray)),
            Span::styled(&target.name, Style::default().fg(Color::Yellow).bold()),
            Span::styled(if target.is_dir { " (Directory)" } else { " (File)" }, Style::default().fg(Color::DarkGray)),
        ]));
        details_lines.push(Line::from(vec![
            Span::styled(" Full Path: ", Style::default().fg(Color::DarkGray)),
            Span::styled(target.path.to_string_lossy(), Style::default().fg(Color::White)),
        ]));
        details_lines.push(Line::from(vec![
            Span::styled(" Disk Size: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format_size(target.size), Style::default().fg(Color::Green).bold()),
            Span::styled(format!("  ({:.2}% of current scope)", pct), Style::default().fg(Color::Cyan)),
        ]));
        details_lines.push(Line::from(vec![
            Span::styled(" Contains:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} subdirs, {} files", target.dir_count, target.file_count),
                Style::default().fg(Color::Rgb(180, 180, 220)),
            ),
        ]));
    } else {
        details_lines.push(Line::from(Span::styled("No item selected", Style::default().fg(Color::DarkGray))));
    }

    let detail_widget = Paragraph::new(details_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(100, 120, 160)))
            .title(Span::styled(" 󱁤 Target Inspection ", Style::default().fg(Color::Yellow).bold())),
    );
    f.render_widget(detail_widget, chunks[0]);

    // Visual Distribution Blocks / Treemap Breakdown
    let mut treemap_lines = Vec::new();
    treemap_lines.push(Line::from(vec![
        Span::styled(" Top Space Consumers ", Style::default().fg(Color::Cyan).bold()),
        Span::styled("(Proportional Heatmap Blocks):", Style::default().fg(Color::DarkGray)),
    ]));
    treemap_lines.push(Line::from(""));

    let top_items: Vec<&DiskNode> = items.iter().take(6).collect();

    if top_items.is_empty() {
        treemap_lines.push(Line::from(Span::styled("  Analyzing space...", Style::default().fg(Color::DarkGray))));
    } else {
        for (i, child) in top_items.iter().enumerate() {
            let color = PALETTE[i % PALETTE.len()];
            let pct = if current_total > 0 {
                (child.size as f64 / current_total as f64) * 100.0
            } else {
                0.0
            };

            let block_cols = ((pct / 100.0) * (area.width.saturating_sub(6) as f64)).clamp(1.0, 48.0) as usize;
            let bar_block = "█".repeat(block_cols);

            treemap_lines.push(Line::from(vec![
                Span::styled(format!(" {:<2} ", i + 1), Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{:<18}", if child.name.len() > 17 { format!("{}…", &child.name[..16]) } else { child.name.clone() }), Style::default().fg(color).bold()),
                Span::styled(format!(" {:>9} ", format_size(child.size)), Style::default().fg(Color::White)),
                Span::styled(format!(" {:>5.1}%", pct), Style::default().fg(color)),
            ]));

            treemap_lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(bar_block, Style::default().fg(color)),
            ]));
            treemap_lines.push(Line::from(""));
        }
    }

    let treemap_widget = Paragraph::new(treemap_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(180, 130, 255)))
                .title(Span::styled(" 󰄛 Space Allocation & Heatmap ", Style::default().fg(Color::Rgb(180, 130, 255)).bold())),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(treemap_widget, chunks[1]);
}

fn render_footer(f: &mut Frame, _app: &App, area: Rect) {
    let shortcuts = vec![
        Span::styled(" [j/↓] ", Style::default().fg(Color::Cyan).bold()),
        Span::styled("Down  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[k/↑] ", Style::default().fg(Color::Cyan).bold()),
        Span::styled("Up  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[l/Enter] ", Style::default().fg(Color::Cyan).bold()),
        Span::styled("Enter  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[h/Back] ", Style::default().fg(Color::Cyan).bold()),
        Span::styled("Parent  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[s] ", Style::default().fg(Color::Yellow).bold()),
        Span::styled("Sort  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[/] ", Style::default().fg(Color::Green).bold()),
        Span::styled("Filter  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[r] ", Style::default().fg(Color::Magenta).bold()),
        Span::styled("Refresh Disks  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[?] ", Style::default().fg(Color::White).bold()),
        Span::styled("Help  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[q] ", Style::default().fg(Color::LightRed).bold()),
        Span::styled("Quit", Style::default().fg(Color::DarkGray)),
    ];

    let footer = Paragraph::new(Line::from(shortcuts)).alignment(Alignment::Center);
    f.render_widget(footer, area);
}

fn render_help_modal(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(60, 60, area);
    f.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(Span::styled("⚡ Keyboard Shortcuts & Navigation", Style::default().fg(Color::Cyan).bold())),
        Line::from(""),
        Line::from(vec![
            Span::styled("  j / Down Arrow      ", Style::default().fg(Color::Yellow).bold()),
            Span::styled("Move selection down", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  k / Up Arrow        ", Style::default().fg(Color::Yellow).bold()),
            Span::styled("Move selection up", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  l / Enter           ", Style::default().fg(Color::Yellow).bold()),
            Span::styled("Drill down into directory", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  h / Backspace       ", Style::default().fg(Color::Yellow).bold()),
            Span::styled("Go up to parent directory", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  s                   ", Style::default().fg(Color::Yellow).bold()),
            Span::styled("Cycle sort: Size 󰄼 / Name 󰄾 / Item Count", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  /                   ", Style::default().fg(Color::Yellow).bold()),
            Span::styled("Live filter / search items (Esc to exit filter)", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  r                   ", Style::default().fg(Color::Yellow).bold()),
            Span::styled("Refresh disk hardware mount statistics", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  ?                   ", Style::default().fg(Color::Yellow).bold()),
            Span::styled("Toggle this modal help dialog", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  q / Ctrl+C          ", Style::default().fg(Color::LightRed).bold()),
            Span::styled("Exit application safely", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled("Press [Esc] or [?] to close this help window.", Style::default().fg(Color::DarkGray))),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled("  HELP & CHEATSHEET  ", Style::default().fg(Color::Yellow).bold()));

    let popup = Paragraph::new(help_text).block(block).alignment(Alignment::Left);
    f.render_widget(popup, popup_area);
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