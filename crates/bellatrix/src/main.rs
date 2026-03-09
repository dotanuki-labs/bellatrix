// Copyright 2026 Dotanuki Labs
// SPDX-License-Identifier: AGPL-3.0-or-later

use bellatrix_core::{Bellatrix, GithubClientConfig};
use clap::{Parser, Subcommand};
use std::env;
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    better_panic::install();
    human_panic::setup_panic!();
    env_logger::init();

    let cmd = Commands::parse().cmd;
    let bellatrix = create_bellatrix()?;
    match cmd {
        Cmd::Check => {
            println!();
            println!("checking available updates for forks");
            println!();
            let analysis = bellatrix.find_forks_behind_upstream().await?;

            if analysis.is_empty() {
                return Ok(());
            }

            for comparison in analysis {
                if comparison.commits_behind > 0 {
                    println!(
                        "{}:{} is {} commits behind {}:{}",
                        comparison.repo.forked,
                        comparison.repo.default_branch,
                        comparison.commits_behind,
                        comparison.repo.upstream,
                        comparison.repo.default_branch
                    );
                } else {
                    println!(
                        "{} is up to date with {}",
                        comparison.repo.forked, comparison.repo.upstream
                    )
                }
            }
        },
        Cmd::Sync => {
            println!("Updating available forks");
            bellatrix.sync_all().await?
        },
    }

    Ok(())
}

fn create_bellatrix() -> anyhow::Result<Bellatrix> {
    let github_api_url = env::var("GITHUB_API_URL").unwrap_or("https://api.github.com".to_string());
    let github_token = env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN env variable is required");
    let github_client_config = GithubClientConfig::new(github_api_url, github_token);
    let github_client = bellatrix_core::github::GithubClient::try_from(github_client_config)?;
    let bellatrix = Bellatrix::new(github_client);
    Ok(bellatrix)
}
