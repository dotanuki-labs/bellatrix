// Copyright 2026 Dotanuki Labs
// SPDX-License-Identifier: AGPL-3.0-or-later

use bellatrix_core::{Bellatrix, GithubClientConfig};
use worker::*;

#[event(scheduled)]
async fn scheduled(_: ScheduledEvent, env: Env, _: ScheduleContext) {
    console_log!("scheduled task started");
    let bellatrix = create_bellatrix(env).await.expect("failed to create bellatrix");

    console_log!("updating available forks");
    match bellatrix.sync_all().await {
        Ok(sync_outcomes) => {
            if sync_outcomes.is_empty() {
                console_log!("✓ no sync required");
                return;
            }

            for fork in sync_outcomes {
                console_log!(
                    "synchronized {} ({} {} commits)",
                    &fork.synchronized,
                    fork.merge_type,
                    fork.commits
                )
            }
        },
        Err(incoming) => {
            console_error!("{}", incoming);
        },
    };
}

async fn create_bellatrix(env: Env) -> anyhow::Result<Bellatrix> {
    let github_api_url = env
        .var("GITHUB_API_URL")
        .map_or("https://api.github.com".to_string(), |url| url.to_string());

    let github_token = env
        .secret_store("GITHUB_TOKEN")?
        .get()
        .await?
        .expect("invalid GITHUB_TOKEN");

    let github_client_config = GithubClientConfig::new(github_api_url, github_token);
    let github_client = bellatrix_core::github::GithubClient::try_from(github_client_config)?;
    let bellatrix = Bellatrix::new(github_client);
    Ok(bellatrix)
}
