//! File system utilities: binary detection, ranged read, atomic write, path helpers.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Check whether `path` looks like a binary file (contains `\0` in the first 8 KiB).
pub fn is_binary_file(path: &Path) -> Result<bool> {
    use std::io::Read;
    let mut f = std::fs::File::open(path)
        .with_context(|| format!("cannot open {}", path.display()))?;
    let mut buf = [0u8; 8192];
    let n = f.read(&mut buf)?;
    Ok(buf[..n].contains(&0))
}

/// Read lines `start..=end` from `path` (1-indexed, inclusive).
pub fn read_file_in_range(path: &Path, start: usize, end: usize) -> Result<String> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read {}", path.display()))?;
    let lines: Vec<&str> = content.lines().collect();
    let start = start.saturating_sub(1);
    let end = end.min(lines.len());
    if start >= lines.len() {
        return Ok(String::new());
    }
    Ok(lines[start..end].join("\n"))
}

/// Write `content` atomically: write to a temporary sibling file, then rename.
pub fn atomic_write(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, content)
        .with_context(|| format!("writing temp file {}", tmp.display()))?;
    std::fs::rename(&tmp, path)
        .with_context(|| format!("rename {} → {}", tmp.display(), path.display()))?;
    Ok(())
}

/// Ensure `path` exists as a directory (creates parents as needed).
pub fn ensure_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)
            .with_context(|| format!("cannot create dir {}", path.display()))?;
    }
    Ok(())
}

/// Resolve a potentially relative `rel` against `base`, expanding leading `~`.
#[must_use]
pub fn resolve_path(base: &Path, rel: &str) -> PathBuf {
    if let Some(stripped) = rel.strip_prefix("~/") {
        return super::platform::home_dir().join(stripped);
    }
    if rel == "~" {
        return super::platform::home_dir();
    }
    let p = Path::new(rel);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        base.join(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_is_binary_text_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("text.txt");
        std::fs::write(&path, "Hello, world!\n").unwrap();
        assert!(!is_binary_file(&path).unwrap());
    }

    #[test]
    fn test_is_binary_binary_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bin.dat");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(&[0x89, 0x50, 0x4e, 0x47, 0x00, 0x01]).unwrap();
        assert!(is_binary_file(&path).unwrap());
    }

    #[test]
    fn test_read_file_in_range() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("lines.txt");
        std::fs::write(&path, "line1\nline2\nline3\nline4\n").unwrap();
        let result = read_file_in_range(&path, 2, 3).unwrap();
        assert_eq!(result, "line2\nline3");
    }

    #[test]
    fn test_atomic_write() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.txt");
        atomic_write(&path, "hello").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn test_resolve_path_absolute() {
        let base = Path::new("/some/base");
        let result = resolve_path(base, "child/file.txt");
        assert!(result.ends_with("child/file.txt"));
    }

    #[test]
    fn test_is_binary_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.dat");
        std::fs::write(&path, b"").unwrap();
        assert!(!is_binary_file(&path).unwrap());
    }

    #[test]
    fn test_read_file_in_range_start_greater_than_end() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("lines.txt");
        std::fs::write(&path, "a\nb\nc\n").unwrap();
        // start(5) > total lines(3) → empty
        let result = read_file_in_range(&path, 5, 10).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_read_file_in_range_end_beyond_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("lines.txt");
        std::fs::write(&path, "a\nb\nc\n").unwrap();
        let result = read_file_in_range(&path, 2, 100).unwrap();
        assert_eq!(result, "b\nc");
    }

    #[test]
    fn test_ensure_dir_creates_nested() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b").join("c");
        ensure_dir(&nested).unwrap();
        assert!(nested.is_dir());
    }

    #[test]
    fn test_atomic_write_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("deep").join("file.txt");
        atomic_write(&path, "content").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "content");
    }
}
