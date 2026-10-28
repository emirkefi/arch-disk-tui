use crate::theme::{IconMode, Theme};
use ratatui::style::Color;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileCategory {
    Directory,
    Media,
    Archive,
    Code,
    Cache,
    Document,
    Binary,
    Other,
}

impl FileCategory {
    pub fn classify(name: &str, is_dir: bool) -> Self {
        let lower = name.to_lowercase();
        if is_dir {
            if matches!(
                lower.as_str(),
                ".cache"
                    | "cache"
                    | "caches"
                    | "node_modules"
                    | "target"
                    | "__pycache__"
                    | ".pytest_cache"
                    | ".mypy_cache"
                    | ".cargo"
                    | ".npm"
                    | ".pnpm-store"
                    | ".gradle"
                    | "tmp"
                    | "temp"
                    | "logs"
                    | ".trash"
                    | "trash"
            ) {
                return Self::Cache;
            }
            return Self::Directory;
        }

        let ext = Path::new(&lower)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match ext {
            // Media (video, audio, images)
            "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" | "wmv" | "m4v" | "mp3" | "flac"
            | "wav" | "ogg" | "m4a" | "aac" | "opus" | "png" | "jpg" | "jpeg" | "gif"
            | "webp" | "svg" | "bmp" | "ico" | "tiff" | "raw" | "heic" | "avif" => Self::Media,
            // Archives & Disk Images
            "zip" | "tar" | "gz" | "tgz" | "xz" | "txz" | "zst" | "tzst" | "bz2" | "tbz2"
            | "7z" | "rar" | "iso" | "img" | "dmg" | "qcow2" | "vdi" | "vmdk" | "deb" | "rpm"
            | "pkg" | "appimage" | "flatpak" | "snap" | "apk" | "jar" | "war" => Self::Archive,
            // Code & Config
            "rs" | "py" | "js" | "ts" | "tsx" | "jsx" | "go" | "c" | "cpp" | "cc" | "h"
            | "hpp" | "java" | "kt" | "swift" | "rb" | "php" | "cs" | "lua" | "zig" | "nim"
            | "hs" | "ml" | "sh" | "bash" | "zsh" | "fish" | "toml" | "yaml" | "yml" | "json"
            | "jsonc" | "xml" | "html" | "css" | "scss" | "sass" | "sql" | "graphql" | "proto"
            | "vue" | "svelte" | "nix" | "cmake" | "mk" | "ini" | "conf" | "cfg" => Self::Code,
            // Caches, Logs & Temp files
            "log" | "tmp" | "temp" | "cache" | "bak" | "old" | "swp" | "swo" | "pid" | "lock"
            | "pyc" | "pyo" | "class" | "rlib" | "rmeta" | "d" => Self::Cache,
            // Documents
            "md" | "txt" | "rst" | "org" | "pdf" | "epub" | "mobi" | "doc" | "docx" | "odt"
            | "xls" | "xlsx" | "ods" | "csv" | "tsv" | "ppt" | "pptx" | "odp" | "tex" | "rtf" => {
                Self::Document
            }
            // Binaries, Libraries & Databases
            "so" | "a" | "o" | "ko" | "dll" | "exe" | "bin" | "elf" | "wasm" | "db"
            | "sqlite" | "sqlite3" | "mdb" | "dat" | "idx" | "pak" | "safetensors" | "gguf"
            | "onnx" | "pt" | "pth" | "ttf" | "otf" | "woff" | "woff2" => Self::Binary,
            _ => Self::Other,
        }
    }

    pub fn color(self, theme: &Theme) -> Color {
        match self {
            Self::Directory => theme.dir_color,
            Self::Media => theme.media_color,
            Self::Archive => theme.archive_color,
            Self::Code => theme.code_color,
            Self::Cache => theme.cache_color,
            Self::Document => theme.doc_color,
            Self::Binary => theme.bin_color,
            Self::Other => theme.other_color,
        }
    }

    pub fn icon(self, icon_mode: IconMode, is_dir: bool, is_symlink: bool, access_denied: bool) -> &'static str {
        if icon_mode == IconMode::None {
            return "";
        }
        let nerd = icon_mode.use_nerd_icons();
        if access_denied {
            return if nerd { "󰌾 " } else { "! " };
        }
        if is_symlink {
            return if nerd { " " } else { "@ " };
        }
        if is_dir {
            return match self {
                Self::Cache => if nerd { "󰃨 " } else { "d " },
                _ => if nerd { " " } else { "/ " },
            };
        }
        if !nerd {
            return match self {
                Self::Directory => "/ ",
                Self::Media => "# ",
                Self::Archive => "@ ",
                Self::Code => "* ",
                Self::Cache => "~ ",
                Self::Document => "= ",
                Self::Binary => "% ",
                Self::Other => "- ",
            };
        }
        match self {
            Self::Directory => " ",
            Self::Media => " ",
            Self::Archive => " ",
            Self::Code => " ",
            Self::Cache => "󰃨 ",
            Self::Document => " ",
            Self::Binary => " ",
            Self::Other => " ",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Directory => "Directory",
            Self::Media => "Media",
            Self::Archive => "Archive / Image",
            Self::Code => "Code / Config",
            Self::Cache => "Cache / Temp / Log",
            Self::Document => "Document",
            Self::Binary => "Binary / Data",
            Self::Other => "File",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiskNode {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub access_denied: bool,
    pub file_count: usize,
    pub dir_count: usize,
    pub children: HashMap<String, DiskNode>,
}

impl DiskNode {
    pub fn new(name: String, path: PathBuf, is_dir: bool) -> Self {
        Self {
            name,
            path,
            size: 0,
            is_dir,
            is_symlink: false,
            access_denied: false,
            file_count: 0,
            dir_count: 0,
            children: HashMap::new(),
        }
    }

    pub fn category(&self) -> FileCategory {
        FileCategory::classify(&self.name, self.is_dir)
    }

    /// Insert a path into the tree and bubble up sizes and counts iteratively
    pub fn insert(&mut self, full_path: &Path, file_size: u64, is_dir: bool, root_path: &Path) {
        self.insert_ext(full_path, file_size, is_dir, false, false, root_path);
    }

    /// Extended insert supporting symlink and permission-denied flags
    pub fn insert_ext(
        &mut self,
        full_path: &Path,
        file_size: u64,
        is_dir: bool,
        is_symlink: bool,
        access_denied: bool,
        root_path: &Path,
    ) {
        let Ok(relative) = full_path.strip_prefix(root_path) else {
            return;
        };

        let mut comps = relative.components().peekable();
        if comps.peek().is_none() {
            if access_denied {
                self.access_denied = true;
            }
            return;
        }

        self.size += file_size;
        if is_dir {
            self.dir_count += 1;
        } else {
            self.file_count += 1;
        }

        let mut curr = self;
        while let Some(comp) = comps.next() {
            let comp_str = match comp.as_os_str().to_str() {
                Some(s) if !s.is_empty() => s,
                _ => continue,
            };
            let is_last = comps.peek().is_none();

            if !curr.children.contains_key(comp_str) {
                let child_path = if is_last && curr.path.join(comp_str) == full_path {
                    full_path.to_path_buf()
                } else {
                    curr.path.join(comp_str)
                };

                let mut new_node = DiskNode::new(
                    comp_str.to_string(),
                    child_path,
                    if is_last { is_dir } else { true },
                );
                if is_last {
                    new_node.is_symlink = is_symlink;
                    new_node.access_denied = access_denied;
                }
                curr.children.insert(comp_str.to_string(), new_node);
            }

            let child = curr.children.get_mut(comp_str).unwrap();
            child.size += file_size;
            if is_last {
                if is_symlink {
                    child.is_symlink = true;
                }
                if access_denied {
                    child.access_denied = true;
                }
            }
            if is_dir {
                child.dir_count += 1;
            } else {
                child.file_count += 1;
            }

            curr = child;
        }
    }

    /// Find node given a path relative to this root
    pub fn find_node<'a>(&'a self, target: &Path) -> Option<&'a DiskNode> {
        if self.path == target {
            return Some(self);
        }
        if let Ok(rel) = target.strip_prefix(&self.path) {
            let mut curr = self;
            for part in rel.components() {
                let name = part.as_os_str().to_str()?;
                curr = curr.children.get(name)?;
            }
            Some(curr)
        } else {
            None
        }
    }

    /// Collect the top `limit` largest individual files across the subtree
    pub fn collect_largest_files<'a>(&'a self, limit: usize) -> Vec<&'a DiskNode> {
        let mut files = Vec::new();
        self.collect_files_inner(&mut files);
        files.sort_by(|a, b| b.size.cmp(&a.size));
        if files.len() > limit {
            files.truncate(limit);
        }
        files
    }

    fn collect_files_inner<'a>(&'a self, acc: &mut Vec<&'a DiskNode>) {
        for child in self.children.values() {
            if child.is_dir {
                child.collect_files_inner(acc);
            } else {
                acc.push(child);
            }
        }
    }

    /// Remove node given a path relative to this root, bubbling down size and count reductions
    pub fn remove_node(&mut self, target: &Path, root_path: &Path) -> Option<DiskNode> {
        let relative = target.strip_prefix(root_path).ok()?;
        let components: Vec<&str> = relative
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();

        if components.is_empty() {
            return None;
        }

        self.remove_slice(&components)
    }

    fn remove_slice(&mut self, comps: &[&str]) -> Option<DiskNode> {
        if comps.is_empty() {
            return None;
        }

        let head = comps[0];
        let tail = &comps[1..];

        if tail.is_empty() {
            if let Some(removed) = self.children.remove(head) {
                self.size = self.size.saturating_sub(removed.size);
                if removed.is_dir {
                    self.file_count = self.file_count.saturating_sub(removed.file_count);
                    self.dir_count = self.dir_count.saturating_sub(removed.dir_count + 1);
                } else {
                    self.file_count = self.file_count.saturating_sub(1);
                }
                return Some(removed);
            }
            None
        } else if let Some(child) = self.children.get_mut(head) {
            let removed = child.remove_slice(tail);
            if let Some(ref rem) = removed {
                self.size = self.size.saturating_sub(rem.size);
                if rem.is_dir {
                    self.file_count = self.file_count.saturating_sub(rem.file_count);
                    self.dir_count = self.dir_count.saturating_sub(rem.dir_count + 1);
                } else {
                    self.file_count = self.file_count.saturating_sub(1);
                }
            }
            removed
        } else {
            None
        }
    }

    /// Export tree to JSON string (sorted by size descending up to `max_depth`)
    pub fn to_json(&self, max_depth: usize) -> String {
        let mut out = String::new();
        self.write_json(&mut out, 0, max_depth);
        out.push('\n');
        out
    }

    fn write_json(&self, out: &mut String, depth: usize, max_depth: usize) {
        let indent = "  ".repeat(depth);
        let _ = writeln!(out, "{{");
        let _ = writeln!(
            out,
            "{indent}  \"name\": \"{}\",",
            escape_json(&self.name)
        );
        let _ = writeln!(
            out,
            "{indent}  \"path\": \"{}\",",
            escape_json(&self.path.to_string_lossy())
        );
        let _ = writeln!(out, "{indent}  \"size\": {},", self.size);
        let _ = writeln!(
            out,
            "{indent}  \"size_human\": \"{}\",",
            format_size(self.size)
        );
        let _ = writeln!(out, "{indent}  \"is_dir\": {},", self.is_dir);
        let _ = writeln!(out, "{indent}  \"file_count\": {},", self.file_count);
        let _ = writeln!(out, "{indent}  \"dir_count\": {},", self.dir_count);
        let _ = writeln!(
            out,
            "{indent}  \"access_denied\": {},",
            self.access_denied
        );

        if self.is_dir && depth < max_depth && !self.children.is_empty() {
            let _ = writeln!(out, "{indent}  \"children\": [");
            let mut sorted: Vec<&DiskNode> = self.children.values().collect();
            sorted.sort_by(|a, b| b.size.cmp(&a.size));
            for (i, child) in sorted.iter().enumerate() {
                let _ = write!(out, "{}    ", indent);
                child.write_json(out, depth + 2, max_depth);
                if i + 1 < sorted.len() {
                    let _ = writeln!(out, ",");
                } else {
                    let _ = writeln!(out);
                }
            }
            let _ = writeln!(out, "{indent}  ]");
        } else {
            let _ = writeln!(out, "{indent}  \"children\": []");
        }
        let _ = write!(out, "{indent}}}");
    }

    /// Export tree entries to CSV string
    pub fn to_csv(&self, max_depth: usize) -> String {
        let mut out = String::from("path,name,type,category,size_bytes,size_human,files,subdirs,access_denied\n");
        self.write_csv(&mut out, 0, max_depth);
        out
    }

    fn write_csv(&self, out: &mut String, depth: usize, max_depth: usize) {
        let item_type = if self.is_dir { "dir" } else { "file" };
        let _ = writeln!(
            out,
            "\"{}\",\"{}\",{},{},{},\"{}\",{},{},{}",
            escape_csv(&self.path.to_string_lossy()),
            escape_csv(&self.name),
            item_type,
            self.category().label(),
            self.size,
            format_size(self.size),
            self.file_count,
            self.dir_count,
            self.access_denied
        );

        if self.is_dir && depth < max_depth {
            let mut sorted: Vec<&DiskNode> = self.children.values().collect();
            sorted.sort_by(|a, b| b.size.cmp(&a.size));
            for child in sorted {
                child.write_csv(out, depth + 1, max_depth);
            }
        }
    }
}

fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn escape_csv(s: &str) -> String {
    s.replace('"', "\"\"")
}

pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub fn format_count(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_remove_file() {
        let root_path = PathBuf::from("/home/user");
        let mut root = DiskNode::new("user".into(), root_path.clone(), true);

        let file1 = PathBuf::from("/home/user/file1.txt");
        let file2 = PathBuf::from("/home/user/sub/file2.txt");

        root.insert(&file1, 500, false, &root_path);
        root.insert(&file2, 300, false, &root_path);

        assert_eq!(root.size, 800);
        assert_eq!(root.file_count, 2);

        let removed = root.remove_node(&file1, &root_path);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().size, 500);

        assert_eq!(root.size, 300);
        assert_eq!(root.file_count, 1);
        assert!(!root.children.contains_key("file1.txt"));
        assert!(root.children.contains_key("sub"));
    }

    #[test]
    fn test_remove_directory_recursively() {
        let root_path = PathBuf::from("/home/user");
        let mut root = DiskNode::new("user".into(), root_path.clone(), true);

        let sub_file1 = PathBuf::from("/home/user/downloads/a.iso");
        let sub_file2 = PathBuf::from("/home/user/downloads/nested/b.iso");

        root.insert(&sub_file1, 1000, false, &root_path);
        root.insert(&sub_file2, 2000, false, &root_path);

        assert_eq!(root.size, 3000);
        assert_eq!(root.file_count, 2);

        let downloads_path = PathBuf::from("/home/user/downloads");
        let removed = root.remove_node(&downloads_path, &root_path);
        assert!(removed.is_some());
        let rem = removed.unwrap();
        assert_eq!(rem.size, 3000);
        assert_eq!(rem.file_count, 2);

        assert_eq!(root.size, 0);
        assert_eq!(root.file_count, 0);
        assert!(!root.children.contains_key("downloads"));
    }

    #[test]
    fn test_collect_largest_files_and_export() {
        let root_path = PathBuf::from("/home/user");
        let mut root = DiskNode::new("user".into(), root_path.clone(), true);

        root.insert(&PathBuf::from("/home/user/small.txt"), 100, false, &root_path);
        root.insert(&PathBuf::from("/home/user/movies/huge.mkv"), 9000, false, &root_path);
        root.insert(&PathBuf::from("/home/user/code/main.rs"), 500, false, &root_path);

        let top = root.collect_largest_files(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].name, "huge.mkv");
        assert_eq!(top[1].name, "main.rs");

        let json = root.to_json(3);
        assert!(json.contains("\"huge.mkv\""));
        let csv = root.to_csv(3);
        assert!(csv.contains("huge.mkv"));
    }

    #[test]
    fn test_high_volume_insert_performance() {
        let root_path = PathBuf::from("/home/user");
        let mut root = DiskNode::new("user".into(), root_path.clone(), true);

        let start = std::time::Instant::now();
        for i in 0..50_000 {
            let path = root_path.join(format!("dir_{}/sub_{}/file_{}.bin", i % 50, (i / 50) % 20, i));
            root.insert(&path, 1024, false, &root_path);
        }
        let elapsed = start.elapsed();

        assert_eq!(root.file_count, 50_000);
        assert_eq!(root.size, 50_000 * 1024);
        assert!(elapsed.as_millis() < 250, "Insertion of 50k items took too long: {:?}", elapsed);
    }
}