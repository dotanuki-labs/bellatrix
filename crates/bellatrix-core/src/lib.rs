// Copyright 2026 Dotanuki Labs
// SPDX-License-Identifier: AGPL-3.0-or-later

pub mod github;

pub use github::*;

use futures::future;
use log::debug;

#[derive(Debug, PartialEq)]
pub struct ForkedRepository {
    pub base: String,
    pub upstream: String,
    pub default_branch: String,
}

#[derive(Debug, PartialEq)]
pub struct BehindUpstream {
    pub repo: ForkedRepository,
    pub commits_behind: u32,
}

#[derive(Debug, PartialEq)]
pub struct SyncOutcome {
    pub synchronized: String,
    pub merge_type: String,
    pub commits: u32,
}

pub struct Bellatrix {
    github_client: Box<dyn GithubApi>,
}

impl Bellatrix {
    pub fn new(github_client: impl GithubApi + 'static) -> Self {
        Bellatrix {
            github_client: Box::new(github_client),
        }
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

        let upstreams = future::join_all(upstream_futures)
            .await
            .into_iter()
            .filter_map(|outcome| match outcome {
                Ok(inner) => Some(inner),
                Err(incoming) => {
                    debug!("cannot determine upstream repo: {:?}", incoming.root_cause());
                    None
                },
            })
            .collect::<Vec<_>>();

        let comparison_futures = upstreams
            .into_iter()
            .map(async |(fork, upstream)| {
                self.github_client
                    .compare_with_upstream(&fork, &upstream)
                    .await
                    .map(|compared| (fork, upstream, compared))
            })
            .collect::<Vec<_>>();

        let comparisons = future::join_all(comparison_futures)
            .await
            .into_iter()
            .filter_map(|outcome| match outcome {
                Ok(inner) => Some(inner),
                Err(incoming) => {
                    debug!("cannot compare fork with upstream: {:?}", incoming.root_cause());
                    None
                },
            })
            .map(|(fork, upstream, comparison)| BehindUpstream {
                repo: ForkedRepository {
                    base: fork.full_name,
                    default_branch: fork.default_branch,
                    upstream: upstream.full_name,
                },
                commits_behind: comparison.ahead_by,
            })
            .filter(|behind| behind.commits_behind > 0)
            .collect::<Vec<_>>();

        Ok(comparisons)
    }

    pub async fn sync_all(&self) -> anyhow::Result<Vec<SyncOutcome>> {
        let forks_behind = self.find_forks_behind_upstream().await?;

        let futures = forks_behind
            .into_iter()
            .map(async |behind| (self.github_client.sync_fork(&behind.repo).await, behind))
            .collect::<Vec<_>>();

        let outcomes = future::join_all(futures)
            .await
            .into_iter()
            .filter_map(|(merging_result, behind_upstream)| match merging_result {
                Ok(merging_result) => {
                    let outcome = SyncOutcome {
                        synchronized: behind_upstream.repo.base,
                        merge_type: merging_result.merge_type,
                        commits: behind_upstream.commits_behind,
                    };
                    Some(outcome)
                },
                Err(incoming) => {
                    debug!("cannot synchronize fork with upstream: {:?}", incoming.root_cause());
                    None
                },
            })
            .collect::<Vec<_>>();

        Ok(outcomes)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        BehindUpstream, Bellatrix, CommitsComparison, ForkedRepository, GithubApi, GithubRepository, RepositoryOwner,
        SyncOutcome, UpstreamMerging, UpstreamRepository,
    };
    use anyhow::Context;
    use assertor::{EqualityAssertion, VecAssertion};
    use async_trait::async_trait;
    use std::collections::HashMap;

    struct FakeGithubClient {
        github_network: HashMap<GithubRepository, Option<(UpstreamRepository, CommitsComparison)>>,
    }

    #[async_trait]
    impl GithubApi for FakeGithubClient {
        async fn list_recently_updated_repos(&self) -> anyhow::Result<Vec<GithubRepository>> {
            Ok(self.github_network.keys().cloned().collect())
        }

        async fn upstream_repo(&self, repo: &GithubRepository) -> anyhow::Result<UpstreamRepository> {
            if &repo.name == "ferris" {
                // simulates http 404
                anyhow::bail!("http 404 : not found")
            }

            let (upstream, _) = self.github_network[repo].clone().context("upstream repo not found")?;
            Ok(upstream)
        }

        async fn compare_with_upstream(
            &self,
            fork: &GithubRepository,
            _: &UpstreamRepository,
        ) -> anyhow::Result<CommitsComparison> {
            if &fork.name == "rustonomics" {
                // simulates networking error
                anyhow::bail!("network unreachable")
            }

            let (_, comparison) = self.github_network[fork].clone().context("upstream repo not found")?;

            Ok(comparison)
        }

        async fn sync_fork(&self, behind_upstream: &ForkedRepository) -> anyhow::Result<UpstreamMerging> {
            if &behind_upstream.upstream == "ferris" {
                anyhow::bail!("http 409 : conflict")
            };

            Ok(UpstreamMerging {
                merge_type: "fast-forward".to_string(),
            })
        }
    }

    fn owned(name: &str) -> GithubRepository {
        GithubRepository {
            full_name: format!("katagi/{}", name),
            name: name.into(),
            default_branch: "main".to_string(),
            owner: RepositoryOwner {
                login: "katagi".to_string(),
            },
            fork: false,
            parent: None,
        }
    }

    fn fork(name: &str) -> GithubRepository {
        GithubRepository {
            full_name: format!("katagi/{}", name),
            name: name.into(),
            default_branch: "main".to_string(),
            owner: RepositoryOwner {
                login: "katagi".to_string(),
            },
            fork: true,
            parent: Some(upstream(name)),
        }
    }

    fn upstream(name: &str) -> UpstreamRepository {
        UpstreamRepository {
            full_name: format!("crabbyverse/{}", name),
            name: name.into(),
            default_branch: "main".to_string(),
            owner: RepositoryOwner {
                login: "crabbyverse".to_string(),
            },
        }
    }

    fn ahead_by(amount: u32) -> CommitsComparison {
        CommitsComparison { ahead_by: amount }
    }

    impl ForkedRepository {
        fn from(name: &str) -> ForkedRepository {
            ForkedRepository {
                base: format!("katagi/{}", name),
                upstream: format!("crabbyverse/{}", name),
                default_branch: "main".to_string(),
            }
        }
    }

    #[tokio::test]
    async fn check_updates_when_available_to_forks() {
        let mut github_network = HashMap::new();
        github_network.insert(fork("shells"), Some((upstream("shells"), ahead_by(0))));
        github_network.insert(fork("claws"), Some((upstream("claws"), ahead_by(3))));
        github_network.insert(owned("ecdysis"), None);

        let github_client = FakeGithubClient { github_network };
        let bellatrix = Bellatrix::new(github_client);

        let behind_upstream = bellatrix
            .find_forks_behind_upstream()
            .await
            .expect("expecting repos from network");

        let expected = vec![BehindUpstream {
            repo: ForkedRepository::from("claws"),
            commits_behind: 3,
        }];

        assertor::assert_that!(behind_upstream).is_equal_to(expected);
    }

    #[tokio::test]
    async fn check_updates_to_non_forks() {
        let mut github_network = HashMap::new();
        github_network.insert(owned("ecdysis"), None);
        github_network.insert(owned("callinectes"), None);

        let github_client = FakeGithubClient { github_network };
        let bellatrix = Bellatrix::new(github_client);

        let behind_upstream = bellatrix
            .find_forks_behind_upstream()
            .await
            .expect("expecting repos from network");

        assertor::assert_that!(behind_upstream).is_empty();
    }

    #[tokio::test]
    async fn check_updates_with_regardless_errors() {
        let mut github_network = HashMap::new();
        github_network.insert(owned("callinectes"), None);

        github_network.insert(fork("shells"), Some((upstream("shells"), ahead_by(3))));

        // ferris will resolve to http 404
        github_network.insert(fork("ferris"), Some((upstream("ferris"), ahead_by(1))));

        // rustonomics will resolve to networking failure
        github_network.insert(fork("rustonomics"), Some((upstream("rustonomics"), ahead_by(42))));

        let github_client = FakeGithubClient { github_network };
        let bellatrix = Bellatrix::new(github_client);

        let behind_upstream = bellatrix
            .find_forks_behind_upstream()
            .await
            .expect("expecting repos from network");

        let expected = vec![BehindUpstream {
            repo: ForkedRepository::from("shells"),
            commits_behind: 3,
        }];

        assertor::assert_that!(behind_upstream).is_equal_to(expected);
    }

    #[tokio::test]
    async fn sync_fork_with_upstreams() {
        let mut github_network = HashMap::new();
        github_network.insert(fork("shells"), Some((upstream("shells"), ahead_by(4))));
        github_network.insert(owned("ecdysis"), None);

        let github_client = FakeGithubClient { github_network };
        let bellatrix = Bellatrix::new(github_client);

        let behind_upstream = bellatrix.sync_all().await.expect("expecting repos from network");

        let expected = vec![SyncOutcome {
            synchronized: "katagi/shells".to_string(),
            merge_type: "fast-forward".to_string(),
            commits: 4,
        }];

        assertor::assert_that!(behind_upstream).is_equal_to(expected);
    }
}
