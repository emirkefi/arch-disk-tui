use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub enum DeletionRestriction {
    RootFilesystem,
    CurrentScanRoot,
    SystemProtected { path: PathBuf },
}

/// Check if the current process is running with superuser (root) privileges.
pub fn is_superuser() -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::geteuid() == 0 }
    }
    #[cfg(not(unix))]
    {
        false
    }
}

/// Get effective UID of the current process.
pub fn current_uid() -> u32 {
    #[cfg(unix)]
    {
        unsafe { libc::geteuid() }
    }
    #[cfg(not(unix))]
    {
        1000
    }
}

/// Check if a path lies within standard Linux system directories.
pub fn is_system_path(path: &Path) -> bool {
    // Filesystem root is system
    if path == Path::new("/") {
        return true;
    }

    let system_prefixes = [
        "/bin", "/boot", "/dev", "/efi", "/etc", "/lib", "/lib64", "/lib32",
        "/lost+found", "/opt", "/proc", "/root", "/sbin", "/srv", "/sys",
        "/usr", "/var",
    ];

    let path_str = path.to_string_lossy();

    // Check raw path against system prefixes
    for prefix in &system_prefixes {
        if path_str == *prefix || path_str.starts_with(&format!("{}/", prefix)) {
            return true;
        }
    }

    // Top-level /home directory itself is a system root, though /home/<user> is personal
    if path_str == "/home" || path_str == "/home/" {
        return true;
    }

    // /run hierarchy: only /run/media is personal/user external media
    if (path_str == "/run" || path_str.starts_with("/run/"))
        && !path_str.starts_with("/run/media")
    {
        return true;
    }

    // Also check canonicalized path to prevent symlink bypasses
    if let Ok(canonical) = std::fs::canonicalize(path) {
        if canonical == Path::new("/") {
            return true;
        }
        let can_str = canonical.to_string_lossy();
        for prefix in &system_prefixes {
            if can_str == *prefix || can_str.starts_with(&format!("{}/", prefix)) {
                return true;
            }
        }
        if can_str == "/home" || can_str == "/home/" {
            return true;
        }
        if (can_str == "/run" || can_str.starts_with("/run/"))
            && !can_str.starts_with("/run/media")
        {
            return true;
        }
    }

    false
}

/// Check if a file or directory is a system file or folder,
/// or belongs to system/root users rather than the personal user.
pub fn is_system_file_or_dir(path: &Path) -> bool {
    // 1. Path hierarchy check
    if is_system_path(path) {
        return true;
    }

    // 2. Ownership check
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let my_uid = current_uid();

        if let Ok(meta) = std::fs::symlink_metadata(path) {
            let file_uid = meta.uid();
            // UID 0 (root) or UID < 1000 (system service accounts)
            if file_uid == 0 || file_uid < 1000 {
                return true;
            }
            // If running as regular user, files not owned by current user are not personal
            if my_uid != 0 && file_uid != my_uid {
                return true;
            }
        }
    }

    false
}

/// Validate if an item can be deleted by the current user.
pub fn check_deletion_permission(
    path: &Path,
    root_path: &Path,
) -> Result<bool, DeletionRestriction> {
    // Never allow deleting the filesystem root '/'
    if path == Path::new("/") {
        return Err(DeletionRestriction::RootFilesystem);
    }

    // Never allow deleting the analyzer's current scan root
    if path == root_path {
        return Err(DeletionRestriction::CurrentScanRoot);
    }

    let is_sys = is_system_file_or_dir(path);
    let superuser = is_superuser();

    // If it's a system file/folder and user is not superuser, block deletion
    if is_sys && !superuser {
        return Err(DeletionRestriction::SystemProtected {
            path: path.to_path_buf(),
        });
    }

    // Returns Ok(is_sys) indicating whether this item is a system resource
    Ok(is_sys)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_path_detection() {
        assert!(is_system_path(Path::new("/")));
        assert!(is_system_path(Path::new("/bin")));
        assert!(is_system_path(Path::new("/bin/sh")));
        assert!(is_system_path(Path::new("/etc")));
        assert!(is_system_path(Path::new("/etc/fstab")));
        assert!(is_system_path(Path::new("/usr/bin/cat")));
        assert!(is_system_path(Path::new("/var/log/syslog")));
        assert!(is_system_path(Path::new("/boot/vmlinuz")));
        assert!(is_system_path(Path::new("/home")));
        assert!(is_system_path(Path::new("/run/systemd")));

        // Personal paths should NOT be flagged as system paths
        assert!(!is_system_path(Path::new("/home/user/Downloads/movie.mkv")));
        assert!(!is_system_path(Path::new("/home/user/Projects/test")));
        assert!(!is_system_path(Path::new("/run/media/user/USB_DRIVE/file.txt")));
    }

    #[test]
    fn test_deletion_permission_root_prevention() {
        let scan_root = PathBuf::from("/home/user");
        let root = Path::new("/");
        let res = check_deletion_permission(root, &scan_root);
        assert_eq!(res, Err(DeletionRestriction::RootFilesystem));
    }

    #[test]
    fn test_deletion_permission_scan_root_prevention() {
        let scan_root = PathBuf::from("/home/user/projects");
        let target = Path::new("/home/user/projects");
        let res = check_deletion_permission(target, &scan_root);
        assert_eq!(res, Err(DeletionRestriction::CurrentScanRoot));
    }

    #[test]
    fn test_deletion_permission_system_path_for_regular_user() {
        let scan_root = PathBuf::from("/home/user");
        let etc_passwd = Path::new("/etc/passwd");
        let res = check_deletion_permission(etc_passwd, &scan_root);

        if !is_superuser() {
            assert!(matches!(res, Err(DeletionRestriction::SystemProtected { .. })));
        } else {
            assert_eq!(res, Ok(true));
        }
    }

    #[test]
    fn test_deletion_permission_personal_file() {
        // Create a temporary file in user's temp directory
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("arch_disk_tui_test_file.tmp");
        let _ = std::fs::write(&test_file, b"sample content");

        let scan_root = temp_dir.clone();
        let res = check_deletion_permission(&test_file, &scan_root);

        // Since the current user created this temp file, it should be deletable
        assert_eq!(res, Ok(false));

        // Clean up
        let _ = std::fs::remove_file(&test_file);
    }
}
