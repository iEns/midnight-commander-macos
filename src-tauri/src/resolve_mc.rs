use std::path::{Path, PathBuf};
use std::{env, fs};

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ResolveMcError {
    #[error("mc (Midnight Commander) not found. Install with: brew install midnight-commander")]
    NotFound,
    #[error("failed to canonicalize mc path: {0}")]
    Canonicalize(String),
}

pub fn default_search_paths(home: &str) -> Vec<PathBuf> {
    vec![
        PathBuf::from("/opt/homebrew/bin/mc"),
        PathBuf::from("/usr/local/bin/mc"),
        PathBuf::from(format!("{home}/.homebrew/bin/mc")),
        PathBuf::from(format!("{home}/.homebrew/opt/midnight-commander/bin/mc")),
    ]
}

pub fn gui_safe_path_prefix(home: &str) -> String {
    format!("/opt/homebrew/bin:/usr/local/bin:{home}/.homebrew/bin:/usr/bin:/bin")
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    fs::metadata(path)
        .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

fn canonicalize_existing(path: &Path) -> Result<PathBuf, ResolveMcError> {
    fs::canonicalize(path).map_err(|err| ResolveMcError::Canonicalize(err.to_string()))
}

pub fn resolve_mc_from_paths(search_paths: &[PathBuf]) -> Result<PathBuf, ResolveMcError> {
    for candidate in search_paths {
        if is_executable(candidate) {
            return canonicalize_existing(candidate);
        }
    }
    Err(ResolveMcError::NotFound)
}

pub fn resolve_mc_with_home(home: &str, path_env: Option<&str>) -> Result<PathBuf, ResolveMcError> {
    let mut search_paths = default_search_paths(home);

    if let Some(path) = path_env {
        for dir in path.split(':').filter(|part| !part.is_empty()) {
            search_paths.push(PathBuf::from(dir).join("mc"));
        }
    }

    resolve_mc_from_paths(&search_paths)
}

pub fn resolve_mc() -> Result<PathBuf, ResolveMcError> {
    let home = env::var("HOME").unwrap_or_else(|_| "/".to_string());
    let path_env = env::var("PATH").ok();
    resolve_mc_with_home(&home, path_env.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    #[test]
    fn resolves_real_mc_on_this_machine() {
        let path = resolve_mc().expect("mc should be installed on the build host");
        assert!(path.is_absolute());
        assert!(path.exists());
        assert!(is_executable(&path));
    }

    #[test]
    fn resolves_with_empty_path_env() {
        let home = env::var("HOME").expect("HOME must be set for test");
        let path =
            resolve_mc_with_home(&home, Some("")).expect("mc should resolve without PATH");
        assert!(path.is_absolute());
        assert!(path.exists());
    }

    #[test]
    fn returns_not_found_for_missing_candidates() {
        let dir = tempdir().expect("tempdir");
        let missing = dir.path().join("definitely-not-mc");
        let err = resolve_mc_from_paths(&[missing]).unwrap_err();
        assert_eq!(err, ResolveMcError::NotFound);
    }

    #[test]
    fn resolves_custom_executable_path() {
        let dir = tempdir().expect("tempdir");
        let fake_mc = dir.path().join("mc");
        fs::write(&fake_mc, b"#!/bin/sh\n").expect("write fake mc");
        let mut perms = fs::metadata(&fake_mc).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&fake_mc, perms).expect("chmod");

        let resolved = resolve_mc_from_paths(&[fake_mc.clone()]).expect("resolve fake mc");
        assert_eq!(resolved, fake_mc.canonicalize().expect("canonicalize"));
    }
}