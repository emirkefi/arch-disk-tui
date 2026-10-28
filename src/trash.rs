use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

/// Move a file or directory to the FreeDesktop.org Trash directory
/// (`$XDG_DATA_HOME/Trash` or `~/.local/share/Trash`).
pub fn move_to_trash(target: &Path) -> io::Result<PathBuf> {
    let abs_target = if target.is_absolute() {
        target.to_path_buf()
    } else {
        std::env::current_dir()?.join(target)
    };

    let trash_base = get_trash_dir()?;
    let files_dir = trash_base.join("files");
    let info_dir = trash_base.join("info");

    fs::create_dir_all(&files_dir)?;
    fs::create_dir_all(&info_dir)?;

    let orig_name = abs_target
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "item".to_string());

    // Find a unique name in Trash/files and Trash/info
    let mut candidate_name = orig_name.clone();
    let mut counter = 1u32;
    while files_dir.join(&candidate_name).symlink_metadata().is_ok()
        || info_dir
            .join(format!("{candidate_name}.trashinfo"))
            .symlink_metadata()
            .is_ok()
    {
        candidate_name = format!("{orig_name}.{counter}");
        counter += 1;
    }

    let info_path = info_dir.join(format!("{candidate_name}.trashinfo"));
    let dest_path = files_dir.join(&candidate_name);

    // Format deletion timestamp (ISO 8601 local/UTC YYYY-MM-DDTHH:MM:SS)
    let timestamp = format_iso8601_now();
    let encoded_path = url_encode_path(&abs_target.to_string_lossy());
    let trashinfo_content = format!(
        "[Trash Info]\nPath={encoded_path}\nDeletionDate={timestamp}\n"
    );

    fs::write(&info_path, trashinfo_content)?;

    // Attempt atomic rename first; if cross-device (EXDEV), copy and remove
    if let Err(err) = fs::rename(&abs_target, &dest_path) {
        if err.raw_os_error() == Some(libc::EXDEV) {
            if let Err(copy_err) = copy_and_delete(&abs_target, &dest_path) {
                let _ = fs::remove_file(&info_path);
                return Err(copy_err);
            }
        } else {
            let _ = fs::remove_file(&info_path);
            return Err(err);
        }
    }

    Ok(dest_path)
}

fn get_trash_dir() -> io::Result<PathBuf> {
    if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
        if !xdg_data.trim().is_empty() {
            return Ok(PathBuf::from(xdg_data).join("Trash"));
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.trim().is_empty() {
            return Ok(PathBuf::from(home).join(".local/share/Trash"));
        }
    }
    Ok(std::env::temp_dir().join(format!("Trash-{}", unsafe { libc::geteuid() })))
}

fn copy_and_delete(src: &Path, dst: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(src)?;
    if meta.file_type().is_symlink() {
        #[cfg(unix)]
        {
            let target = fs::read_link(src)?;
            std::os::unix::fs::symlink(target, dst)?;
            fs::remove_file(src)?;
            return Ok(());
        }
    }
    if meta.is_dir() {
        copy_dir_recursive(src, dst)?;
        fs::remove_dir_all(src)?;
    } else {
        fs::copy(src, dst)?;
        fs::remove_file(src)?;
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if file_type.is_symlink() {
            #[cfg(unix)]
            {
                let link_target = fs::read_link(&src_path)?;
                std::os::unix::fs::symlink(link_target, &dst_path)?;
            }
        } else if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn url_encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for b in path.bytes() {
        match b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'/'
            | b'-'
            | b'_'
            | b'.'
            | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn format_iso8601_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as libc::time_t)
        .unwrap_or(0);
    unsafe {
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&secs, &mut tm);
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            tm.tm_year + 1900,
            tm.tm_mon + 1,
            tm.tm_mday,
            tm.tm_hour,
            tm.tm_min,
            tm.tm_sec
        )
    }
}

/// Copy text to system clipboard via external utilities (wl-copy, xclip, xsel)
/// and OSC 52 escape sequence fallback.
pub fn copy_to_clipboard(text: &str) -> bool {
    let mut copied = false;

    // 1. Try Wayland / X11 clipboard utilities
    let commands: &[(&str, &[&str])] = &[
        ("wl-copy", &[]),
        ("xclip", &["-selection", "clipboard"]),
        ("xsel", &["--clipboard", "--input"]),
    ];

    for (cmd, args) in commands {
        if let Ok(mut child) = Command::new(cmd)
            .args(*args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                if stdin.write_all(text.as_bytes()).is_ok() {
                    drop(stdin);
                    if let Ok(status) = child.wait() {
                        if status.success() {
                            copied = true;
                            break;
                        }
                    }
                }
            }
        }
    }

    // 2. Also emit OSC 52 terminal escape sequence (supported by Kitty, Alacritty, WezTerm, Foot, tmux)
    let encoded = base64_encode(text.as_bytes());
    let mut stdout = io::stdout();
    if write!(stdout, "\x1b]52;c;{encoded}\x07").is_ok() && stdout.flush().is_ok() {
        copied = true;
    }

    copied
}

fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        out.push(TABLE[((triple >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_to_trash_creates_trashinfo_and_moves_file() {
        let temp_base = std::env::temp_dir().join("arch_disk_tui_trash_test");
        let _ = fs::remove_dir_all(&temp_base);
        fs::create_dir_all(&temp_base).unwrap();

        let fake_xdg = temp_base.join("xdg_data");
        unsafe {
            std::env::set_var("XDG_DATA_HOME", &fake_xdg);
        }

        let sample_file = temp_base.join("hello test.txt");
        fs::write(&sample_file, b"trash content").unwrap();

        let trashed_path = move_to_trash(&sample_file).unwrap();
        assert!(!sample_file.exists());
        assert!(trashed_path.exists());

        let info_file = fake_xdg.join("Trash/info/hello test.txt.trashinfo");
        assert!(info_file.exists());
        let info_text = fs::read_to_string(info_file).unwrap();
        assert!(info_text.starts_with("[Trash Info]\nPath="));
        assert!(info_text.contains("hello%20test.txt"));
        assert!(info_text.contains("DeletionDate="));

        let _ = fs::remove_dir_all(&temp_base);
    }

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode(b"/home/user/test"), "L2hvbWUvdXNlci90ZXN0");
    }
}
