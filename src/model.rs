use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DiskNode {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub is_dir: bool,
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
            file_count: if is_dir { 0 } else { 1 },
            dir_count: 0,
            children: HashMap::new(),
        }
    }

    /// Insert a path into the tree and bubble up sizes and counts
    pub fn insert(&mut self, full_path: &Path, file_size: u64, is_dir: bool, root_path: &Path) {
        if let Ok(relative) = full_path.strip_prefix(root_path) {
            let components: Vec<String> = relative
                .components()
                .filter_map(|c| c.as_os_str().to_str().map(String::from))
                .collect();

            if components.is_empty() {
                return;
            }

            self.insert_recursive(components, full_path, file_size, is_dir);
        }
    }

    fn insert_recursive(&mut self, mut comps: Vec<String>, full_path: &Path, file_size: u64, is_dir: bool) {
        self.size += file_size;
        if is_dir {
            self.dir_count += 1;
        } else {
            self.file_count += 1;
        }

        if comps.is_empty() {
            return;
        }

        let head = comps.remove(0);
        let is_last = comps.is_empty();

        let child_path = if self.path.join(&head) == full_path {
            full_path.to_path_buf()
        } else {
            self.path.join(&head)
        };

        let child = self.children.entry(head.clone()).or_insert_with(|| {
            DiskNode::new(head, child_path, if is_last { is_dir } else { true })
        });

        child.insert_recursive(comps, full_path, file_size, is_dir);
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