// Copyright 2026 Dotanuki Labs
// SPDX-License-Identifier: AGPL-3.0-or-later

pub mod github;

pub use github::*;

use futures::future::join_all;

#[derive(Debug)]
pub struct ForkedRepository {
    pub forked: String,
    pub upstream: String,
    pub default_branch: String,
}

#[derive(Debug)]
pub struct BehindUpstream {
    pub repo: ForkedRepository,
    pub commits_behind: u32,
}

pub struct Bellatrix {
    github_client: GithubClient,
}

impl Bellatrix {
    pub fn new(github_client: GithubClient) -> Self {
        Self { github_client }
    }

    pub async fn find_forks_behind_upstream(&self) -> anyhow::Result<Vec<BehindUpstream>> {
        let repos = self.github_client.list_recently_updated_repos().await?;
        let forks = repos.into_iter().filter(|repo| repo.fork).collect::<Vec<_>>();

        let upstream_futures = forks
            .into_iter()
            .map(async |fork| {
                self.github_client
                    .upstream_repo(&fork)
                    .await
                    .map(|upstream| (fork, upstream))
            })
            .collect::<Vec<_>>();

        let upstreams: Vec<(GithubRepository, UpstreamRepository)> = join_all(upstream_futures)
            .await
            .into_iter()
            .filter_map(|outcome| outcome.ok())
            .collect::<Vec<_>>();

        let comparison_futures = upstreams
            .into_iter()
            .map(async |(fork, upstream)| {
                self.github_client
                    .compare_with_upstream(&fork, &upstream)
                    .await
                    .map(|compared_upstream| (fork, upstream, compared_upstream))
            })
            .collect::<Vec<_>>();

        let comparisons = join_all(comparison_futures)
            .await
            .into_iter()
            .filter_map(|outcome| outcome.ok())
            .filter(|(_, _, comparison)| comparison.ahead_by > 0)
            .map(|(fork, upstream, comparison)| BehindUpstream {
                repo: ForkedRepository {
                    forked: fork.full_name,
                    default_branch: fork.default_branch,
                    upstream: upstream.full_name,
                },
                commits_behind: comparison.ahead_by,
            })
            .collect::<Vec<_>>();

        Ok(comparisons)
    }

    pub async fn sync_all(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
