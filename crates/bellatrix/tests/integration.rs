// Copyright 2026 Dotanuki Labs
// SPDX-License-Identifier: AGPL-3.0-or-later

use assert_cmd::Command;
use predicates::str::contains;

fn sut() -> Command {
    assert_cmd::cargo::cargo_bin_cmd!("bellatrix")
}

#[test]
fn should_parse_arguments() {
    let execution = sut().args(["--name", "John"]).assert();

    execution.stdout(contains("Hello, John"));
}

#[test]
fn should_show_help() {
    let description = "Usage: bellatrix";

    let execution = sut().arg("--help").assert();
    execution.stdout(contains(description));
}

#[test]
fn should_fail_without_arguments() {
    let instruction = "required arguments were not provided";

    let execution = sut().assert();
    execution.failure().stderr(contains(instruction));
}
