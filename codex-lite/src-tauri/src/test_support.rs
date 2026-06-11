use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

static ENV_LOCK: Mutex<()> = Mutex::new(());

pub struct TestEnv {
    pub root: PathBuf,
    _data_dir: PathBuf,
    _codex_home: PathBuf,
    _guard: MutexGuard<'static, ()>,
}

impl TestEnv {
    pub fn new(name: &str) -> Self {
        let guard = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let root =
            std::env::temp_dir().join(format!("codex-lite-test-{}-{}", name, uuid::Uuid::new_v4()));
        let data_dir = root.join("data");
        let codex_home = root.join("codex-home");
        std::fs::create_dir_all(&data_dir).expect("test data dir should be created");
        std::fs::create_dir_all(&codex_home).expect("test codex home should be created");
        std::env::set_var("CODEX_LITE_DATA_DIR", &data_dir);
        std::env::set_var("CODEX_LITE_CODEX_HOME", &codex_home);

        Self {
            root,
            _data_dir: data_dir,
            _codex_home: codex_home,
            _guard: guard,
        }
    }

    pub fn fixture_path(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("src-tauri should have a project parent")
            .join("fixtures")
            .join("redacted-auth")
            .join(name)
    }

    pub fn fixture_content(name: &str) -> String {
        std::fs::read_to_string(Self::fixture_path(name)).expect("fixture should be readable")
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        std::env::remove_var("CODEX_LITE_DATA_DIR");
        std::env::remove_var("CODEX_LITE_CODEX_HOME");
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
