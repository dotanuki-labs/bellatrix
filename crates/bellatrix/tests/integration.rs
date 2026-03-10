// Copyright 2026 Dotanuki Labs
// SPDX-License-Identifier: AGPL-3.0-or-later

use assert_cmd::Command;
use httpmock::MockServer;
use predicates::str::contains;
use std::path::PathBuf;

fn sut() -> Command {
    assert_cmd::cargo::cargo_bin_cmd!("bellatrix")
}

#[test]
fn should_show_help() {
    let description = "Usage: bellatrix";

    let execution = sut().arg("--help").assert();
    execution.stdout(contains(description));
}

#[test]
#[allow(unsafe_code)]
fn should_check_updates_for_existing_forks() {
    let server = MockServer::start();
    let recordings = setup_playback("check-forks.yaml");
    server.playback(recordings);

    unsafe {
        std::env::set_var("GITHUB_API_URL", server.base_url());
        std::env::set_var("GITHUB_TOKEN", "fake-api-token")
    }

    let execution = sut().arg("check").assert();

    let feedback = "ubiratansoares/advisory-db:main is 16 commits behind rustsec/advisory-db:main";
    execution.success().stdout(contains(feedback));
}

#[test]
#[allow(unsafe_code)]
fn should_sync_fork_behind_upstream() {
    let server = MockServer::start();
    let recordings = setup_playback("sync-forks.yaml");
    server.playback(recordings);

    unsafe {
        std::env::set_var("GITHUB_API_URL", server.base_url());
        std::env::set_var("GITHUB_TOKEN", "fake-api-token")
    }

    let execution = sut().arg("sync").assert();

    let feedback = "synchronized ubiratansoares/advisory-db (fast-forward 17 commits)";
    execution.success().stdout(contains(feedback));
}

fn setup_playback(playback: &str) -> PathBuf {
    let current_dir = std::env::current_dir().expect("could not get current directory");
    let recordings = current_dir.join("tests").join("playbacks").join(playback);
    assert!(recordings.exists());
    recordings
}
