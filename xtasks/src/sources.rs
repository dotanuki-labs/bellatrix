// Copyright 2026 Dotanuki Labs
// SPDX-License-Identifier: AGPL-3.0-or-later

use xshell::{Shell, cmd};

pub fn check_source_files(shell: &Shell) -> anyhow::Result<()> {
    check_code_formatting(shell)?;
    check_code_smells(shell)?;
    Ok(())
}

fn check_code_formatting(shell: &Shell) -> anyhow::Result<()> {
    println!();
    println!("🔥 Checking formatting of Rust code (rustfmt)");
    println!();

    cmd!(shell, "cargo fmt --check").run()?;

    Ok(())
}

fn check_code_smells(shell: &Shell) -> anyhow::Result<()> {
    println!();
    println!("🔥 Checking smells in Rust code (clippy)");
    println!();

    cmd!(shell, "cargo clippy --all-targets --all-features -- -D warnings").run()?;

    Ok(())
}
