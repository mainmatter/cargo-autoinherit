use std::{env::temp_dir, fs, path::PathBuf};

use cargo_autoinherit::{auto_inherit, AutoInheritConf};
use git2::Repository;
use insta::assert_snapshot;
use rand::{
    distributions::{Alphanumeric, DistString},
    thread_rng,
};

const TEST_REPO_URL: &str = match option_env!("TEST_REPO_URL") {
    Some(url) => url,
    None => "https://github.com/hdoordt/cargo-autoinherit-test",
};

fn clone_test_project() -> PathBuf {
    let dir: String = Alphanumeric.sample_string(&mut thread_rng(), 32);
    let dir = temp_dir().join(dir);
    Repository::clone(TEST_REPO_URL, &dir).expect("Error cloning test repository");
    dir
}

fn run_with_conf(conf: AutoInheritConf) -> PathBuf {
    let repo_path = clone_test_project();

    let conf = AutoInheritConf {
        manifest_path: Some(PathBuf::from(format!(
            "{}/Cargo.toml",
            repo_path.to_str().unwrap()
        ))),
        ..conf
    };
    auto_inherit(conf).unwrap();
    repo_path
}

#[test]
fn default_behavior() {
    let repo_path = run_with_conf(AutoInheritConf::default());
    insta::glob!(repo_path, "**/Cargo.toml", |p| {
        let input = fs::read_to_string(dbg!(p)).unwrap();
        assert_snapshot!(input);
    });
}

#[test]
fn prefer_simple_dotted() {
    let repo_path = run_with_conf(AutoInheritConf {
        prefer_simple_dotted: true,
        ..Default::default()
    });
    insta::glob!(repo_path, "**/Cargo.toml", |p| {
        let input = fs::read_to_string(dbg!(p)).unwrap();
        assert_snapshot!(input);
    });
}

#[test]
fn with_excluded_member() {
    let repo_path = run_with_conf(AutoInheritConf {
        prefer_simple_dotted: false,
        exclude_members: vec!["cargo-autoinherit-test-cli".to_string()],
        ..Default::default()
    });
    insta::glob!(repo_path, "**/Cargo.toml", |p| {
        let input = fs::read_to_string(dbg!(p)).unwrap();
        assert_snapshot!(input);
    });
}
