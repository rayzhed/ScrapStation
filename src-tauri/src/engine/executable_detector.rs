use std::path::Path;
use walkdir::WalkDir;

use super::library_tracker::{GameExecutable, ExeType};

/// Patterns that indicate an installer executable
const INSTALLER_PATTERNS: &[&str] = &[
    "unins", "setup", "install", "uninstall",
    "update", "patch", "autorun", "launcher_setup",
    "installer", "uninst", "remove"
];

/// Patterns that indicate a redistributable
const REDIST_PATTERNS: &[&str] = &[
    "vcredist", "vc_redist", "dxsetup", "dxwebsetup",
    "dotnet", "physx", "oalinst", "openal", "directx",
    "visual", "xnafx", "msvc", "redistributable",
    "prereq", "vc20", "d3d", "easyanticheat_setup"
];

/// Directories that contain redistributables
const REDIST_DIRS: &[&str] = &[
    "_commonredist", "commonredist", "redist",
    "redistributables", "directx", "vcredist",
    "__installer", "support", "prerequisites",
    "_redist", "dotnet", "physx", "setup",
    "third_party", "thirdparty", "_support"
];

/// Patterns that positively indicate a game executable
const GAME_EXE_PATTERNS: &[&str] = &[
    "game", "play", "start", "launch", "run", "main"
];

/// Preferred locations for game executables
const PREFERRED_DIRS: &[&str] = &[
    "bin", "binaries", "x64", "win64", "game",
    "bin64", "binary", "x86_64", "win32", "x86"
];

/// Launcher patterns (lower priority than main exe)
const LAUNCHER_PATTERNS: &[&str] = &[
    "launcher", "loader", "bootstrap", "starter"
];

/// Executable detector
pub struct ExecutableDetector;

impl ExecutableDetector {
    /// Detect all executables in a game folder and score them
    pub fn detect_executables(game_folder: &Path, game_title: &str) -> Vec<GameExecutable> {
        let mut executables = Vec::new();

        // Clean the game title for comparison
        let clean_title = Self::clean_title(game_title);

        // Walk the directory
        for entry in WalkDir::new(game_folder)
            .max_depth(5) // Don't go too deep
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Only process .exe files
            if !path.is_file() {
                continue;
            }

            let extension = path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            if extension != "exe" {
                continue;
            }

            // Get relative path from game folder
            let relative_path = path.strip_prefix(game_folder)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            let filename = path.file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Score and classify the executable
            let (score, exe_type) = Self::score_executable(path, &filename, &relative_path, &clean_title);

            executables.push(GameExecutable {
                path: relative_path,
                name: filename,
                score,
                exe_type,
            });
        }

        // Sort by score (highest first), then by path depth (shallower first)
        executables.sort_by(|a, b| {
            b.score.partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    let a_depth = a.path.matches(['/', '\\']).count();
                    let b_depth = b.path.matches(['/', '\\']).count();
                    a_depth.cmp(&b_depth)
                })
        });

        executables
    }

    /// Clean a game title for comparison
    fn clean_title(title: &str) -> String {
        title
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Score an executable and determine its type
    fn score_executable(
        path: &Path,
        filename: &str,
        relative_path: &str,
        clean_title: &str,
    ) -> (f32, ExeType) {
        let filename_lower = filename.to_lowercase();
        let path_lower = relative_path.to_lowercase();

        // Check if it's a redistributable (immediate disqualification)
        if Self::is_redistributable(&filename_lower, &path_lower) {
            return (0.0, ExeType::Redistributable);
        }

        // Check if it's an installer
        if Self::is_installer(&filename_lower) {
            return (0.05, ExeType::Installer);
        }

        // Check if it's in a redistributable directory
        if Self::in_redist_directory(&path_lower) {
            return (0.0, ExeType::Redistributable);
        }

        let mut score = 0.5f32;
        let mut exe_type = ExeType::Unknown;

        // Check if it's a launcher
        if Self::is_launcher(&filename_lower) {
            exe_type = ExeType::Launcher;
            score += 0.1; // Launchers are okay but not preferred
        }

        // Bonus: filename matches game title
        let title_similarity = Self::calculate_title_similarity(&filename_lower, clean_title);
        score += title_similarity * 0.35;

        if title_similarity > 0.5 {
            exe_type = ExeType::Main;
        }

        // Bonus: in root directory or shallow
        let depth = relative_path.matches(['/', '\\']).count();
        if depth == 0 {
            score += 0.15;
        } else if depth == 1 {
            score += 0.1;
        }

        // Bonus: in preferred directory
        if Self::in_preferred_directory(&path_lower) {
            score += 0.1;
        }

        // Bonus: has game-related keywords
        if GAME_EXE_PATTERNS.iter().any(|p| filename_lower.contains(p)) {
            score += 0.1;
            if exe_type == ExeType::Unknown {
                exe_type = ExeType::Main;
            }
        }

        // Small bonus for larger executables (main game exes are usually bigger)
        if let Ok(metadata) = path.metadata() {
            let size_mb = metadata.len() as f32 / (1024.0 * 1024.0);
            if size_mb > 50.0 {
                score += 0.1;
            } else if size_mb > 10.0 {
                score += 0.05;
            }
        }

        // Penalty for very short filenames (less likely to be main exe)
        if filename_lower.len() <= 3 {
            score -= 0.1;
        }

        // If still unknown and decent score, call it a tool
        if exe_type == ExeType::Unknown && score > 0.3 {
            exe_type = ExeType::Tool;
        }

        (score.clamp(0.0, 1.0), exe_type)
    }

    /// Check if filename indicates a redistributable
    fn is_redistributable(filename_lower: &str, path_lower: &str) -> bool {
        REDIST_PATTERNS.iter().any(|p| filename_lower.contains(p))
            || path_lower.contains("vcredist")
            || path_lower.contains("directx")
    }

    /// Check if filename indicates an installer
    fn is_installer(filename_lower: &str) -> bool {
        INSTALLER_PATTERNS.iter().any(|p| filename_lower.contains(p))
    }

    /// Check if executable is in a redistributable directory
    fn in_redist_directory(path_lower: &str) -> bool {
        REDIST_DIRS.iter().any(|dir| path_lower.contains(dir))
    }

    /// Check if filename indicates a launcher
    fn is_launcher(filename_lower: &str) -> bool {
        LAUNCHER_PATTERNS.iter().any(|p| filename_lower.contains(p))
    }

    /// Check if executable is in a preferred directory
    fn in_preferred_directory(path_lower: &str) -> bool {
        PREFERRED_DIRS.iter().any(|dir| path_lower.contains(dir))
    }

    /// Calculate similarity between filename and game title
    fn calculate_title_similarity(filename_lower: &str, clean_title: &str) -> f32 {
        if clean_title.is_empty() {
            return 0.0;
        }

        let title_words: Vec<&str> = clean_title.split_whitespace().collect();
        if title_words.is_empty() {
            return 0.0;
        }

        // Count matching words (minimum 3 chars to count)
        let matching_words = title_words.iter()
            .filter(|word| word.len() >= 3 && filename_lower.contains(*word))
            .count();

        // Calculate ratio
        let ratio = matching_words as f32 / title_words.len() as f32;

        // Bonus if filename starts with first word of title
        let first_word = title_words.first().unwrap_or(&"");
        if first_word.len() >= 3 && filename_lower.starts_with(first_word) {
            return (ratio + 0.2).min(1.0);
        }

        ratio
    }

    /// Get the best executable (highest score, not an installer/redist)
    pub fn get_best_executable(executables: &[GameExecutable]) -> Option<&GameExecutable> {
        executables.iter()
            .filter(|e| e.exe_type != ExeType::Installer && e.exe_type != ExeType::Redistributable)
            .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// Calculate total size of a directory
    pub fn calculate_directory_size(path: &Path) -> u64 {
        WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum()
    }
}
