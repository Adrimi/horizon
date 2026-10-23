//! The size of a world's folder on disk, shown in the world list. The sum
//! is made on a background thread so the list opens at once; the cache is
//! read again every time the list asks after a few seconds.
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
};

const REFRESH: Duration = Duration::from_secs(5);

enum State {
    Counting(Option<u64>),
    Done(u64, Instant),
}

static SIZES: LazyLock<Mutex<HashMap<PathBuf, State>>> = LazyLock::new(Mutex::default);

/// 1024-based units with one decimal ("1.2 GB"); bytes stay whole ("512 B").
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

/// The sum of the sizes of the files under `dir`.
pub fn folder_size(dir: &Path) -> u64 {
    let mut total = 0;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(read) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in read.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                stack.push(entry.path());
            } else if kind.is_file() {
                total += entry.metadata().map_or(0, |m| m.len());
            }
        }
    }
    total
}

/// The size of the world folder as text, or `None` until it is counted
/// (a recount starts in the background when the cached one is old).
pub fn label(folder: &Path) -> Option<String> {
    let mut sizes = SIZES.lock().ok()?;
    let known = match sizes.get(folder) {
        Some(State::Counting(known)) => return known.map(format_size),
        Some(State::Done(bytes, at)) if at.elapsed() < REFRESH => {
            return Some(format_size(*bytes));
        }
        Some(State::Done(bytes, _)) => Some(*bytes),
        None => None,
    };
    sizes.insert(folder.to_path_buf(), State::Counting(known));
    let path = folder.to_path_buf();
    std::thread::spawn(move || {
        let bytes = folder_size(&path);
        if let Ok(mut sizes) = SIZES.lock() {
            sizes.insert(path, State::Done(bytes, Instant::now()));
        }
    });
    known.map(format_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_read_in_1024_units_with_one_decimal() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(340 * 1024 * 1024), "340.0 MB");
        assert_eq!(format_size(1288490189), "1.2 GB");
        assert_eq!(format_size(3 * 1024 * 1024 * 1024 * 1024), "3.0 TB");
    }

    #[test]
    fn a_folder_is_the_sum_of_its_files() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/world-size-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("region")).unwrap();
        std::fs::write(dir.join("level.dat"), [0u8; 100]).unwrap();
        std::fs::write(dir.join("region/r.0.0.mca"), [0u8; 400]).unwrap();
        assert_eq!(folder_size(&dir), 500);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
