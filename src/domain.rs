use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use color_eyre::{Result, eyre::eyre};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendStatus {
    Available,
    Missing,
}

#[derive(Debug, Clone)]
pub struct ParuCacheSummary {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub path: PathBuf,
    pub pkgbuild_path: Option<PathBuf>,
    pub has_pkgbuild: bool,
    pub has_git_metadata: bool,
    pub package_archives: usize,
    pub source_archives: usize,
}

#[derive(Debug, Clone, Default)]
pub struct ParuCacheDetails {
    pub total_files: usize,
    pub total_bytes: u64,
    pub package_archives: usize,
    pub source_archives: usize,
    pub sample_files: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SystemSnapshot {
    pub pacman: BackendStatus,
    pub paru: BackendStatus,
    pub paru_clone_dir: Option<PathBuf>,
    pub paru_cache: Vec<ParuCacheSummary>,
}

impl SystemSnapshot {
    pub fn detect() -> Self {
        let paru_clone_dir = detect_paru_clone_dir();
        let paru_cache = paru_clone_dir
            .as_ref()
            .map(|clone_dir| load_paru_cache_index(clone_dir))
            .unwrap_or_default();

        Self {
            pacman: detect_command("pacman"),
            paru: detect_command("paru"),
            paru_clone_dir,
            paru_cache,
        }
    }
}

pub fn matches_keyword(entry: &ParuCacheSummary, keyword: &str) -> bool {
    let keyword = keyword.trim();
    if keyword.is_empty() {
        return true;
    }

    let haystack = format!(
        "{}\n{}\n{}\n{}",
        entry.name.to_ascii_lowercase(),
        entry
            .version
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase(),
        entry
            .description
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase(),
        entry
            .url
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase(),
    );

    keyword
        .split_whitespace()
        .all(|token| haystack.contains(&token.to_ascii_lowercase()))
}

pub fn inspect_cache(entry: &ParuCacheSummary) -> ParuCacheDetails {
    let mut details = ParuCacheDetails::default();
    let mut stack = vec![entry.path.clone()];

    while let Some(dir) = stack.pop() {
        let Ok(read_dir) = fs::read_dir(&dir) else {
            continue;
        };

        for child in read_dir.flatten() {
            let Ok(file_type) = child.file_type() else {
                continue;
            };

            let file_name = child.file_name().to_string_lossy().to_string();
            let path = child.path();

            if file_type.is_dir() {
                if file_name == ".git" {
                    continue;
                }
                stack.push(path);
                continue;
            }

            details.total_files += 1;
            if let Ok(metadata) = child.metadata() {
                details.total_bytes += metadata.len();
            }

            match classify_cache_file(&file_name) {
                CacheFileKind::PackageArchive => details.package_archives += 1,
                CacheFileKind::SourceArchive => details.source_archives += 1,
                CacheFileKind::Other => {}
            }

            if details.sample_files.len() < 8 {
                if let Ok(relative) = path.strip_prefix(&entry.path) {
                    details.sample_files.push(relative.display().to_string());
                }
            }
        }
    }

    details.sample_files.sort();
    details
}

pub fn trash_paru_cache(entry: &ParuCacheSummary, clone_dir: &Path) -> Result<()> {
    let target = entry.path.canonicalize().map_err(|error| {
        eyre!(
            "failed to resolve cache path {}: {error}",
            entry.path.display()
        )
    })?;
    let clone_dir = clone_dir.canonicalize().map_err(|error| {
        eyre!(
            "failed to resolve clone dir {}: {error}",
            clone_dir.display()
        )
    })?;

    if target.parent() != Some(clone_dir.as_path()) {
        return Err(eyre!(
            "refusing to trash {} because it is not a direct child of {}",
            target.display(),
            clone_dir.display()
        ));
    }

    if !target.is_dir() {
        return Err(eyre!(
            "refusing to trash {} because it is not a directory",
            target.display()
        ));
    }

    let status = Command::new("gio")
        .arg("trash")
        .arg("--")
        .arg(&target)
        .status()
        .map_err(|error| {
            eyre!(
                "failed to execute gio trash for {}: {error}",
                target.display()
            )
        })?;

    if !status.success() {
        return Err(eyre!(
            "gio trash failed for {} with status {}",
            target.display(),
            status
        ));
    }

    Ok(())
}

fn detect_command(command: &str) -> BackendStatus {
    let status = Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {command} >/dev/null 2>&1"))
        .status();

    match status {
        Ok(status) if status.success() => BackendStatus::Available,
        _ => BackendStatus::Missing,
    }
}

fn detect_paru_clone_dir() -> Option<PathBuf> {
    detect_configured_clone_dir().or_else(default_paru_clone_dir)
}

fn detect_configured_clone_dir() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(config_home) = config_home_dir() {
        candidates.push(config_home.join("paru").join("paru.conf"));
    }
    if let Some(home_dir) = home_dir() {
        candidates.push(home_dir.join(".config/paru/paru.conf"));
    }
    candidates.push(PathBuf::from("/etc/paru.conf"));

    for candidate in candidates {
        let Ok(contents) = fs::read_to_string(candidate) else {
            continue;
        };

        if let Some(clone_dir) = parse_clone_dir_from_config(&contents) {
            return Some(expand_tilde(&clone_dir));
        }
    }

    None
}

fn default_paru_clone_dir() -> Option<PathBuf> {
    let base = env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|dir| dir.join(".cache")))?;

    Some(base.join("paru").join("clone"))
}

fn config_home_dir() -> Option<PathBuf> {
    env::var_os("XDG_CONFIG_HOME").map(PathBuf::from)
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home_dir) = home_dir() {
            return home_dir.join(rest);
        }
    }

    PathBuf::from(path)
}

fn parse_clone_dir_from_config(contents: &str) -> Option<String> {
    for line in contents.lines() {
        let line = line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        if key.trim().eq_ignore_ascii_case("CloneDir") {
            return Some(clean_assignment_value(value));
        }
    }

    None
}

fn load_paru_cache_index(clone_dir: &Path) -> Vec<ParuCacheSummary> {
    let Ok(read_dir) = fs::read_dir(clone_dir) else {
        return Vec::new();
    };

    let mut entries = read_dir
        .flatten()
        .filter_map(|entry| summarize_cache_dir(&entry.path()))
        .collect::<Vec<_>>();

    entries.sort_by(|left, right| left.name.cmp(&right.name));
    entries
}

fn summarize_cache_dir(path: &Path) -> Option<ParuCacheSummary> {
    if !path.is_dir() {
        return None;
    }

    let name = path.file_name()?.to_string_lossy().to_string();
    if name.starts_with('.') {
        return None;
    }

    let pkgbuild_path = path.join("PKGBUILD");
    let has_pkgbuild = pkgbuild_path.is_file();
    let metadata = if has_pkgbuild {
        parse_pkgbuild_file(&pkgbuild_path)
    } else {
        PkgbuildMetadata::default()
    };
    let (package_archives, source_archives, has_git_metadata) = scan_top_level_cache(path);

    Some(ParuCacheSummary {
        name,
        version: metadata.version,
        description: metadata.description,
        url: metadata.url,
        path: path.to_path_buf(),
        pkgbuild_path: has_pkgbuild.then_some(pkgbuild_path),
        has_pkgbuild,
        has_git_metadata,
        package_archives,
        source_archives,
    })
}

fn parse_pkgbuild_file(path: &Path) -> PkgbuildMetadata {
    let Ok(contents) = fs::read_to_string(path) else {
        return PkgbuildMetadata::default();
    };

    parse_pkgbuild_metadata(&contents)
}

fn parse_pkgbuild_metadata(contents: &str) -> PkgbuildMetadata {
    let mut metadata = PkgbuildMetadata::default();

    for line in contents.lines() {
        let line = line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }

        if metadata.version.is_none() {
            metadata.version = parse_assignment(line, "pkgver");
        }
        if metadata.description.is_none() {
            metadata.description = parse_assignment(line, "pkgdesc");
        }
        if metadata.url.is_none() {
            metadata.url = parse_assignment(line, "url");
        }

        if metadata.version.is_some() && metadata.description.is_some() && metadata.url.is_some() {
            break;
        }
    }

    metadata
}

fn parse_assignment(line: &str, key: &str) -> Option<String> {
    let (left, right) = line.split_once('=')?;
    if left.trim() != key {
        return None;
    }

    Some(clean_assignment_value(right))
}

fn clean_assignment_value(value: &str) -> String {
    let value = value.trim();

    if let Some(stripped) = value
        .strip_prefix('"')
        .and_then(|item| item.strip_suffix('"'))
    {
        return stripped.to_string();
    }

    if let Some(stripped) = value
        .strip_prefix('\'')
        .and_then(|item| item.strip_suffix('\''))
    {
        return stripped.to_string();
    }

    value.to_string()
}

fn scan_top_level_cache(path: &Path) -> (usize, usize, bool) {
    let Ok(read_dir) = fs::read_dir(path) else {
        return (0, 0, false);
    };

    let mut package_archives = 0;
    let mut source_archives = 0;
    let mut has_git_metadata = false;

    for entry in read_dir.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_string();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            if file_name == ".git" {
                has_git_metadata = true;
            }
            continue;
        }

        match classify_cache_file(&file_name) {
            CacheFileKind::PackageArchive => package_archives += 1,
            CacheFileKind::SourceArchive => source_archives += 1,
            CacheFileKind::Other => {}
        }
    }

    (package_archives, source_archives, has_git_metadata)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CacheFileKind {
    PackageArchive,
    SourceArchive,
    Other,
}

fn classify_cache_file(file_name: &str) -> CacheFileKind {
    if file_name.contains(".pkg.tar") {
        return CacheFileKind::PackageArchive;
    }

    const SOURCE_SUFFIXES: &[&str] = &[
        ".deb", ".rpm", ".apk", ".zip", ".7z", ".rar", ".tar", ".tar.gz", ".tgz", ".tar.xz",
        ".txz", ".tar.bz2", ".tbz", ".tbz2", ".tar.zst", ".tzst", ".gz", ".xz", ".bz2", ".zst",
    ];

    if SOURCE_SUFFIXES
        .iter()
        .any(|suffix| file_name.ends_with(suffix))
    {
        return CacheFileKind::SourceArchive;
    }

    CacheFileKind::Other
}

#[derive(Debug, Clone, Default)]
struct PkgbuildMetadata {
    version: Option<String>,
    description: Option<String>,
    url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env::temp_dir, process};

    #[test]
    fn parses_pkgbuild_metadata_fields() {
        let metadata = parse_pkgbuild_metadata(
            r#"
pkgver=1.125.0
pkgdesc="Visual Studio Code"
url='https://code.visualstudio.com/'
"#,
        );

        assert_eq!(metadata.version.as_deref(), Some("1.125.0"));
        assert_eq!(metadata.description.as_deref(), Some("Visual Studio Code"));
        assert_eq!(
            metadata.url.as_deref(),
            Some("https://code.visualstudio.com/")
        );
    }

    #[test]
    fn matches_keyword_across_name_and_metadata() {
        let entry = ParuCacheSummary {
            name: "visual-studio-code-bin".to_string(),
            version: Some("1.125.0".to_string()),
            description: Some("Editor for modern apps".to_string()),
            url: Some("https://code.visualstudio.com/".to_string()),
            path: PathBuf::from("/tmp/visual-studio-code-bin"),
            pkgbuild_path: Some(PathBuf::from("/tmp/visual-studio-code-bin/PKGBUILD")),
            has_pkgbuild: true,
            has_git_metadata: true,
            package_archives: 2,
            source_archives: 3,
        };

        assert!(matches_keyword(&entry, "code 1.125"));
        assert!(matches_keyword(&entry, "modern"));
        assert!(!matches_keyword(&entry, "firefox"));
    }

    #[test]
    fn classifies_package_and_source_archives() {
        assert_eq!(
            classify_cache_file("visual-studio-code-bin-1.125.0-1-x86_64.pkg.tar.zst"),
            CacheFileKind::PackageArchive
        );
        assert_eq!(
            classify_cache_file("code_1.125.0_amd64.deb"),
            CacheFileKind::SourceArchive
        );
        assert_eq!(classify_cache_file("PKGBUILD"), CacheFileKind::Other);
    }

    #[test]
    fn parses_clonedir_from_paru_config() {
        let config = r#"
[options]
CloneDir = ~/.cache/custom-paru
"#;

        assert_eq!(
            parse_clone_dir_from_config(config).as_deref(),
            Some("~/.cache/custom-paru")
        );
    }

    #[test]
    fn rejects_trashing_outside_clone_dir() {
        let temp_dir = temp_dir().join(format!("pacc-test-outside-{}", process::id()));
        let clone_dir = temp_dir.join("clone");
        let outside_dir = temp_dir.join("outside");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&clone_dir).expect("create clone dir");
        fs::create_dir_all(&outside_dir).expect("create outside dir");

        let entry = ParuCacheSummary {
            name: "outside".to_string(),
            version: None,
            description: None,
            url: None,
            path: outside_dir.clone(),
            pkgbuild_path: None,
            has_pkgbuild: false,
            has_git_metadata: false,
            package_archives: 0,
            source_archives: 0,
        };

        let error = trash_paru_cache(&entry, &clone_dir).expect_err("must reject");
        assert!(error.to_string().contains("not a direct child"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn rejects_trashing_missing_dir() {
        let temp_dir = temp_dir().join(format!("pacc-test-missing-{}", process::id()));
        let clone_dir = temp_dir.join("clone");
        let missing_dir = clone_dir.join("missing");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&clone_dir).expect("create clone dir");

        let entry = ParuCacheSummary {
            name: "missing".to_string(),
            version: None,
            description: None,
            url: None,
            path: missing_dir.clone(),
            pkgbuild_path: None,
            has_pkgbuild: false,
            has_git_metadata: false,
            package_archives: 0,
            source_archives: 0,
        };

        let error = trash_paru_cache(&entry, &clone_dir).expect_err("must reject");
        assert!(error.to_string().contains("failed to resolve cache path"));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
