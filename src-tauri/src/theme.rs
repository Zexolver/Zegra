//! Custom user CSS theme loading, per the README's promise that users can
//! bring their own CSS theme/layout file.

use std::path::Path;

const MAX_THEME_SIZE_BYTES: u64 = 2 * 1024 * 1024; // 2 MiB is generous for a CSS file

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ThemeError {
    #[error("theme file not found: {0}")]
    NotFound(String),
    #[error("theme file must have a .css extension: {0}")]
    NotCss(String),
    #[error("theme file is too large (max {} bytes)", MAX_THEME_SIZE_BYTES)]
    TooLarge,
    #[error("failed to read theme file: {0}")]
    Read(String),
}

/// Reads and returns the contents of a user-supplied CSS theme file, with
/// basic validation so a malformed setting can't hand the frontend something
/// unexpected (wrong extension, missing file, absurdly large file).
pub fn load_theme_css(path: &Path) -> Result<String, ThemeError> {
    if path.extension().and_then(|e| e.to_str()) != Some("css") {
        return Err(ThemeError::NotCss(path.to_string_lossy().to_string()));
    }
    let metadata = std::fs::metadata(path)
        .map_err(|_| ThemeError::NotFound(path.to_string_lossy().to_string()))?;
    if metadata.len() > MAX_THEME_SIZE_BYTES {
        return Err(ThemeError::TooLarge);
    }
    std::fs::read_to_string(path).map_err(|e| ThemeError::Read(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn loads_valid_css_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("mytheme.css");
        std::fs::write(&path, "body { background: black; }").unwrap();
        let css = load_theme_css(&path).unwrap();
        assert!(css.contains("background: black"));
    }

    #[test]
    fn rejects_non_css_extension() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("mytheme.txt");
        std::fs::write(&path, "body {}").unwrap();
        assert_eq!(
            load_theme_css(&path),
            Err(ThemeError::NotCss(path.to_string_lossy().to_string()))
        );
    }

    #[test]
    fn missing_file_errors() {
        let path = Path::new("/does/not/exist/theme.css");
        assert!(matches!(load_theme_css(path), Err(ThemeError::NotFound(_))));
    }

    #[test]
    fn oversized_file_is_rejected() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("huge.css");
        let big = vec![b'a'; (MAX_THEME_SIZE_BYTES + 1) as usize];
        std::fs::write(&path, big).unwrap();
        assert_eq!(load_theme_css(&path), Err(ThemeError::TooLarge));
    }
}
