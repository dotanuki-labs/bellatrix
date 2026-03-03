// Copyright 2026 Dotanuki Labs
// SPDX-License-Identifier: AGPL-3.0-or-later

use clap::{Parser, Subcommand};
use tikv_jemallocator::Jemalloc;

#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[derive(Parser)]
#[command(about, long_about = None)]
struct Commands {
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Checks available updates for existing forks
    Check,
    /// Syncs forks with upstream
    Sync,
}

fn main() -> anyhow::Result<()> {
    better_panic::install();
    human_panic::setup_panic!();

    let cmd = Commands::parse().cmd;

    match cmd {
        Cmd::Check => {
            println!("checking available updates for forks");
        },
        Cmd::Sync => {
            println!("Updating available forks")
        },
    }

    Ok(())
}
