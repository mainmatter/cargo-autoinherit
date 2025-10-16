// Common test utilities shared across test modules

use std::path::{Path, PathBuf};
use assert_cmd::Command;

/// Gets the path to a fixture directory
pub fn fixture_path(name: &str) -> PathBuf {
    Path::new("tests").join("fixtures").join(name)
}

/// Creates a Command configured to run cargo-autoinherit in the specified directory
pub fn cargo_autoinherit<P: AsRef<Path>>(dir: P) -> Command {
    let mut cmd = Command::cargo_bin("cargo-autoinherit").unwrap();
    cmd.current_dir(dir);
    cmd.arg("autoinherit");
    cmd
}


#[cfg(test)]
mod tests {
    use super::*;
    use predicates::prelude::*;

    // Tests for the new-child-dep fixture in check mode

    /// Test that check mode correctly identifies a dependency in child
    /// that should be added to the workspace
    #[test]
    fn test_identifies_missing_workspace_dependency() {
        let path = fixture_path("new-child-dep");

        cargo_autoinherit(&path)
            .arg("--check")
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "`regex` should be declared as a workspace dependency"
            ));
    }

    /// Test that check mode can be instructed to ignore a specific package
    #[test]
    fn test_exclude_member_skips_checking() {
        let path = fixture_path("new-child-dep");

        cargo_autoinherit(&path)
            .args(&["--check", "--exclude-members", "child"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Excluded workspace member `child`"));
    }

    // Tests for the uninherited-existing-child-dep fixture in check mode

    /// Test that check mode correctly identifies a dependency that exists in the workspace
    /// but is redefined in a child package rather than being inherited
    #[test]
    fn test_identifies_uninherited_dependency() {
        let path = fixture_path("uninherited-existing-child-dep");

        cargo_autoinherit(&path)
            .arg("--check")
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "`regex` should inherit from a workspace dependency"
            ));
    }

    /// Test that "--exclude-members" flag works correctly
    #[test]
    fn test_exclude_flag_allows_uninherited_dependency() {
        let path = fixture_path("uninherited-existing-child-dep");

        cargo_autoinherit(&path)
            .args(&["--check", "--exclude-members", "child"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Excluded workspace member `child`"));
    }

    /// Test that check mode also works with the "--prefer-simple-dotted" flag
    #[test]
    fn test_check_with_prefer_simple_dotted() {
        let path = fixture_path("uninherited-existing-child-dep");

        cargo_autoinherit(&path)
            .args(&["--check", "--prefer-simple-dotted"])
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "`regex` should inherit from a workspace dependency"
            ));
    }

    /// Test that multiple sources are detected and reported properly
    #[test]
    fn test_identifies_multiple_sources() {
        let path = fixture_path("multiple-source-child-deps");

        cargo_autoinherit(&path)
            .arg("--check")
            .assert()
            .failure()
            .stderr(
                predicate::str::contains(
                    "`rand` won't be auto-inherited because there are multiple sources for it:"
                )
                .and(predicate::str::contains("version: ^0.8.0"))
                .and(predicate::str::contains("version: =0.8.0"))
            );
    }

    /// Test that the --allow-multiple-sources flag lets the check pass
    #[test]
    fn test_allow_multiple_sources_flag() {
        let path = fixture_path("multiple-source-child-deps");

        // Should still indicate the presence of multiple sources in output
        // (but it shouldn't be treated as an error)

        cargo_autoinherit(&path)
            .args(&["--check", "--allow-multiple-sources"])
            .assert()
            .success()
            .stderr(predicate::str::contains(
                "`rand` won't be auto-inherited because there are multiple sources for it:"
            ));
    }
}
