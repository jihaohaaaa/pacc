use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendStatus {
    Available,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageSource {
    Pacman,
    Paru,
    Aur,
}

#[derive(Debug, Clone)]
pub struct PackageSummary {
    pub name: String,
    pub version: String,
    pub source: PackageSource,
    pub has_update: bool,
}

#[derive(Debug, Clone)]
pub struct SystemSnapshot {
    pub pacman: BackendStatus,
    pub paru: BackendStatus,
    pub packages: Vec<PackageSummary>,
}

impl SystemSnapshot {
    pub fn detect() -> Self {
        Self {
            pacman: detect_command("pacman"),
            paru: detect_command("paru"),
            packages: demo_packages(),
        }
    }
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

fn demo_packages() -> Vec<PackageSummary> {
    vec![
        PackageSummary {
            name: "linux".to_string(),
            version: "6.16.8.arch1-1".to_string(),
            source: PackageSource::Pacman,
            has_update: false,
        },
        PackageSummary {
            name: "neovim".to_string(),
            version: "0.11.4-1".to_string(),
            source: PackageSource::Pacman,
            has_update: true,
        },
        PackageSummary {
            name: "paru".to_string(),
            version: "2.1.0-1".to_string(),
            source: PackageSource::Paru,
            has_update: false,
        },
        PackageSummary {
            name: "visual-studio-code-bin".to_string(),
            version: "1.102.0-1".to_string(),
            source: PackageSource::Aur,
            has_update: true,
        },
    ]
}
