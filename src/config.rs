use std::env;
use std::path::PathBuf;

pub const NIFFLER_HOME_ENV: &str = "NIFFLER_HOME";

pub fn data_root() -> PathBuf {
    data_root_from_env(env::var_os(NIFFLER_HOME_ENV).map(PathBuf::from), home_dir())
}

pub fn data_root_from_env(env_root: Option<PathBuf>, home: Option<PathBuf>) -> PathBuf {
    if let Some(path) = env_root {
        return path;
    }

    home.unwrap_or_else(|| PathBuf::from(".")).join(".niffler")
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_root_overrides_default_home_root() {
        let root = data_root_from_env(
            Some(PathBuf::from("/tmp/custom")),
            Some(PathBuf::from("/home/me")),
        );

        assert_eq!(root, PathBuf::from("/tmp/custom"));
    }

    #[test]
    fn default_root_is_niffler_directory_under_home() {
        let root = data_root_from_env(None, Some(PathBuf::from("/home/me")));

        assert_eq!(root, PathBuf::from("/home/me/.niffler"));
    }
}
