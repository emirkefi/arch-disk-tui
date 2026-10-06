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
            file_count: 0,
            dir_count: 0,
            children: HashMap::new(),
        }
    }

    /// Insert a path into the tree and bubble up sizes and counts iteratively
    pub fn insert(&mut self, full_path: &Path, file_size: u64, is_dir: bool, root_path: &Path) {
        let Ok(relative) = full_path.strip_prefix(root_path) else {
            return;
        };

        let mut comps = relative.components().peekable();
        if comps.peek().is_none() {
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

                let new_node = DiskNode::new(
                    comp_str.to_string(),
                    child_path,
                    if is_last { is_dir } else { true },
                );
                curr.children.insert(comp_str.to_string(), new_node);
            }

            let child = curr.children.get_mut(comp_str).unwrap();
            child.size += file_size;
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
        // 50,000 insertions should complete well under 200ms
        assert!(elapsed.as_millis() < 250, "Insertion of 50k items took too long: {:?}", elapsed);
    }
}