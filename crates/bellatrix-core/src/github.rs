// Copyright 2026 Dotanuki Labs
// SPDX-License-Identifier: AGPL-3.0-or-later

use header::{HeaderMap, HeaderValue};
use reqwest::header;
use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::RetryTransientMiddleware;
use reqwest_retry::policies::ExponentialBackoff;
use serde::Deserialize;
use std::time::Duration;

pub struct GithubClient {
    base_url: String,
    http_client: ClientWithMiddleware,
}

#[derive(Debug, Deserialize)]
pub struct RepositoryOwner {
    pub login: String,
}

#[derive(Debug, Deserialize)]
pub struct UpstreamRepository {
    pub full_name: String,
    pub name: String,
    pub owner: RepositoryOwner,
    pub default_branch: String,
}

#[derive(Debug, Deserialize)]
pub struct GithubRepository {
    pub full_name: String,
    pub default_branch: String,
    pub fork: bool,
    pub name: String,
    pub owner: RepositoryOwner,
    pub parent: Option<UpstreamRepository>,
}

#[derive(Debug, Deserialize)]
pub struct ForkComparison {
    pub ahead_by: u32,
    pub behind_by: u32,
}

impl GithubClient {
    pub async fn list_recently_updated_repos(&self) -> anyhow::Result<Vec<GithubRepository>> {
        let endpoint = format!(
            "{}/user/repos?per_page=100&sort=updated&visibility=public&affiliation=owner",
            self.base_url
        );

        dbg!(&endpoint);

        let repos = self
            .http_client
            .get(&endpoint)
            .send()
            .await?
            .error_for_status()?
            .json::<_>()
            .await?;

        Ok(repos)
    }

    pub async fn upstream_repo(&self, repo: &GithubRepository) -> anyhow::Result<UpstreamRepository> {
        let endpoint = format!("{}/repos/{}", self.base_url, repo.full_name);

        dbg!(&endpoint);
        let upstream = self
            .http_client
            .get(&endpoint)
            .send()
            .await?
            .error_for_status()?
            .json::<GithubRepository>()
            .await?;

        Ok(upstream.parent.expect("fork should have a parent repository"))
    }

    pub async fn compare_with_upstream(
        &self,
        fork: &GithubRepository,
        upstream: &UpstreamRepository,
    ) -> anyhow::Result<ForkComparison> {
        let endpoint = format!(
            "{}/repos/{}/compare/{}:{}:{}...{}:{}:{}",
            self.base_url,
            fork.full_name,
            fork.owner.login,
            fork.name,
            fork.default_branch,
            upstream.owner.login,
            upstream.name,
            upstream.default_branch
        );

        dbg!(&endpoint);

        let comparison = self
            .http_client
            .get(&endpoint)
            .send()
            .await?
            .error_for_status()?
            .json::<ForkComparison>()
            .await?;

        Ok(comparison)
    }

    pub async fn sync_fork(&self, _: &GithubRepository, _: &UpstreamRepository) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn new(base_url: String, auth_token: String) -> Self {
        let user_agent = format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        let bearer_token = format!("Bearer {}", auth_token);

        let user_agent = HeaderValue::from_str(&user_agent).expect("invalid header value");
        let user_auth = HeaderValue::from_str(&bearer_token).expect("invalid header value");

        let mut headers = HeaderMap::new();
        headers.insert(header::USER_AGENT, user_agent);
        headers.insert(header::AUTHORIZATION, user_auth);

        let base_http_client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(15))
            .build()
            .expect("cannot build HTTP client");

        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(0);

        let http_client = reqwest_middleware::ClientBuilder::new(base_http_client)
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();
        Self { base_url, http_client }
    }
}
