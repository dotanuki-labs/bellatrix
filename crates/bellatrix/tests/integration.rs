// Copyright 2026 Dotanuki Labs
// SPDX-License-Identifier: AGPL-3.0-or-later

use assert_cmd::Command;
use predicates::str::contains;

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
fn should_check_available_updates() {
    let feedback = "checking available updates for forks";

    let execution = sut().arg("check").assert();
    execution.success().stdout(contains(feedback));
}
