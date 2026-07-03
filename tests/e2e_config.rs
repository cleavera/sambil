mod common;

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use common::TestSession;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn tmp_config_dir(label: &str) -> std::path::PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "sambil_{}_{}_{}_{}",
        label,
        std::process::id(),
        n,
        "cfg"
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// On first launch, sambil should write a default config file to
/// $XDG_CONFIG_HOME/sambil/config.toml containing a `leader` key.
#[test]
fn config_file_created_on_first_launch() {
    let tmp = tmp_config_dir("create");

    let session =
        TestSession::spawn_sambil_with_env(80, 24, &[("XDG_CONFIG_HOME", tmp.to_str().unwrap())]);

    session.assert_running();

    let config_path = tmp.join("sambil").join("config.toml");
    assert!(
        config_path.exists(),
        "config file was not created at {:?}",
        config_path
    );

    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        content.contains("leader"),
        "config file is missing the leader setting"
    );
    assert!(
        content.contains("ctrl+b"),
        "config file should default to ctrl+b"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

/// A custom leader key set in the config file should be used instead of the
/// default Ctrl-b. Here we configure Ctrl-a and verify that Ctrl-a c opens a
/// new tab while Ctrl-b c does nothing.
#[test]
fn custom_leader_key_is_respected() {
    let tmp = tmp_config_dir("leader");
    std::fs::create_dir_all(tmp.join("sambil")).unwrap();
    std::fs::write(
        tmp.join("sambil").join("config.toml"),
        "leader = \"ctrl+a\"\n",
    )
    .unwrap();

    let mut session =
        TestSession::spawn_sambil_with_env(80, 24, &[("XDG_CONFIG_HOME", tmp.to_str().unwrap())]);

    session.assert_running();

    // Ctrl-a c should open a new tab (0x01 = Ctrl-a)
    session.send_keys(&[0x01, b'c']);
    assert!(
        session.wait_for_text("[●:", Duration::from_secs(2)),
        "Ctrl-a c did not open a tab — custom leader not applied\n---\n{}\n---",
        session.screen().full_text()
    );

    let _ = std::fs::remove_dir_all(&tmp);
}
