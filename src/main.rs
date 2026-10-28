use clap::Parser;
use crossterm::{
    cursor,
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton,
        MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    io::{self, Stdout},
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::{Duration, Instant},
};
use sysinfo::Disks;

mod model;
use model::{format_count, format_size, DiskNode};

mod safety;
use safety::{check_deletion_permission, is_superuser, DeletionRestriction};

mod theme;
use theme::{AppConfig, IconMode, Theme, ThemeName};

mod trash;
use trash::{copy_to_clipboard, move_to_trash};

#[derive(Parser, Debug, Clone)]
#[command(
    name = "arch-disk-tui",
    author = "Emir <emirlkf@hotmail.com>",
    version = "0.2.0",
    about = "Ultra-fast, modern terminal disk space analyzer and heatmap for Linux"
)]
struct CliArgs {
    /// Path to analyze (defaults to current directory)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Stay on the same filesystem as the root path (like du -x)
    #[arg(short = 'x', long = "one-file-system")]
    one_file_system: bool,

    /// Exclude paths or directory names matching pattern (can be specified multiple times)
    #[arg(short = 'e', long = "exclude")]
    exclude: Vec<String>,

    /// Color theme: arch, catppuccin, gruvbox, mono, no_color
    #[arg(long = "theme")]
    theme: Option<String>,

    /// Icon mode: auto, nerd, ascii, none
    #[arg(long = "icons")]
    icons: Option<String>,

    /// Export scan results to JSON file (use '-' for stdout) without launching TUI
    #[arg(long = "export-json")]
    export_json: Option<PathBuf>,

    /// Export scan results to CSV file (use '-' for stdout) without launching TUI
    #[arg(long = "export-csv")]
    export_csv: Option<PathBuf>,

    /// Maximum depth when exporting to JSON or CSV
    #[arg(long = "export-depth", default_value_t = 4)]
    export_depth: usize,
}

struct ScanItem {
    path: PathBuf,
    size: u64,
    is_dir: bool,
    is_symlink: bool,
    access_denied: bool,
}

enum ScanMessage {
    Batch(Vec<ScanItem>),
    Progress {
        files: usize,
        bytes: u64,
        current_path: String,
    },
    Finished,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SortField {
    Size,
    Name,
    Count,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SortDirection {
    Desc,
    Asc,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ViewMode {
    List,
    Treemap,
    TopFiles,
}

#[derive(Clone, Debug)]
struct DeleteTarget {
    name: String,
    path: PathBuf,
    size: u64,
    is_dir: bool,
    file_count: usize,
    dir_count: usize,
    is_system: bool,
}

#[derive(Clone, Debug)]
struct DeleteConfirmDialog {
    targets: Vec<DeleteTarget>,
    total_size: u64,
    permanent: bool,
    any_system: bool,
}

#[derive(Clone, Debug)]
struct SystemProtectedDialog {
    target_name: String,
    target_path: PathBuf,
}

#[derive(Clone, Debug)]
struct MessageDialog {
    title: String,
    message: String,
    is_error: bool,
}

#[derive(Clone, Debug)]
enum Modal {
    Help,
    DeleteConfirm(DeleteConfirmDialog),
    SystemProtected(SystemProtectedDialog),
    Message(MessageDialog),
}

struct App {
    root_path: PathBuf,
    current_path: PathBuf,
    root_node: DiskNode,
    disks: Disks,
    list_state: ListState,
    sort_field: SortField,
    sort_dir: SortDirection,
    view_mode: ViewMode,
    theme: Theme,
    icon_mode: IconMode,
    one_file_system: bool,
    exclude_patterns: Vec<String>,
    scanning: bool,
    scanned_files_count: usize,
    scanned_bytes_count: u64,
    current_scanning_item: String,
    scan_start_time: Instant,
    scan_duration: Option<Duration>,
    modal: Option<Modal>,
    status_notification: Option<(String, Instant)>,
    filter_query: String,
    is_filtering: bool,
    cursor_history: HashMap<PathBuf, String>,
    marked_paths: HashSet<PathBuf>,
    last_click: Option<(Instant, usize)>,
    explorer_inner_area: Rect,
    treemap_rects: Vec<(Rect, usize)>,
    breadcrumb_targets: Vec<( u16, u16, PathBuf )>,
}

impl App {
    fn new(
        target_path: PathBuf,
        theme: Theme,
        icon_mode: IconMode,
        one_file_system: bool,
        exclude_patterns: Vec<String>,
    ) -> Self {
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
            root_node: DiskNode::new(name, abs_path, true),
            disks: Disks::new_with_refreshed_list(),
            list_state,
            sort_field: SortField::Size,
            sort_dir: SortDirection::Desc,
            view_mode: ViewMode::List,
            theme,
            icon_mode,
            one_file_system,
            exclude_patterns,
            scanning: true,
            scanned_files_count: 0,
            scanned_bytes_count: 0,
            current_scanning_item: String::from("Starting scan..."),
            scan_start_time: Instant::now(),
            scan_duration: None,
            modal: None,
            status_notification: None,
            filter_query: String::new(),
            is_filtering: false,
            cursor_history: HashMap::new(),
            marked_paths: HashSet::new(),
            last_click: None,
            explorer_inner_area: Rect::default(),
            treemap_rects: Vec::new(),
            breadcrumb_targets: Vec::new(),
        }
    }

    fn current_node(&self) -> Option<&DiskNode> {
        self.root_node.find_node(&self.current_path)
    }

    fn get_active_items(&self) -> Vec<&DiskNode> {
        if self.view_mode == ViewMode::TopFiles {
            let scope = self.current_node().unwrap_or(&self.root_node);
            let mut top = scope.collect_largest_files(200);
            if !self.filter_query.is_empty() {
                let q = self.filter_query.to_lowercase();
                top.retain(|n| n.name.to_lowercase().contains(&q));
            }
            top.truncate(100);
            return top;
        }

        let Some(node) = self.current_node() else {
            return Vec::new();
        };
        Self::filter_and_sort_node(node, &self.filter_query, self.sort_field, self.sort_dir)
    }

    /// Alias kept for unit test compatibility
    #[allow(dead_code)]
    fn get_filtered_sorted_children(&self) -> Vec<&DiskNode> {
        let Some(node) = self.current_node() else {
            return Vec::new();
        };
        Self::filter_and_sort_node(node, &self.filter_query, self.sort_field, self.sort_dir)
    }

    fn filter_and_sort_node<'a>(
        node: &'a DiskNode,
        filter_query: &str,
        sort_field: SortField,
        sort_dir: SortDirection,
    ) -> Vec<&'a DiskNode> {
        let filter_lower = if filter_query.is_empty() {
            None
        } else {
            Some(filter_query.to_lowercase())
        };

        let mut items: Vec<&DiskNode> = node
            .children
            .values()
            .filter(|child| {
                if let Some(ref query) = filter_lower {
                    child.name.to_lowercase().contains(query)
                } else {
                    true
                }
            })
            .collect();

        items.sort_by(|a, b| {
            let ord = match sort_field {
                SortField::Size => a.size.cmp(&b.size).then_with(|| a.name.cmp(&b.name)),
                SortField::Name => a
                    .name
                    .to_lowercase()
                    .cmp(&b.name.to_lowercase())
                    .then_with(|| a.size.cmp(&b.size)),
                SortField::Count => (a.file_count + a.dir_count)
                    .cmp(&(b.file_count + b.dir_count))
                    .then_with(|| a.size.cmp(&b.size)),
            };
            match sort_dir {
                SortDirection::Asc => ord,
                SortDirection::Desc => ord.reverse(),
            }
        });

        items
    }

    fn record_current_cursor(&mut self) {
        let selected_name = {
            let items = self.get_active_items();
            self.list_state
                .selected()
                .and_then(|i| items.get(i))
                .map(|n| n.name.clone())
        };
        if let Some(name) = selected_name {
            self.cursor_history.insert(self.current_path.clone(), name);
        }
    }

    fn restore_cursor_for(&mut self, dir_path: &Path, fallback_name: Option<&str>) {
        let target_name = fallback_name
            .map(|s| s.to_string())
            .or_else(|| self.cursor_history.get(dir_path).cloned());

        let items = self.get_active_items();
        if items.is_empty() {
            self.list_state.select(Some(0));
            return;
        }

        if let Some(name) = target_name {
            if let Some(idx) = items.iter().position(|n| n.name == name) {
                self.list_state.select(Some(idx));
                return;
            }
        }
        self.list_state.select(Some(0));
    }

    fn next_item(&mut self) {
        let len = self.get_active_items().len();
        if len == 0 {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => (i + 1) % len,
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn previous_item(&mut self) {
        let len = self.get_active_items().len();
        if len == 0 {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    len - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn jump_top(&mut self) {
        if !self.get_active_items().is_empty() {
            self.list_state.select(Some(0));
        }
    }

    fn jump_bottom(&mut self) {
        let len = self.get_active_items().len();
        if len > 0 {
            self.list_state.select(Some(len - 1));
        }
    }

    fn half_page_down(&mut self) {
        let len = self.get_active_items().len();
        if len == 0 {
            return;
        }
        let step = (self.explorer_inner_area.height as usize / 2).max(5);
        let cur = self.list_state.selected().unwrap_or(0);
        self.list_state.select(Some((cur + step).min(len - 1)));
    }

    fn half_page_up(&mut self) {
        let len = self.get_active_items().len();
        if len == 0 {
            return;
        }
        let step = (self.explorer_inner_area.height as usize / 2).max(5);
        let cur = self.list_state.selected().unwrap_or(0);
        self.list_state.select(Some(cur.saturating_sub(step)));
    }

    fn enter_dir(&mut self) {
        if self.view_mode == ViewMode::TopFiles {
            // In TopFiles mode, pressing Enter/l jumps to the file's parent directory and selects it
            let target_info = {
                let items = self.get_active_items();
                self.list_state
                    .selected()
                    .and_then(|idx| items.get(idx))
                    .map(|n| (n.path.parent().map(|p| p.to_path_buf()), n.name.clone()))
            };
            if let Some((Some(parent), file_name)) = target_info {
                self.view_mode = ViewMode::List;
                self.current_path = parent.clone();
                self.filter_query.clear();
                self.restore_cursor_for(&parent, Some(&file_name));
            }
            return;
        }

        let target_info = {
            let items = self.get_active_items();
            self.list_state
                .selected()
                .and_then(|selected| items.get(selected))
                .map(|t| (t.is_dir, t.path.clone(), t.access_denied))
        };

        if let Some((is_dir, target_path, access_denied)) = target_info {
            if is_dir {
                if access_denied {
                    self.status_notification = Some((
                        String::from("Permission denied — run with sudo to inspect this folder"),
                        Instant::now(),
                    ));
                }
                self.record_current_cursor();
                self.current_path = target_path.clone();
                self.filter_query.clear();
                self.restore_cursor_for(&target_path, None);
            }
        }
    }

    fn exit_dir(&mut self) {
        if self.view_mode == ViewMode::TopFiles {
            self.view_mode = ViewMode::List;
            return;
        }
        if self.current_path != self.root_path {
            let leaving_name = self
                .current_path
                .file_name()
                .map(|s| s.to_string_lossy().to_string());
            if let Some(parent) = self.current_path.parent() {
                let parent_buf = parent.to_path_buf();
                self.record_current_cursor();
                self.current_path = parent_buf.clone();
                self.filter_query.clear();
                self.restore_cursor_for(&parent_buf, leaving_name.as_deref());
            }
        }
    }

    fn jump_home(&mut self) -> Option<PathBuf> {
        let Ok(home_str) = std::env::var("HOME") else {
            return None;
        };
        let home = PathBuf::from(home_str);
        if !home.exists() {
            return None;
        }
        if home.starts_with(&self.root_path) && self.root_node.find_node(&home).is_some() {
            self.record_current_cursor();
            self.current_path = home.clone();
            self.filter_query.clear();
            self.restore_cursor_for(&home, None);
            self.status_notification = Some((
                format!("Jumped to ~ ({})", home.display()),
                Instant::now(),
            ));
            None
        } else {
            Some(home)
        }
    }

    fn cycle_sort(&mut self) {
        self.sort_field = match self.sort_field {
            SortField::Size => SortField::Name,
            SortField::Name => SortField::Count,
            SortField::Count => SortField::Size,
        };
        self.sort_dir = match self.sort_field {
            SortField::Size => SortDirection::Desc,
            SortField::Name => SortDirection::Asc,
            SortField::Count => SortDirection::Desc,
        };
        self.list_state.select(Some(0));
    }

    fn reverse_sort(&mut self) {
        self.sort_dir = match self.sort_dir {
            SortDirection::Desc => SortDirection::Asc,
            SortDirection::Asc => SortDirection::Desc,
        };
        self.list_state.select(Some(0));
    }

    fn sort_badge(&self) -> String {
        let field = match self.sort_field {
            SortField::Size => "Size",
            SortField::Name => "Name",
            SortField::Count => "Count",
        };
        let arrow = match self.sort_dir {
            SortDirection::Desc => "↓",
            SortDirection::Asc => "↑",
        };
        format!("Sort: {field} {arrow}")
    }

    fn toggle_mark_selected(&mut self) {
        let target_path = {
            let items = self.get_active_items();
            self.list_state
                .selected()
                .and_then(|i| items.get(i))
                .map(|n| n.path.clone())
        };
        if let Some(path) = target_path {
            if !self.marked_paths.remove(&path) {
                self.marked_paths.insert(path);
            }
        }
    }

    fn marked_summary(&self) -> (usize, u64) {
        let mut count = 0usize;
        let mut bytes = 0u64;
        for path in &self.marked_paths {
            if let Some(node) = self.root_node.find_node(path) {
                count += 1;
                bytes += node.size;
            }
        }
        (count, bytes)
    }

    fn copy_selected_path(&mut self) {
        let target_path = {
            let items = self.get_active_items();
            self.list_state
                .selected()
                .and_then(|i| items.get(i))
                .map(|n| n.path.to_string_lossy().to_string())
        };
        if let Some(path_str) = target_path {
            copy_to_clipboard(&path_str);
            self.status_notification = Some((
                format!("Copied path: {path_str}"),
                Instant::now(),
            ));
        }
    }

    fn request_delete(&mut self, permanent: bool) {
        let candidate_nodes: Vec<DeleteTarget> = if !self.marked_paths.is_empty() {
            let mut list = Vec::new();
            for path in &self.marked_paths {
                if let Some(n) = self.root_node.find_node(path) {
                    list.push(DeleteTarget {
                        name: n.name.clone(),
                        path: n.path.clone(),
                        size: n.size,
                        is_dir: n.is_dir,
                        file_count: n.file_count,
                        dir_count: n.dir_count,
                        is_system: false,
                    });
                }
            }
            list.sort_by(|a, b| b.size.cmp(&a.size));
            list
        } else {
            let items = self.get_active_items();
            if let Some(selected) = self.list_state.selected() {
                if let Some(target) = items.get(selected) {
                    vec![DeleteTarget {
                        name: target.name.clone(),
                        path: target.path.clone(),
                        size: target.size,
                        is_dir: target.is_dir,
                        file_count: target.file_count,
                        dir_count: target.dir_count,
                        is_system: false,
                    }]
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        };

        if candidate_nodes.is_empty() {
            return;
        }

        let mut verified = Vec::with_capacity(candidate_nodes.len());
        let mut total_size = 0u64;
        let mut any_system = false;

        for mut item in candidate_nodes {
            match check_deletion_permission(&item.path, &self.root_path) {
                Err(DeletionRestriction::RootFilesystem) => {
                    self.modal = Some(Modal::Message(MessageDialog {
                        title: "Action Forbidden".into(),
                        message: "The root directory '/' cannot be deleted under any circumstances!"
                            .into(),
                        is_error: true,
                    }));
                    return;
                }
                Err(DeletionRestriction::CurrentScanRoot) => {
                    self.modal = Some(Modal::Message(MessageDialog {
                        title: "Action Forbidden".into(),
                        message: "Cannot delete the active scan root while viewing it!".into(),
                        is_error: true,
                    }));
                    return;
                }
                Err(DeletionRestriction::SystemProtected { .. }) => {
                    self.modal = Some(Modal::SystemProtected(SystemProtectedDialog {
                        target_name: item.name,
                        target_path: item.path,
                    }));
                    return;
                }
                Ok(is_sys) => {
                    item.is_system = is_sys;
                    if is_sys {
                        any_system = true;
                    }
                    total_size += item.size;
                    verified.push(item);
                }
            }
        }

        self.modal = Some(Modal::DeleteConfirm(DeleteConfirmDialog {
            targets: verified,
            total_size,
            permanent,
            any_system,
        }));
    }

    /// Kept for unit test compatibility (defaults to permanent delete in unit test or trash)
    #[allow(dead_code)]
    fn request_delete_selected(&mut self) {
        self.request_delete(true);
    }

    fn execute_deletion(&mut self) {
        if let Some(Modal::DeleteConfirm(dialog)) = self.modal.take() {
            let mut freed_bytes = 0u64;
            let mut deleted_count = 0usize;
            let mut first_name = String::new();
            let mut last_error: Option<String> = None;

            for target in &dialog.targets {
                if first_name.is_empty() {
                    first_name = target.name.clone();
                }

                let res = if dialog.permanent {
                    if target.is_dir {
                        let is_symlink = target
                            .path
                            .symlink_metadata()
                            .map(|m| m.file_type().is_symlink())
                            .unwrap_or(false);
                        if is_symlink {
                            std::fs::remove_file(&target.path)
                        } else {
                            std::fs::remove_dir_all(&target.path)
                        }
                    } else {
                        std::fs::remove_file(&target.path)
                    }
                } else {
                    move_to_trash(&target.path).map(|_| ())
                };

                match res {
                    Ok(()) => {
                        self.root_node.remove_node(&target.path, &self.root_path);
                        self.marked_paths.remove(&target.path);
                        freed_bytes += target.size;
                        deleted_count += 1;
                    }
                    Err(err) => {
                        last_error = Some(format!(
                            "Could not delete '{}':\n{}",
                            target.path.display(),
                            err
                        ));
                    }
                }
            }

            let remaining = self.get_active_items();
            if remaining.is_empty() {
                self.list_state.select(None);
            } else if let Some(sel) = self.list_state.selected() {
                if sel >= remaining.len() {
                    self.list_state
                        .select(Some(remaining.len().saturating_sub(1)));
                }
            }

            self.disks.refresh(true);

            if let Some(err_msg) = last_error {
                self.modal = Some(Modal::Message(MessageDialog {
                    title: "Deletion Failed".into(),
                    message: err_msg,
                    is_error: true,
                }));
            } else if deleted_count > 0 {
                let action = if dialog.permanent {
                    "Permanently deleted"
                } else {
                    "Moved to Trash"
                };
                let target_desc = if deleted_count == 1 {
                    format!("'{first_name}'")
                } else {
                    format!("{deleted_count} items")
                };
                self.status_notification = Some((
                    format!(
                        "{action} {target_desc} · Freed {}",
                        format_size(freed_bytes)
                    ),
                    Instant::now(),
                ));
            }
        }
    }

    fn active_disk_usage(&self) -> Option<(String, u64, u64, f64)> {
        let mut best_match: Option<(&sysinfo::Disk, usize)> = None;
        for disk in &self.disks {
            let mount = disk.mount_point();
            if self.current_path.starts_with(mount) {
                let len = mount.as_os_str().len();
                if best_match.map_or(true, |(_, best_len)| len > best_len) {
                    best_match = Some((disk, len));
                }
            }
        }
        let (disk, _) = best_match.or_else(|| self.disks.iter().next().map(|d| (d, 0)))?;
        let total = disk.total_space();
        let avail = disk.available_space();
        let used = total.saturating_sub(avail);
        let pct = if total > 0 {
            (used as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        let mount_str = disk.mount_point().to_string_lossy().to_string();
        Some((mount_str, used, total, pct))
    }
}

fn matches_exclude(path: &Path, name: &str, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return false;
    }
    let path_str = path.to_string_lossy();
    for pat in patterns {
        if pat.is_empty() {
            continue;
        }
        if name == pat || path_str.contains(pat.as_str()) {
            return true;
        }
        if let Some(stripped) = pat.strip_prefix("*.") {
            if name.ends_with(&format!(".{stripped}")) {
                return true;
            }
        }
    }
    false
}

fn start_scanner(
    root: PathBuf,
    one_file_system: bool,
    exclude_patterns: Vec<String>,
    tx: mpsc::Sender<ScanMessage>,
) {
    thread::spawn(move || {
        let is_root_fs = root == PathBuf::from("/");
        #[cfg(unix)]
        let root_dev = {
            use std::os::unix::fs::MetadataExt;
            std::fs::metadata(&root).map(|m| m.dev()).ok()
        };

        let exclude_for_closure = exclude_patterns.clone();
        let walker = jwalk::WalkDir::new(&root)
            .follow_links(false)
            .skip_hidden(false)
            .process_read_dir(move |_depth, _path, _state, children| {
                children.retain(|dir_entry_result| {
                    let Ok(dir_entry) = dir_entry_result else {
                        return false;
                    };
                    let path = dir_entry.path();
                    let path_str = path.to_string_lossy();

                    if is_root_fs
                        && (path_str.starts_with("/proc")
                            || path_str.starts_with("/sys")
                            || path_str.starts_with("/dev")
                            || path_str.starts_with("/run"))
                    {
                        return false;
                    }

                    let file_name = dir_entry.file_name.to_string_lossy();
                    if matches_exclude(&path, &file_name, &exclude_for_closure) {
                        return false;
                    }

                    #[cfg(unix)]
                    if one_file_system {
                        if let Some(rdev) = root_dev {
                            use std::os::unix::fs::MetadataExt;
                            if let Ok(meta) = dir_entry.metadata() {
                                if meta.dev() != rdev {
                                    return false;
                                }
                            }
                        }
                    }

                    true
                });
            });

        let mut count = 0usize;
        let mut total_bytes = 0u64;
        let mut batch = Vec::with_capacity(512);
        let mut last_progress = Instant::now();
        #[cfg(unix)]
        let mut seen_inodes: HashSet<(u64, u64)> = HashSet::new();

        for entry in walker {
            let Ok(entry) = entry else {
                continue;
            };

            let path = entry.path();
            if path == root {
                continue;
            }

            if is_root_fs {
                let path_str = path.to_string_lossy();
                if path_str.starts_with("/proc")
                    || path_str.starts_with("/sys")
                    || path_str.starts_with("/dev")
                    || path_str.starts_with("/run")
                {
                    continue;
                }
            }

            let is_symlink = entry.file_type.is_symlink();
            let is_dir = entry.file_type.is_dir();
            let access_denied = if is_dir && !is_symlink {
                std::fs::read_dir(&path).is_err()
            } else {
                false
            };

            let mut size = 0u64;
            if !is_dir && !is_symlink {
                if let Ok(meta) = entry.metadata() {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::MetadataExt;
                        if meta.nlink() > 1 {
                            let key = (meta.dev(), meta.ino());
                            if seen_inodes.insert(key) {
                                size = meta.len();
                            }
                        } else {
                            size = meta.len();
                        }
                    }
                    #[cfg(not(unix))]
                    {
                        size = meta.len();
                    }
                }
            }

            count += 1;
            total_bytes += size;
            batch.push(ScanItem {
                path,
                size,
                is_dir,
                is_symlink,
                access_denied,
            });

            if batch.len() >= 512 {
                let to_send = std::mem::replace(&mut batch, Vec::with_capacity(512));
                if tx.send(ScanMessage::Batch(to_send)).is_err() {
                    return;
                }
            }

            if count % 1000 == 0 && last_progress.elapsed() >= Duration::from_millis(80) {
                last_progress = Instant::now();
                let current_path = entry.path().to_string_lossy().to_string();
                if tx
                    .send(ScanMessage::Progress {
                        files: count,
                        bytes: total_bytes,
                        current_path,
                    })
                    .is_err()
                {
                    return;
                }
            }
        }

        if !batch.is_empty() {
            let _ = tx.send(ScanMessage::Batch(batch));
        }

        let _ = tx.send(ScanMessage::Progress {
            files: count,
            bytes: total_bytes,
            current_path: String::new(),
        });

        let _ = tx.send(ScanMessage::Finished);
    });
}

fn run_headless_export(
    root_path: PathBuf,
    one_file_system: bool,
    exclude: Vec<String>,
    json_out: Option<PathBuf>,
    csv_out: Option<PathBuf>,
    max_depth: usize,
) -> Result<(), Box<dyn Error>> {
    let abs_path = std::fs::canonicalize(&root_path).unwrap_or(root_path);
    let name = abs_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| abs_path.to_string_lossy().to_string());
    let mut root_node = DiskNode::new(name, abs_path.clone(), true);

    let (tx, rx) = mpsc::channel();
    start_scanner(abs_path.clone(), one_file_system, exclude, tx);

    while let Ok(msg) = rx.recv() {
        match msg {
            ScanMessage::Batch(batch) => {
                for item in batch {
                    root_node.insert_ext(
                        &item.path,
                        item.size,
                        item.is_dir,
                        item.is_symlink,
                        item.access_denied,
                        &abs_path,
                    );
                }
            }
            ScanMessage::Progress { .. } => {}
            ScanMessage::Finished => break,
        }
    }

    if let Some(json_target) = json_out {
        let payload = root_node.to_json(max_depth);
        if json_target.as_os_str() == "-" {
            print!("{payload}");
        } else {
            std::fs::write(&json_target, payload)?;
            eprintln!("Exported JSON scan results to {}", json_target.display());
        }
    }

    if let Some(csv_target) = csv_out {
        let payload = root_node.to_csv(max_depth);
        if csv_target.as_os_str() == "-" {
            print!("{payload}");
        } else {
            std::fs::write(&csv_target, payload)?;
            eprintln!("Exported CSV scan results to {}", csv_target.display());
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = CliArgs::parse();
    let mut cfg = AppConfig::load();

    if let Some(ref t) = args.theme {
        cfg.theme = ThemeName::from_str(t);
    }
    if let Some(ref ic) = args.icons {
        cfg.icons = IconMode::from_str(ic);
    }
    if args.one_file_system {
        cfg.one_file_system = true;
    }
    if !args.exclude.is_empty() {
        cfg.exclude.extend(args.exclude.clone());
    }

    if args.export_json.is_some() || args.export_csv.is_some() {
        return run_headless_export(
            args.path,
            cfg.one_file_system,
            cfg.exclude,
            args.export_json,
            args.export_csv,
            args.export_depth,
        );
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture, cursor::Hide)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let theme = Theme::from_name(cfg.theme);
    let mut app = App::new(
        args.path,
        theme,
        cfg.icons,
        cfg.one_file_system,
        cfg.exclude.clone(),
    );
    let (tx, rx): (mpsc::Sender<ScanMessage>, Receiver<ScanMessage>) = mpsc::channel();
    start_scanner(
        app.root_path.clone(),
        app.one_file_system,
        app.exclude_patterns.clone(),
        tx,
    );

    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(50);

    let res = run_app(&mut terminal, &mut app, rx, tick_rate, &mut last_tick);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        cursor::Show
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Application Error: {err:?}");
    }

    Ok(())
}

fn open_selected_external(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn Error>> {
    let target = {
        let items = app.get_active_items();
        app.list_state
            .selected()
            .and_then(|i| items.get(i))
            .map(|n| (n.path.clone(), n.is_dir))
    };
    let Some((path, is_dir)) = target else {
        return Ok(());
    };

    if !is_dir {
        if let Ok(editor) = std::env::var("EDITOR") {
            if !editor.trim().is_empty() {
                disable_raw_mode()?;
                execute!(
                    terminal.backend_mut(),
                    LeaveAlternateScreen,
                    DisableMouseCapture,
                    cursor::Show
                )?;
                let status = Command::new(editor).arg(&path).status();
                enable_raw_mode()?;
                execute!(
                    terminal.backend_mut(),
                    EnterAlternateScreen,
                    EnableMouseCapture,
                    cursor::Hide
                )?;
                terminal.clear()?;
                if status.is_ok() {
                    app.status_notification = Some((
                        format!("Opened '{}' in $EDITOR", path.display()),
                        Instant::now(),
                    ));
                    return Ok(());
                }
            }
        }
    }

    let res = Command::new("xdg-open")
        .arg(&path)
        .stdin( std::process::Stdio::null() )
        .stdout( std::process::Stdio::null() )
        .stderr( std::process::Stdio::null() )
        .spawn();

    match res {
        Ok(_) => {
            app.status_notification = Some((
                format!("Opened '{}'", path.display()),
                Instant::now(),
            ));
        }
        Err(e) => {
            app.status_notification = Some((
                format!("Failed to open: {e}"),
                Instant::now(),
            ));
        }
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    mut rx: Receiver<ScanMessage>,
    tick_rate: Duration,
    last_tick: &mut Instant,
) -> Result<(), Box<dyn Error>> {
    let mut last_draw = Instant::now();
    let min_draw_interval = Duration::from_millis(30);

    loop {
        let mut had_updates = false;
        let drain_start = Instant::now();

        while drain_start.elapsed() < Duration::from_millis(15) {
            match rx.try_recv() {
                Ok(ScanMessage::Batch(batch)) => {
                    for item in batch {
                        app.root_node.insert_ext(
                            &item.path,
                            item.size,
                            item.is_dir,
                            item.is_symlink,
                            item.access_denied,
                            &app.root_path,
                        );
                    }
                    app.scanned_bytes_count = app.root_node.size;
                    had_updates = true;
                }
                Ok(ScanMessage::Progress {
                    files,
                    bytes,
                    current_path,
                }) => {
                    app.scanned_files_count = files;
                    app.scanned_bytes_count = bytes.max(app.root_node.size);
                    if !current_path.is_empty() {
                        app.current_scanning_item = current_path;
                    }
                    had_updates = true;
                }
                Ok(ScanMessage::Finished) => {
                    app.scanning = false;
                    app.scanned_bytes_count = app.root_node.size;
                    app.scan_duration = Some(app.scan_start_time.elapsed());
                    had_updates = true;
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    app.scanning = false;
                    break;
                }
            }
        }

        let now = Instant::now();
        if had_updates || now.duration_since(last_draw) >= tick_rate {
            if now.duration_since(last_draw) >= min_draw_interval {
                terminal.draw(|f| render_ui(f, app))?;
                last_draw = Instant::now();
            }
        }

        let timeout = if app.scanning {
            Duration::from_millis(10)
        } else {
            tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or(Duration::from_millis(10))
        };

        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    if let Some(modal) = app.modal.clone() {
                        match modal {
                            Modal::Help => match key.code {
                                KeyCode::Esc
                                | KeyCode::Char('?')
                                | KeyCode::Char('q')
                                | KeyCode::Enter => {
                                    app.modal = None;
                                }
                                _ => {}
                            },
                            Modal::DeleteConfirm(_) => match key.code {
                                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                                    app.execute_deletion();
                                }
                                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                                    app.modal = None;
                                }
                                _ => {}
                            },
                            Modal::SystemProtected(_) => match key.code {
                                KeyCode::Esc
                                | KeyCode::Enter
                                | KeyCode::Char('q')
                                | KeyCode::Char(' ') => {
                                    app.modal = None;
                                }
                                _ => {}
                            },
                            Modal::Message(_) => match key.code {
                                KeyCode::Esc
                                | KeyCode::Enter
                                | KeyCode::Char('q')
                                | KeyCode::Char(' ') => {
                                    app.modal = None;
                                }
                                _ => {}
                            },
                        }
                    } else if app.is_filtering {
                        match key.code {
                            KeyCode::Esc => {
                                app.filter_query.clear();
                                app.is_filtering = false;
                                app.list_state.select(Some(0));
                            }
                            KeyCode::Enter => {
                                app.is_filtering = false;
                            }
                            KeyCode::Backspace => {
                                app.filter_query.pop();
                                app.list_state.select(Some(0));
                            }
                            KeyCode::Down => app.next_item(),
                            KeyCode::Up => app.previous_item(),
                            KeyCode::Char(c) => {
                                app.filter_query.push(c);
                                app.list_state.select(Some(0));
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') => return Ok(()),
                            KeyCode::Char('c')
                                if key.modifiers.contains(KeyModifiers::CONTROL) =>
                            {
                                return Ok(())
                            }
                            KeyCode::Char('d')
                                if key.modifiers.contains(KeyModifiers::CONTROL) =>
                            {
                                app.half_page_down();
                            }
                            KeyCode::Char('u')
                                if key.modifiers.contains(KeyModifiers::CONTROL) =>
                            {
                                app.half_page_up();
                            }
                            KeyCode::Char('?') => app.modal = Some(Modal::Help),
                            KeyCode::Char('/') => {
                                app.is_filtering = true;
                            }
                            KeyCode::Char('s') => app.cycle_sort(),
                            KeyCode::Char('S') => app.reverse_sort(),
                            KeyCode::Char('t') => {
                                app.view_mode = match app.view_mode {
                                    ViewMode::Treemap => ViewMode::List,
                                    _ => ViewMode::Treemap,
                                };
                            }
                            KeyCode::Char('T') => {
                                let next_name = app.theme.name.next();
                                app.theme = Theme::from_name(next_name);
                                app.status_notification = Some((
                                    format!("Theme: {}", app.theme.name.label()),
                                    Instant::now(),
                                ));
                            }
                            KeyCode::Char('f') => {
                                app.view_mode = match app.view_mode {
                                    ViewMode::TopFiles => ViewMode::List,
                                    _ => ViewMode::TopFiles,
                                };
                                app.list_state.select(Some(0));
                            }
                            KeyCode::Char('g') | KeyCode::Home => app.jump_top(),
                            KeyCode::Char('G') | KeyCode::End => app.jump_bottom(),
                            KeyCode::PageDown => app.half_page_down(),
                            KeyCode::PageUp => app.half_page_up(),
                            KeyCode::Char('~') => {
                                if let Some(new_home_root) = app.jump_home() {
                                    let name = new_home_root
                                        .file_name()
                                        .map(|s| s.to_string_lossy().to_string())
                                        .unwrap_or_else(|| "~".into());
                                    app.root_path = new_home_root.clone();
                                    app.current_path = new_home_root.clone();
                                    app.root_node = DiskNode::new(name, new_home_root, true);
                                    app.marked_paths.clear();
                                    app.scanning = true;
                                    app.scanned_files_count = 0;
                                    app.scanned_bytes_count = 0;
                                    app.scan_start_time = Instant::now();
                                    let (new_tx, new_rx) = mpsc::channel();
                                    rx = new_rx;
                                    start_scanner(
                                        app.root_path.clone(),
                                        app.one_file_system,
                                        app.exclude_patterns.clone(),
                                        new_tx,
                                    );
                                }
                            }
                            KeyCode::Char(' ') => app.toggle_mark_selected(),
                            KeyCode::Char('y') => app.copy_selected_path(),
                            KeyCode::Char('o') => {
                                open_selected_external(terminal, app)?;
                            }
                            KeyCode::Char('r') => {
                                app.disks.refresh(true);
                                app.status_notification = Some((
                                    String::from("Refreshed partition metrics"),
                                    Instant::now(),
                                ));
                            }
                            KeyCode::Char('d') => {
                                app.request_delete(false);
                            }
                            KeyCode::Char('D') | KeyCode::Delete => {
                                app.request_delete(true);
                            }
                            KeyCode::Down | KeyCode::Char('j') => app.next_item(),
                            KeyCode::Up | KeyCode::Char('k') => app.previous_item(),
                            KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => {
                                app.enter_dir();
                            }
                            KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace => {
                                app.exit_dir();
                            }
                            KeyCode::Esc => {
                                if !app.filter_query.is_empty() {
                                    app.filter_query.clear();
                                    app.list_state.select(Some(0));
                                } else if !app.marked_paths.is_empty() {
                                    app.marked_paths.clear();
                                } else if app.view_mode != ViewMode::List {
                                    app.view_mode = ViewMode::List;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    handle_mouse_event(app, mouse);
                }
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            *last_tick = Instant::now();
        }
    }
}

fn handle_mouse_event(app: &mut App, mouse: MouseEvent) {
    if app.modal.is_some() {
        return;
    }

    match mouse.kind {
        MouseEventKind::ScrollDown => {
            app.next_item();
        }
        MouseEventKind::ScrollUp => {
            app.previous_item();
        }
        MouseEventKind::Down(MouseButton::Left) => {
            let col = mouse.column;
            let row = mouse.row;

            // Check breadcrumb clicks in header
            if row == 1 {
                for (x_start, x_end, target_path) in app.breadcrumb_targets.clone() {
                    if col >= x_start && col < x_end {
                        if target_path != app.current_path {
                            app.record_current_cursor();
                            app.current_path = target_path.clone();
                            app.filter_query.clear();
                            app.restore_cursor_for(&target_path, None);
                        }
                        return;
                    }
                }
            }

            let inner = app.explorer_inner_area;
            if col >= inner.x
                && col < inner.x + inner.width
                && row >= inner.y
                && row < inner.y + inner.height
            {
                if app.view_mode == ViewMode::Treemap {
                    for &(rect, item_idx) in &app.treemap_rects {
                        if col >= rect.x
                            && col < rect.x + rect.width
                            && row >= rect.y
                            && row < rect.y + rect.height
                        {
                            let is_double = app
                                .last_click
                                .map_or(false, |(t, prev)| prev == item_idx && t.elapsed() < Duration::from_millis(400));
                            app.list_state.select(Some(item_idx));
                            if is_double {
                                app.enter_dir();
                                app.last_click = None;
                            } else {
                                app.last_click = Some((Instant::now(), item_idx));
                            }
                            return;
                        }
                    }
                } else {
                    let offset = app.list_state.offset();
                    let clicked_idx = offset + (row - inner.y) as usize;
                    let len = app.get_active_items().len();
                    if clicked_idx < len {
                        let is_double = app
                            .last_click
                            .map_or(false, |(t, prev)| prev == clicked_idx && t.elapsed() < Duration::from_millis(400));
                        app.list_state.select(Some(clicked_idx));
                        if is_double {
                            app.enter_dir();
                            app.last_click = None;
                        } else {
                            app.last_click = Some((Instant::now(), clicked_idx));
                        }
                    }
                }
            }
        }
        MouseEventKind::Down(MouseButton::Right) => {
            app.exit_dir();
        }
        _ => {}
    }
}

fn render_ui(f: &mut Frame, app: &mut App) {
    let size = f.area();

    let bg_block = Block::default().style(Style::default().bg(app.theme.bg));
    f.render_widget(bg_block, size);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header: Breadcrumb path + Partition usage gauge + Sort & Progress
            Constraint::Length(5), // Drives & System Storage Health
            Constraint::Min(10),   // Content: Explorer / Treemap / TopFiles + Details & Heatmap
            Constraint::Length(1), // Context-aware 1-line Footer
        ])
        .split(size);

    render_header(f, app, chunks[0]);
    render_disks(f, app, chunks[1]);
    render_body(f, app, chunks[2]);
    render_footer(f, app, chunks[3]);

    if let Some(ref modal) = app.modal {
        match modal {
            Modal::Help => render_help_modal(f, app, size),
            Modal::DeleteConfirm(dialog) => render_delete_confirm_modal(f, app, dialog, size),
            Modal::SystemProtected(dialog) => render_system_protected_modal(f, app, dialog, size),
            Modal::Message(dialog) => render_message_modal(f, app, dialog, size),
        }
    }
}

fn render_header(f: &mut Frame, app: &mut App, area: Rect) {
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(44), // Logo + Active Partition Usage Gauge
            Constraint::Min(24),    // Breadcrumb Path Bar + Search Prompt
            Constraint::Length(36), // Sort indicator + Live Scan Progress
        ])
        .split(area);

    let nerd = app.icon_mode.use_nerd_icons();
    let arch_icon = if nerd { " 󰣇 " } else { " " };
    let mut left_spans = vec![
        Span::styled(arch_icon, Style::default().fg(app.theme.accent).bold()),
        Span::styled("ARCH·DISK", Style::default().fg(app.theme.text).bold()),
    ];
    if is_superuser() {
        left_spans.push(Span::styled(
            " [ROOT]",
            Style::default().fg(app.theme.gauge_crit).bold(),
        ));
    }

    if let Some((_mount, used, total, pct)) = app.active_disk_usage() {
        let gauge_color = if pct >= 90.0 {
            app.theme.gauge_crit
        } else if pct >= 75.0 {
            app.theme.gauge_warn
        } else {
            app.theme.gauge_ok
        };
        let bar_w = 8usize;
        let filled = ((pct / 100.0) * bar_w as f64).round().clamp(0.0, bar_w as f64) as usize;
        left_spans.push(Span::styled("  [", Style::default().fg(app.theme.text_dim)));
        left_spans.push(Span::styled(
            "━".repeat(filled),
            Style::default().fg(gauge_color).bold(),
        ));
        left_spans.push(Span::styled(
            "─".repeat(bar_w.saturating_sub(filled)),
            Style::default().fg(app.theme.bar_empty),
        ));
        left_spans.push(Span::styled("] ", Style::default().fg(app.theme.text_dim)));
        left_spans.push(Span::styled(
            format!("{} / {} · {:.0}%", format_size(used), format_size(total), pct),
            Style::default().fg(gauge_color).bold(),
        ));
    }

    let left_block = Paragraph::new(Line::from(left_spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if is_superuser() {
                Style::default().fg(app.theme.gauge_crit)
            } else {
                Style::default().fg(app.theme.border)
            }),
    );
    f.render_widget(left_block, header_chunks[0]);

    // Middle: Breadcrumb path bar (/ › home › emir › Downloads) + visible Search prompt + Toast
    app.breadcrumb_targets.clear();
    let mut mid_spans = Vec::new();
    let mut cur_x = header_chunks[1].x + 2;
    mid_spans.push(Span::raw(" "));

    let components: Vec<_> = app.current_path.components().collect();
    let mut acc_path = PathBuf::new();
    for (idx, comp) in components.iter().enumerate() {
        acc_path.push(comp.as_os_str());
        let label = if idx == 0 && comp.as_os_str() == "/" {
            "/".to_string()
        } else {
            comp.as_os_str().to_string_lossy().to_string()
        };
        let is_last = idx + 1 == components.len();
        let style = if is_last {
            Style::default().fg(app.theme.accent).bold()
        } else {
            Style::default().fg(app.theme.text)
        };
        let start_x = cur_x;
        let width = label.chars().count() as u16;
        cur_x += width;
        app.breadcrumb_targets
            .push((start_x, cur_x, acc_path.clone()));
        mid_spans.push(Span::styled(label, style));

        if !is_last {
            mid_spans.push(Span::styled(" › ", Style::default().fg(app.theme.text_dim)));
            cur_x += 3;
        }
    }

    if app.is_filtering || !app.filter_query.is_empty() {
        let match_count = app.get_active_items().len();
        mid_spans.push(Span::styled("   / ", Style::default().fg(app.theme.accent).bold()));
        mid_spans.push(Span::styled(
            app.filter_query.clone(),
            Style::default().fg(app.theme.text).bold(),
        ));
        if app.is_filtering {
            mid_spans.push(Span::styled(
                "_",
                Style::default().fg(app.theme.accent).bold(),
            ));
        }
        mid_spans.push(Span::styled(
            format!(" ({match_count} matches)"),
            Style::default().fg(app.theme.gauge_warn),
        ));
    } else if let Some((ref msg, time)) = app.status_notification {
        if time.elapsed() < Duration::from_secs(4) {
            mid_spans.push(Span::styled("  │ ", Style::default().fg(app.theme.text_dim)));
            mid_spans.push(Span::styled(
                msg.clone(),
                Style::default().fg(app.theme.gauge_ok).bold(),
            ));
        }
    }

    let path_widget = Paragraph::new(Line::from(mid_spans))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(if app.is_filtering {
                    Style::default().fg(app.theme.accent)
                } else {
                    Style::default().fg(app.theme.border)
                }),
        )
        .alignment(Alignment::Left);
    f.render_widget(path_widget, header_chunks[1]);

    // Right: Sort indicator (Sort: Size ↓) + Scan spinner (files scanned · size so far)
    let sort_badge = app.sort_badge();
    let status_line = if app.scanning {
        let spinner = match (app.scan_start_time.elapsed().as_millis() / 120) % 6 {
            0 => "⠋",
            1 => "⠙",
            2 => "⠹",
            3 => "⠸",
            4 => "⠼",
            _ => "⠴",
        };
        Line::from(vec![
            Span::styled(
                format!(" {} ", sort_badge),
                Style::default().fg(app.theme.accent).bold(),
            ),
            Span::styled("│ ", Style::default().fg(app.theme.text_dim)),
            Span::styled(
                format!("{spinner} "),
                Style::default().fg(app.theme.gauge_warn).bold(),
            ),
            Span::styled(
                format!(
                    "{} · {} ",
                    format_count(app.scanned_files_count),
                    format_size(app.scanned_bytes_count)
                ),
                Style::default().fg(app.theme.text),
            ),
        ])
    } else {
        let secs = app.scan_duration.map(|d| d.as_secs_f32()).unwrap_or(0.0);
        Line::from(vec![
            Span::styled(
                format!(" {} ", sort_badge),
                Style::default().fg(app.theme.accent).bold(),
            ),
            Span::styled("│ ", Style::default().fg(app.theme.text_dim)),
            Span::styled("✓ ", Style::default().fg(app.theme.gauge_ok).bold()),
            Span::styled(
                format!("{} ({:.2}s) ", format_count(app.scanned_files_count), secs),
                Style::default().fg(app.theme.text_dim),
            ),
        ])
    };

    let status_widget = Paragraph::new(status_line)
        .alignment(Alignment::Right)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(if app.scanning {
                    Style::default().fg(app.theme.gauge_warn)
                } else {
                    Style::default().fg(app.theme.border)
                }),
        );
    f.render_widget(status_widget, header_chunks[2]);
}

fn render_disks(f: &mut Frame, app: &App, area: Rect) {
    let mut disk_lines = Vec::new();
    let nerd = app.icon_mode.use_nerd_icons();

    for disk in &app.disks {
        let total = disk.total_space();
        let available = disk.available_space();
        let used = total.saturating_sub(available);
        let ratio = if total > 0 {
            used as f64 / total as f64
        } else {
            0.0
        };
        let pct = ratio * 100.0;

        let mount = disk.mount_point().to_string_lossy();
        let name = disk.name().to_string_lossy();
        let fs = disk.file_system().to_string_lossy();

        let gauge_color = if pct >= 90.0 {
            app.theme.gauge_crit
        } else if pct >= 75.0 {
            app.theme.gauge_warn
        } else {
            app.theme.gauge_ok
        };

        let meter_width = 18;
        let filled = ((pct / 100.0) * meter_width as f64).round() as usize;
        let filled = filled.clamp(0, meter_width);
        let bar_filled = "━".repeat(filled);
        let bar_empty = "─".repeat(meter_width - filled);
        let disk_icon = if nerd { "  " } else { " * " };

        disk_lines.push(Line::from(vec![
            Span::styled(disk_icon, Style::default().fg(app.theme.accent).bold()),
            Span::styled(
                format!("{:<10} ", name),
                Style::default().fg(app.theme.text).bold(),
            ),
            Span::styled(
                format!("({:<5}) ", fs),
                Style::default().fg(app.theme.text_dim),
            ),
            Span::styled("on ", Style::default().fg(app.theme.text_dim)),
            Span::styled(
                format!("{:<12} ", mount),
                Style::default().fg(app.theme.gauge_warn),
            ),
            Span::styled("[", Style::default().fg(app.theme.text_dim)),
            Span::styled(bar_filled, Style::default().fg(gauge_color).bold()),
            Span::styled(bar_empty, Style::default().fg(app.theme.bar_empty)),
            Span::styled("] ", Style::default().fg(app.theme.text_dim)),
            Span::styled(
                format!(
                    "{} / {} · {:4.1}%",
                    format_size(used),
                    format_size(total),
                    pct
                ),
                Style::default().fg(gauge_color).bold(),
            ),
        ]));
    }

    if disk_lines.is_empty() {
        disk_lines.push(Line::from(Span::styled(
            " No mounts detected",
            Style::default().fg(app.theme.text_dim),
        )));
    }

    let title_icon = if nerd { "  " } else { " " };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.theme.border))
        .title(Span::styled(
            format!("{title_icon}Partitions & Mount Telemetry "),
            Style::default().fg(app.theme.accent).bold(),
        ));

    let disk_widget = Paragraph::new(disk_lines).block(block);
    f.render_widget(disk_widget, area);
}

fn render_body(f: &mut Frame, app: &mut App, area: Rect) {
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(56), Constraint::Percentage(44)])
        .split(area);

    let current_node_info = app
        .current_node()
        .map(|n| (n.size.max(1), n.access_denied));
    let (current_total_size, current_access_denied) = current_node_info.unwrap_or((1, false));

    let items: Vec<DiskNode> = app.get_active_items().into_iter().cloned().collect();
    let item_refs: Vec<&DiskNode> = items.iter().collect();

    app.explorer_inner_area = Rect {
        x: body_chunks[0].x + 1,
        y: body_chunks[0].y + 1,
        width: body_chunks[0].width.saturating_sub(2),
        height: body_chunks[0].height.saturating_sub(2),
    };

    match app.view_mode {
        ViewMode::List | ViewMode::TopFiles => {
            render_explorer(
                f,
                app,
                current_total_size,
                current_access_denied,
                &item_refs,
                body_chunks[0],
            );
        }
        ViewMode::Treemap => {
            render_treemap_view(
                f,
                app,
                current_total_size,
                &item_refs,
                body_chunks[0],
            );
        }
    }

    let selected_idx = app.list_state.selected();
    render_analytics(
        f,
        app,
        selected_idx,
        current_total_size,
        &item_refs,
        body_chunks[1],
    );
}

fn render_inline_bar(pct: f64, width: usize, color: Color, empty_color: Color) -> Vec<Span<'static>> {
    const BLOCKS: [&str; 8] = ["▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"];
    let clamped = pct.clamp(0.0, 100.0) / 100.0;
    let total_eighths = (clamped * (width * 8) as f64).round() as usize;
    let full_cells = total_eighths / 8;
    let rem = total_eighths % 8;

    let mut filled = "█".repeat(full_cells.min(width));
    let mut used_cells = full_cells.min(width);
    if used_cells < width && rem > 0 {
        filled.push_str(BLOCKS[rem - 1]);
        used_cells += 1;
    } else if used_cells == 0 && pct > 0.1 {
        filled.push_str("▏");
        used_cells = 1;
    }
    let empty = "░".repeat(width.saturating_sub(used_cells));

    vec![
        Span::styled(filled, Style::default().fg(color)),
        Span::styled(empty, Style::default().fg(empty_color)),
    ]
}

fn highlight_search_spans(
    text: &str,
    query: &str,
    normal_style: Style,
    match_style: Style,
) -> Vec<Span<'static>> {
    if query.is_empty() {
        return vec![Span::styled(text.to_string(), normal_style)];
    }
    let lower_text = text.to_lowercase();
    let lower_query = query.to_lowercase();
    if let Some(byte_idx) = lower_text.find(&lower_query) {
        let end_idx = (byte_idx + lower_query.len()).min(text.len());
        if text.is_char_boundary(byte_idx) && text.is_char_boundary(end_idx) {
            let before = text[..byte_idx].to_string();
            let matched = text[byte_idx..end_idx].to_string();
            let after = text[end_idx..].to_string();
            let mut spans = Vec::with_capacity(3);
            if !before.is_empty() {
                spans.push(Span::styled(before, normal_style));
            }
            spans.push(Span::styled(matched, match_style));
            if !after.is_empty() {
                spans.push(Span::styled(after, normal_style));
            }
            return spans;
        }
    }
    vec![Span::styled(text.to_string(), normal_style)]
}

fn render_explorer(
    f: &mut Frame,
    app: &mut App,
    current_total_size: u64,
    current_access_denied: bool,
    items: &[&DiskNode],
    area: Rect,
) {
    let mut list_items = Vec::with_capacity(items.len().min(200));
    let bar_width = 10usize;
    let max_name_cols = (area.width as usize).saturating_sub(36).max(14);
    let top_denom = if app.view_mode == ViewMode::TopFiles {
        items.first().map(|n| n.size.max(1)).unwrap_or(1)
    } else {
        current_total_size.max(1)
    };

    for child in items {
        let cat = child.category();
        let color = cat.color(&app.theme);
        let pct = if top_denom > 0 {
            (child.size as f64 / top_denom as f64) * 100.0
        } else {
            0.0
        };
        let dir_share_pct = if current_total_size > 0 {
            (child.size as f64 / current_total_size as f64) * 100.0
        } else {
            0.0
        };

        let is_marked = app.marked_paths.contains(&child.path);
        let mark_span = if is_marked {
            Span::styled("● ", Style::default().fg(app.theme.gauge_warn).bold())
        } else {
            Span::raw("  ")
        };

        let icon = cat.icon(
            app.icon_mode,
            child.is_dir,
            child.is_symlink,
            child.access_denied,
        );

        let display_name = if app.view_mode == ViewMode::TopFiles {
            if let Ok(rel) = child.path.strip_prefix(&app.current_path) {
                rel.to_string_lossy().to_string()
            } else {
                child.name.clone()
            }
        } else {
            child.name.clone()
        };

        let char_count = display_name.chars().count();
        let name_trimmed = if char_count > max_name_cols {
            let truncated: String = display_name
                .chars()
                .take(max_name_cols.saturating_sub(1))
                .collect();
            format!("{truncated}…")
        } else {
            format!("{:<width$}", display_name, width = max_name_cols)
        };

        let name_style = if child.access_denied {
            Style::default().fg(app.theme.gauge_crit)
        } else if child.is_dir {
            Style::default().fg(app.theme.text).bold()
        } else {
            Style::default().fg(color)
        };
        let match_style = Style::default()
            .fg(app.theme.match_fg)
            .bg(app.theme.match_bg)
            .bold();

        let mut row_spans = Vec::with_capacity(12);
        row_spans.push(mark_span);
        row_spans.extend(render_inline_bar(
            pct,
            bar_width,
            color,
            app.theme.bar_empty,
        ));
        row_spans.push(Span::styled(
            format!(" {:>5.1}% ", dir_share_pct),
            Style::default().fg(app.theme.text_dim),
        ));
        row_spans.push(Span::styled(
            format!("{:>9}  ", format_size(child.size)),
            Style::default().fg(color).bold(),
        ));
        row_spans.push(Span::styled(icon, Style::default().fg(color)));
        row_spans.extend(highlight_search_spans(
            &name_trimmed,
            &app.filter_query,
            name_style,
            match_style,
        ));

        if child.access_denied {
            row_spans.push(Span::styled(
                " [needs sudo]",
                Style::default().fg(app.theme.gauge_crit).bold(),
            ));
        }

        list_items.push(ListItem::new(Line::from(row_spans)));
    }

    if list_items.is_empty() {
        let msg = if app.scanning {
            "  ⠋ Scanning directory in background..."
        } else if current_access_denied {
            "  ! Permission denied — re-run with sudo to inspect this folder"
        } else if !app.filter_query.is_empty() {
            "  No items matched search query (press Esc to clear)"
        } else {
            "  Empty folder (0 B) — press h or Backspace to go back"
        };
        let msg_color = if current_access_denied {
            app.theme.gauge_crit
        } else {
            app.theme.text_dim
        };
        list_items.push(ListItem::new(Line::from(Span::styled(
            msg,
            Style::default().fg(msg_color),
        ))));
    }

    let nerd = app.icon_mode.use_nerd_icons();
    let title = if app.view_mode == ViewMode::TopFiles {
        let icon = if nerd { " 󰈔 " } else { " " };
        format!(
            "{icon}Top Largest Files ({} files) — [f] back to directory ",
            items.len()
        )
    } else {
        let icon = if nerd { "  " } else { " " };
        format!(
            "{icon}Explorer [{}] — {} ",
            format_size(current_total_size),
            app.sort_badge()
        )
    };

    let list_widget = List::new(list_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(app.theme.border_focused))
                .title(Span::styled(
                    title,
                    Style::default().fg(app.theme.accent).bold(),
                )),
        )
        .highlight_style(
            Style::default()
                .bg(app.theme.highlight_bg)
                .fg(app.theme.highlight_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("❯ ");

    f.render_stateful_widget(list_widget, area, &mut app.list_state);
}

fn compute_treemap_rects(
    area: Rect,
    items: &[&DiskNode],
    indices: &[usize],
    horizontal: bool,
    out: &mut Vec<(Rect, usize)>,
) {
    if indices.is_empty() || area.width < 2 || area.height < 2 {
        return;
    }
    if indices.len() == 1 {
        out.push((area, indices[0]));
        return;
    }

    let total: u64 = indices.iter().map(|&i| items[i].size.max(1)).sum();
    if total == 0 {
        out.push((area, indices[0]));
        return;
    }

    let mut half_sum = 0u64;
    let mut split_pos = 1usize;
    for (pos, &idx) in indices.iter().enumerate() {
        half_sum += items[idx].size.max(1);
        if half_sum >= total / 2 && pos + 1 < indices.len() {
            split_pos = pos + 1;
            break;
        }
    }

    let first_weight: u64 = indices[..split_pos]
        .iter()
        .map(|&i| items[i].size.max(1))
        .sum();
    let ratio = (first_weight as f64 / total as f64).clamp(0.15, 0.85);

    if horizontal {
        let w1 = ((area.width as f64) * ratio).round() as u16;
        let w1 = w1.clamp(1, area.width.saturating_sub(1));
        let r1 = Rect {
            x: area.x,
            y: area.y,
            width: w1,
            height: area.height,
        };
        let r2 = Rect {
            x: area.x + w1,
            y: area.y,
            width: area.width - w1,
            height: area.height,
        };
        compute_treemap_rects(r1, items, &indices[..split_pos], !horizontal, out);
        compute_treemap_rects(r2, items, &indices[split_pos..], !horizontal, out);
    } else {
        let h1 = ((area.height as f64) * ratio).round() as u16;
        let h1 = h1.clamp(1, area.height.saturating_sub(1));
        let r1 = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: h1,
        };
        let r2 = Rect {
            x: area.x,
            y: area.y + h1,
            width: area.width,
            height: area.height - h1,
        };
        compute_treemap_rects(r1, items, &indices[..split_pos], !horizontal, out);
        compute_treemap_rects(r2, items, &indices[split_pos..], !horizontal, out);
    }
}

fn render_treemap_view(
    f: &mut Frame,
    app: &mut App,
    current_total_size: u64,
    items: &[&DiskNode],
    area: Rect,
) {
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.theme.border_focused))
        .title(Span::styled(
            format!(
                " Treemap [{}] — [t] switch to list ",
                format_size(current_total_size)
            ),
            Style::default().fg(app.theme.accent).bold(),
        ));
    let inner = outer_block.inner(area);
    f.render_widget(outer_block, area);

    app.treemap_rects.clear();
    if items.is_empty() || inner.width < 4 || inner.height < 3 {
        let msg = Paragraph::new("  Empty directory or terminal too small for treemap")
            .style(Style::default().fg(app.theme.text_dim));
        f.render_widget(msg, inner);
        return;
    }

    let max_blocks = items.len().min(18);
    let indices: Vec<usize> = (0..max_blocks).collect();
    let mut rects = Vec::new();
    compute_treemap_rects(inner, items, &indices, true, &mut rects);
    app.treemap_rects = rects.clone();

    let selected_idx = app.list_state.selected().unwrap_or(0);

    for (rect, idx) in rects {
        if rect.width < 2 || rect.height < 1 {
            continue;
        }
        let node = items[idx];
        let color = node.category().color(&app.theme);
        let is_selected = idx == selected_idx;
        let pct = if current_total_size > 0 {
            (node.size as f64 / current_total_size as f64) * 100.0
        } else {
            0.0
        };

        let border_type = if is_selected {
            BorderType::Double
        } else {
            BorderType::Rounded
        };
        let border_style = if is_selected {
            Style::default().fg(app.theme.text).bold()
        } else {
            Style::default().fg(color)
        };

        if rect.width >= 6 && rect.height >= 3 {
            let mut lines = Vec::new();
            let max_chars = rect.width.saturating_sub(2) as usize;
            let label: String = node.name.chars().take(max_chars).collect();
            lines.push(Line::from(Span::styled(
                label,
                Style::default()
                    .fg(if is_selected { app.theme.text } else { color })
                    .bold(),
            )));
            if rect.height >= 4 {
                let sz = format!("{} ({:.0}%)", format_size(node.size), pct);
                let sz_trunc: String = sz.chars().take(max_chars).collect();
                lines.push(Line::from(Span::styled(
                    sz_trunc,
                    Style::default().fg(app.theme.text_dim),
                )));
            }
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(border_type)
                .border_style(border_style)
                .style(if is_selected {
                    Style::default().bg(app.theme.highlight_bg)
                } else {
                    Style::default()
                });
            f.render_widget(Paragraph::new(lines).block(block), rect);
        } else {
            let block = Block::default().style(Style::default().bg(color));
            f.render_widget(block, rect);
        }
    }
}

fn render_analytics(
    f: &mut Frame,
    app: &App,
    selected_idx: Option<usize>,
    current_total: u64,
    items: &[&DiskNode],
    area: Rect,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(9), Constraint::Min(8)])
        .split(area);

    let selected_node = selected_idx
        .and_then(|idx| items.get(idx).copied())
        .or_else(|| items.first().copied());

    let mut details_lines = Vec::new();
    if let Some(target) = selected_node {
        let pct = if current_total > 0 {
            (target.size as f64 / current_total as f64) * 100.0
        } else {
            0.0
        };
        let cat = target.category();
        let cat_color = cat.color(&app.theme);

        details_lines.push(Line::from(vec![
            Span::styled(" Name:      ", Style::default().fg(app.theme.text_dim)),
            Span::styled(&target.name, Style::default().fg(cat_color).bold()),
        ]));
        details_lines.push(Line::from(vec![
            Span::styled(" Category:  ", Style::default().fg(app.theme.text_dim)),
            Span::styled(cat.label(), Style::default().fg(cat_color)),
            if target.access_denied {
                Span::styled(
                    " (Permission Denied - needs sudo)",
                    Style::default().fg(app.theme.gauge_crit).bold(),
                )
            } else if target.is_symlink {
                Span::styled(" (Symlink)", Style::default().fg(app.theme.text_dim))
            } else {
                Span::raw("")
            },
        ]));
        details_lines.push(Line::from(vec![
            Span::styled(" Full Path: ", Style::default().fg(app.theme.text_dim)),
            Span::styled(
                target.path.to_string_lossy(),
                Style::default().fg(app.theme.text),
            ),
        ]));
        details_lines.push(Line::from(vec![
            Span::styled(" Disk Size: ", Style::default().fg(app.theme.text_dim)),
            Span::styled(
                format_size(target.size),
                Style::default().fg(app.theme.gauge_ok).bold(),
            ),
            Span::styled(
                format!("  ({:.2}% of scope)", pct),
                Style::default().fg(app.theme.accent),
            ),
        ]));
        details_lines.push(Line::from(vec![
            Span::styled(" Contains:  ", Style::default().fg(app.theme.text_dim)),
            Span::styled(
                format!(
                    "{} subdirs, {} files",
                    format_count(target.dir_count),
                    format_count(target.file_count)
                ),
                Style::default().fg(app.theme.text),
            ),
        ]));
    } else {
        details_lines.push(Line::from(Span::styled(
            " No item selected",
            Style::default().fg(app.theme.text_dim),
        )));
    }

    let detail_widget = Paragraph::new(details_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(app.theme.border))
            .title(Span::styled(
                " Target Inspection ",
                Style::default().fg(app.theme.gauge_warn).bold(),
            )),
    );
    f.render_widget(detail_widget, chunks[0]);

    let mut treemap_lines = Vec::new();
    treemap_lines.push(Line::from(vec![
        Span::styled(
            " Top Space Consumers ",
            Style::default().fg(app.theme.accent).bold(),
        ),
        Span::styled(
            "([t] toggle 2D treemap · [f] top files):",
            Style::default().fg(app.theme.text_dim),
        ),
    ]));
    treemap_lines.push(Line::from(""));

    let top_items: Vec<&DiskNode> = items.iter().copied().take(6).collect();

    if top_items.is_empty() {
        treemap_lines.push(Line::from(Span::styled(
            "  No space consumers in current view",
            Style::default().fg(app.theme.text_dim),
        )));
    } else {
        for (i, child) in top_items.iter().enumerate() {
            let color = child.category().color(&app.theme);
            let pct = if current_total > 0 {
                (child.size as f64 / current_total as f64) * 100.0
            } else {
                0.0
            };

            let block_cols = ((pct / 100.0) * (area.width.saturating_sub(8) as f64))
                .clamp(1.0, 44.0) as usize;
            let bar_block = "█".repeat(block_cols);

            let name_disp = if child.name.chars().count() > 20 {
                let s: String = child.name.chars().take(19).collect();
                format!("{s}…")
            } else {
                child.name.clone()
            };

            treemap_lines.push(Line::from(vec![
                Span::styled(
                    format!(" {:<2} ", i + 1),
                    Style::default().fg(app.theme.text_dim),
                ),
                Span::styled(
                    format!("{:<20}", name_disp),
                    Style::default().fg(color).bold(),
                ),
                Span::styled(
                    format!(" {:>9} ", format_size(child.size)),
                    Style::default().fg(app.theme.text),
                ),
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
                .border_style(Style::default().fg(app.theme.secondary))
                .title(Span::styled(
                    " Space Allocation & Heatmap ",
                    Style::default().fg(app.theme.secondary).bold(),
                )),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(treemap_widget, chunks[1]);
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let mut spans = Vec::new();

    let (marked_count, marked_bytes) = app.marked_summary();
    if marked_count > 0 {
        spans.push(Span::styled(
            format!(" ● {marked_count} selected · {} ", format_size(marked_bytes)),
            Style::default()
                .fg(app.theme.match_fg)
                .bg(app.theme.gauge_warn)
                .bold(),
        ));
        spans.push(Span::styled(" │ ", Style::default().fg(app.theme.text_dim)));
    }

    if app.modal.is_some() {
        spans.push(Span::styled(
            "Esc/n",
            Style::default().fg(app.theme.accent).bold(),
        ));
        spans.push(Span::styled(
            " close/cancel · ",
            Style::default().fg(app.theme.text_dim),
        ));
        spans.push(Span::styled(
            "Enter/y",
            Style::default().fg(app.theme.gauge_ok).bold(),
        ));
        spans.push(Span::styled(
            " confirm",
            Style::default().fg(app.theme.text_dim),
        ));
    } else if app.is_filtering {
        spans.push(Span::styled(
            "Type",
            Style::default().fg(app.theme.accent).bold(),
        ));
        spans.push(Span::styled(
            " to filter · ",
            Style::default().fg(app.theme.text_dim),
        ));
        spans.push(Span::styled(
            "↑/↓",
            Style::default().fg(app.theme.accent).bold(),
        ));
        spans.push(Span::styled(
            " select · ",
            Style::default().fg(app.theme.text_dim),
        ));
        spans.push(Span::styled(
            "Enter",
            Style::default().fg(app.theme.gauge_ok).bold(),
        ));
        spans.push(Span::styled(
            " apply · ",
            Style::default().fg(app.theme.text_dim),
        ));
        spans.push(Span::styled(
            "Esc",
            Style::default().fg(app.theme.gauge_crit).bold(),
        ));
        spans.push(Span::styled(
            " clear filter",
            Style::default().fg(app.theme.text_dim),
        ));
    } else {
        let hints: &[(&str, &str)] = match app.view_mode {
            ViewMode::TopFiles => &[
                ("j/k", "move"),
                ("Enter", "locate"),
                ("Space", "mark"),
                ("d", "trash"),
                ("D", "perm-del"),
                ("y", "copy"),
                ("o", "open"),
                ("f/Esc", "list"),
                ("?", "help"),
            ],
            ViewMode::Treemap => &[
                ("j/k", "move"),
                ("l/Enter", "open"),
                ("h", "back"),
                ("t", "list view"),
                ("d", "trash"),
                ("/", "search"),
                ("?", "help"),
            ],
            ViewMode::List => &[
                ("j/k", "move"),
                ("l", "open"),
                ("h", "back"),
                ("Space", "mark"),
                ("d", "trash"),
                ("D", "delete"),
                ("s/S", "sort"),
                ("t", "treemap"),
                ("f", "top files"),
                ("/", "search"),
                ("?", "help"),
            ],
        };

        for (i, (key, desc)) in hints.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(
                    " · ",
                    Style::default().fg(app.theme.text_dim),
                ));
            }
            spans.push(Span::styled(
                *key,
                Style::default().fg(app.theme.accent).bold(),
            ));
            spans.push(Span::styled(
                format!(" {desc}"),
                Style::default().fg(app.theme.text_dim),
            ));
        }
    }

    let footer = Paragraph::new(Line::from(spans)).alignment(Alignment::Center);
    f.render_widget(footer, area);
}

fn render_help_modal(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(72, 78, area);
    f.render_widget(Clear, popup_area);

    let key_style = Style::default().fg(app.theme.gauge_warn).bold();
    let val_style = Style::default().fg(app.theme.text);

    let help_text = vec![
        Line::from(Span::styled(
            "Keyboard & Mouse Controls",
            Style::default().fg(app.theme.accent).bold(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  j / k / ↑ / ↓ / Scroll  ", key_style),
            Span::styled("Move selection up / down", val_style),
        ]),
        Line::from(vec![
            Span::styled("  g / G                   ", key_style),
            Span::styled("Jump to top / bottom of list", val_style),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+d / Ctrl+u         ", key_style),
            Span::styled("Scroll half-page down / up", val_style),
        ]),
        Line::from(vec![
            Span::styled("  l / Enter / Dbl-Click   ", key_style),
            Span::styled("Enter directory (or jump to file in Top-N view)", val_style),
        ]),
        Line::from(vec![
            Span::styled("  h / Backspace / R-Click ", key_style),
            Span::styled("Return to parent directory (remembers cursor)", val_style),
        ]),
        Line::from(vec![
            Span::styled("  ~                       ", key_style),
            Span::styled("Jump to home directory ($HOME)", val_style),
        ]),
        Line::from(vec![
            Span::styled("  s / S                   ", key_style),
            Span::styled("Cycle sort mode (Size / Name / Count) / Reverse direction", val_style),
        ]),
        Line::from(vec![
            Span::styled("  /                       ", key_style),
            Span::styled("Filter items with live match count & highlighting", val_style),
        ]),
        Line::from(vec![
            Span::styled("  t                       ", key_style),
            Span::styled("Toggle 2D proportional block Treemap view", val_style),
        ]),
        Line::from(vec![
            Span::styled("  f                       ", key_style),
            Span::styled("Toggle Top-N largest files across the entire scan", val_style),
        ]),
        Line::from(vec![
            Span::styled("  Space                   ", key_style),
            Span::styled("Mark / unmark item for multi-select batch actions", val_style),
        ]),
        Line::from(vec![
            Span::styled("  d                       ", key_style),
            Span::styled("Move selected/marked item(s) to FreeDesktop Trash", val_style),
        ]),
        Line::from(vec![
            Span::styled("  D / Delete              ", Style::default().fg(app.theme.gauge_crit).bold()),
            Span::styled("Permanently delete selected/marked item(s)", val_style),
        ]),
        Line::from(vec![
            Span::styled("  y / o                   ", key_style),
            Span::styled("Copy path to clipboard (y) / Open in $EDITOR or file manager (o)", val_style),
        ]),
        Line::from(vec![
            Span::styled("  T / r                   ", key_style),
            Span::styled("Cycle color theme (T) / Refresh partition telemetry (r)", val_style),
        ]),
        Line::from(vec![
            Span::styled("  q / Ctrl+c              ", Style::default().fg(app.theme.gauge_crit).bold()),
            Span::styled("Quit application safely", val_style),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Press [Esc] or [?] to close this window.",
            Style::default().fg(app.theme.text_dim),
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg))
        .title(Span::styled(
            " HELP & CHEATSHEET ",
            Style::default().fg(app.theme.gauge_warn).bold(),
        ));

    let popup = Paragraph::new(help_text)
        .block(block)
        .alignment(Alignment::Left);
    f.render_widget(popup, popup_area);
}

fn render_delete_confirm_modal(
    f: &mut Frame,
    app: &App,
    dialog: &DeleteConfirmDialog,
    area: Rect,
) {
    let popup_area = centered_rect(68, 56, area);
    f.render_widget(Clear, popup_area);

    let border_color = if dialog.any_system || dialog.permanent {
        app.theme.gauge_crit
    } else {
        app.theme.gauge_warn
    };

    let title = if dialog.any_system {
        " CAUTION: SUPERUSER SYSTEM DELETION "
    } else if dialog.permanent {
        " CONFIRM PERMANENT DELETION "
    } else {
        " MOVE TO TRASH "
    };

    let prompt = if dialog.permanent {
        "  Are you sure you want to PERMANENTLY delete the following?"
    } else {
        "  Move the following item(s) to the system Trash?"
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            prompt,
            Style::default().fg(app.theme.text).bold(),
        )),
        Line::from(""),
    ];

    if dialog.targets.len() == 1 {
        let t = &dialog.targets[0];
        lines.push(Line::from(vec![
            Span::styled("  Item Type:  ", Style::default().fg(app.theme.text_dim)),
            Span::styled(
                if t.is_dir {
                    "Directory / Folder (Recursive)"
                } else {
                    "Regular File"
                },
                Style::default().fg(app.theme.accent).bold(),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Item Name:  ", Style::default().fg(app.theme.text_dim)),
            Span::styled(&t.name, Style::default().fg(app.theme.gauge_warn).bold()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Location:   ", Style::default().fg(app.theme.text_dim)),
            Span::styled(
                t.path.to_string_lossy(),
                Style::default().fg(app.theme.text),
            ),
        ]));
        if t.is_dir {
            lines.push(Line::from(vec![
                Span::styled("  Contains:   ", Style::default().fg(app.theme.text_dim)),
                Span::styled(
                    format!("{} subfolders, {} files", t.dir_count, t.file_count),
                    Style::default().fg(app.theme.text),
                ),
            ]));
        }
    } else {
        lines.push(Line::from(vec![
            Span::styled("  Batch Size: ", Style::default().fg(app.theme.text_dim)),
            Span::styled(
                format!("{} marked items", dialog.targets.len()),
                Style::default().fg(app.theme.gauge_warn).bold(),
            ),
        ]));
        for t in dialog.targets.iter().take(4) {
            lines.push(Line::from(vec![
                Span::styled("    • ", Style::default().fg(app.theme.text_dim)),
                Span::styled(&t.name, Style::default().fg(app.theme.text)),
                Span::styled(
                    format!(" ({})", format_size(t.size)),
                    Style::default().fg(app.theme.text_dim),
                ),
            ]));
        }
        if dialog.targets.len() > 4 {
            lines.push(Line::from(Span::styled(
                format!("    …and {} more", dialog.targets.len() - 4),
                Style::default().fg(app.theme.text_dim),
            )));
        }
    }

    lines.push(Line::from(vec![
        Span::styled("  Total Size: ", Style::default().fg(app.theme.text_dim)),
        Span::styled(
            format_size(dialog.total_size),
            Style::default().fg(border_color).bold(),
        ),
    ]));
    lines.push(Line::from(""));

    if dialog.any_system {
        lines.push(Line::from(Span::styled(
            "  EXTREME CAUTION: Superuser privilege active on system path!",
            Style::default().fg(app.theme.gauge_crit).bold(),
        )));
    } else if dialog.permanent {
        lines.push(Line::from(Span::styled(
            "  CAUTION: Permanent deletion bypasses Trash and cannot be undone.",
            Style::default().fg(app.theme.gauge_crit).bold(),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "  Safe Action: Items will be moved to your FreeDesktop Trash folder.",
            Style::default().fg(app.theme.gauge_ok),
        )));
    }

    lines.push(Line::from(""));
    let confirm_label = if dialog.permanent {
        " Confirm Permanent Delete    "
    } else {
        " Confirm Move to Trash       "
    };
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            " [ y / Enter ] ",
            Style::default()
                .bg(border_color)
                .fg(app.theme.match_fg)
                .bold(),
        ),
        Span::styled(confirm_label, Style::default().fg(border_color).bold()),
        Span::styled(
            "[ n / Esc ] ",
            Style::default()
                .bg(app.theme.highlight_bg)
                .fg(app.theme.text)
                .bold(),
        ),
        Span::styled(" Cancel", Style::default().fg(app.theme.text_dim)),
    ]));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(app.theme.bg))
        .title(Span::styled(
            title,
            Style::default().fg(border_color).bold(),
        ));

    let popup = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
    f.render_widget(popup, popup_area);
}

fn render_system_protected_modal(
    f: &mut Frame,
    app: &App,
    dialog: &SystemProtectedDialog,
    area: Rect,
) {
    let popup_area = centered_rect(68, 52, area);
    f.render_widget(Clear, popup_area);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  CANNOT DELETE SYSTEM FILES OR FOLDERS",
            Style::default().fg(app.theme.gauge_crit).bold(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Target:    ", Style::default().fg(app.theme.text_dim)),
            Span::styled(
                &dialog.target_name,
                Style::default().fg(app.theme.gauge_warn).bold(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Path:      ", Style::default().fg(app.theme.text_dim)),
            Span::styled(
                dialog.target_path.to_string_lossy(),
                Style::default().fg(app.theme.text),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Security Policy:",
            Style::default().fg(app.theme.accent).bold(),
        )),
        Line::from(Span::styled(
            "    System components and root-owned files are protected from modification.",
            Style::default().fg(app.theme.text),
        )),
        Line::from(Span::styled(
            "    To delete system files, re-launch with sudo:",
            Style::default().fg(app.theme.text_dim),
        )),
        Line::from(vec![
            Span::raw("      "),
            Span::styled(
                "sudo arch-disk-tui ",
                Style::default().fg(app.theme.gauge_ok).bold(),
            ),
            Span::styled("<path>", Style::default().fg(app.theme.text)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                " [ Esc / Enter ] ",
                Style::default()
                    .bg(app.theme.highlight_bg)
                    .fg(app.theme.text)
                    .bold(),
            ),
            Span::styled(" Dismiss", Style::default().fg(app.theme.text_dim)),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(app.theme.gauge_crit))
        .style(Style::default().bg(app.theme.bg))
        .title(Span::styled(
            " ACCESS RESTRICTED - SYSTEM PROTECTED ",
            Style::default().fg(app.theme.gauge_crit).bold(),
        ));

    let popup = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
    f.render_widget(popup, popup_area);
}

fn render_message_modal(f: &mut Frame, app: &App, dialog: &MessageDialog, area: Rect) {
    let popup_area = centered_rect(60, 35, area);
    f.render_widget(Clear, popup_area);

    let border_color = if dialog.is_error {
        app.theme.gauge_crit
    } else {
        app.theme.accent
    };

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", dialog.message),
            Style::default().fg(app.theme.text),
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                " [ Esc / Enter ] ",
                Style::default()
                    .bg(app.theme.highlight_bg)
                    .fg(app.theme.text)
                    .bold(),
            ),
            Span::styled(" Dismiss", Style::default().fg(app.theme.text_dim)),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(app.theme.bg))
        .title(Span::styled(
            format!(" {} ", dialog.title),
            Style::default().fg(border_color).bold(),
        ));

    let popup = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app(root: PathBuf) -> App {
        App::new(
            root,
            Theme::arch_ice_blue(),
            IconMode::Ascii,
            false,
            Vec::new(),
        )
    }

    #[test]
    fn test_app_delete_file_and_folder_flow() {
        let temp_dir = std::env::temp_dir().join("arch_disk_tui_app_test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let file_a = temp_dir.join("test_file.txt");
        std::fs::write(&file_a, b"hello world").unwrap();

        let sub_dir = temp_dir.join("my_subfolder");
        std::fs::create_dir_all(&sub_dir).unwrap();
        let file_b = sub_dir.join("nested.bin");
        std::fs::write(&file_b, b"1234567890").unwrap();

        let mut app = test_app(temp_dir.clone());
        app.root_node.insert(&file_a, 11, false, &temp_dir);
        app.root_node.insert(&sub_dir, 10, true, &temp_dir);
        app.root_node.insert(&file_b, 10, false, &temp_dir);

        let items = app.get_filtered_sorted_children();
        assert_eq!(items.len(), 2);

        let file_idx = items.iter().position(|i| i.name == "test_file.txt").unwrap();
        app.list_state.select(Some(file_idx));

        app.request_delete_selected();
        assert!(
            matches!(app.modal, Some(Modal::DeleteConfirm(ref d)) if d.targets.len() == 1 && d.targets[0].name == "test_file.txt")
        );

        app.execute_deletion();
        assert!(!file_a.exists());
        assert!(app.modal.is_none());
        assert!(app.status_notification.is_some());

        let remaining = app.get_filtered_sorted_children();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].name, "my_subfolder");
        app.list_state.select(Some(0));

        app.request_delete_selected();
        assert!(
            matches!(app.modal, Some(Modal::DeleteConfirm(ref d)) if d.targets[0].name == "my_subfolder" && d.targets[0].is_dir)
        );

        app.execute_deletion();
        assert!(!sub_dir.exists());
        assert!(!file_b.exists());
        assert!(app.modal.is_none());
        assert_eq!(app.get_filtered_sorted_children().len(), 0);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cursor_memory_on_back_navigation() {
        let root_path = PathBuf::from("/home/user");
        let mut app = test_app(root_path.clone());
        app.root_node
            .insert(&PathBuf::from("/home/user/alpha/a.txt"), 1000, false, &root_path);
        app.root_node
            .insert(&PathBuf::from("/home/user/beta/b.txt"), 500, false, &root_path);
        app.root_node
            .insert(&PathBuf::from("/home/user/gamma/c.txt"), 100, false, &root_path);

        // Select second item ("beta")
        app.list_state.select(Some(1));
        app.enter_dir();
        assert_eq!(app.current_path, PathBuf::from("/home/user/beta"));

        // Exit back to parent -> should restore cursor on "beta" (index 1), not reset to 0!
        app.exit_dir();
        assert_eq!(app.current_path, root_path);
        assert_eq!(app.list_state.selected(), Some(1));
    }

    #[test]
    fn test_sort_cycle_and_reverse() {
        let root_path = PathBuf::from("/home/user");
        let mut app = test_app(root_path.clone());
        app.root_node
            .insert(&PathBuf::from("/home/user/b.txt"), 100, false, &root_path);
        app.root_node
            .insert(&PathBuf::from("/home/user/a.txt"), 500, false, &root_path);

        assert_eq!(app.sort_badge(), "Sort: Size ↓");
        assert_eq!(app.get_active_items()[0].name, "a.txt");

        app.reverse_sort();
        assert_eq!(app.sort_badge(), "Sort: Size ↑");
        assert_eq!(app.get_active_items()[0].name, "b.txt");

        app.cycle_sort();
        assert_eq!(app.sort_badge(), "Sort: Name ↑");
        assert_eq!(app.get_active_items()[0].name, "a.txt");
    }

    #[test]
    fn test_multi_select_batch_trash() {
        let temp_dir = std::env::temp_dir().join("arch_disk_tui_batch_test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();
        unsafe {
            std::env::set_var("XDG_DATA_HOME", temp_dir.join("trash_xdg"));
        }

        let f1 = temp_dir.join("one.bin");
        let f2 = temp_dir.join("two.bin");
        std::fs::write(&f1, b"1111").unwrap();
        std::fs::write(&f2, b"22222222").unwrap();

        let mut app = test_app(temp_dir.clone());
        app.root_node.insert(&f1, 4, false, &temp_dir);
        app.root_node.insert(&f2, 8, false, &temp_dir);

        app.list_state.select(Some(0));
        app.toggle_mark_selected();
        app.list_state.select(Some(1));
        app.toggle_mark_selected();

        assert_eq!(app.marked_summary(), (2, 12));

        // Request trash (permanent = false)
        app.request_delete(false);
        assert!(
            matches!(app.modal, Some(Modal::DeleteConfirm(ref d)) if d.targets.len() == 2 && !d.permanent)
        );
        app.execute_deletion();
        assert!(!f1.exists());
        assert!(!f2.exists());
        assert!(app.marked_paths.is_empty());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_app_system_file_deletion_blocked_for_regular_user() {
        let scan_root = PathBuf::from("/etc");
        let mut app = test_app(scan_root.clone());
        let system_file = PathBuf::from("/etc/fstab");
        app.root_node.insert(&system_file, 100, false, &scan_root);

        let items = app.get_filtered_sorted_children();
        assert_eq!(items.len(), 1);
        app.list_state.select(Some(0));

        app.request_delete_selected();

        if !is_superuser() {
            assert!(
                matches!(app.modal, Some(Modal::SystemProtected(ref d)) if d.target_name == "fstab")
            );
        } else {
            assert!(
                matches!(app.modal, Some(Modal::DeleteConfirm(ref d)) if d.targets[0].name == "fstab" && d.any_system)
            );
        }
    }
}