// Copyright 2026 Dotanuki Labs
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{Context, anyhow};
use async_trait::async_trait;
use header::{HeaderMap, HeaderValue};
use reqwest::header;
use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::RetryTransientMiddleware;
use reqwest_retry::policies::ExponentialBackoff;
use serde::Deserialize;
use std::time::Duration;

pub struct GithubClientConfig {
    github_api_url: String,
    github_token: String,
}

impl GithubClientConfig {
    pub fn new(github_api_url: String, github_token: String) -> Self {
        Self {
            github_api_url,
            github_token,
        }
    }
}

pub struct GithubClient {
    github_api_url: String,
    http_client: ClientWithMiddleware,
}

impl TryFrom<GithubClientConfig> for GithubClient {
    type Error = anyhow::Error;

    fn try_from(config: GithubClientConfig) -> anyhow::Result<Self> {
        let github_api_url = config.github_api_url.trim().to_string();

        if github_api_url.is_empty() {
            anyhow::bail!("github API URL is empty");
        }

        let github_token = config.github_token.trim();

        if github_token.is_empty() {
            anyhow::bail!("github API token is empty");
        }

        let user_agent = format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        let bearer_token = format!("Bearer {}", github_token);

        let user_agent = HeaderValue::from_str(&user_agent)?;
        let user_auth = HeaderValue::from_str(&bearer_token)?;

        let mut headers = HeaderMap::new();
        headers.insert(header::USER_AGENT, user_agent);
        headers.insert(header::AUTHORIZATION, user_auth);

        let base_http_client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(15))
            .build()?;

        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(2);

        let http_client = reqwest_middleware::ClientBuilder::new(base_http_client)
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();
        let client = Self {
            github_api_url,
            http_client,
        };

        Ok(client)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct RepositoryOwner {
    pub login: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct UpstreamRepository {
    pub full_name: String,
    pub name: String,
    pub owner: RepositoryOwner,
    pub default_branch: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct GithubRepository {
    pub full_name: String,
    pub default_branch: String,
    pub fork: bool,
    pub name: String,
    pub owner: RepositoryOwner,
    pub parent: Option<UpstreamRepository>,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct CommitsComparison {
    pub ahead_by: u32,
}

#[async_trait]
pub trait GithubApi {
    async fn list_recently_updated_repos(&self) -> anyhow::Result<Vec<GithubRepository>>;

    async fn upstream_repo(&self, repo: &GithubRepository) -> anyhow::Result<UpstreamRepository>;

    async fn compare_with_upstream(
        &self,
        fork: &GithubRepository,
        upstream: &UpstreamRepository,
    ) -> anyhow::Result<CommitsComparison>;
}

#[async_trait]
impl GithubApi for GithubClient {
    async fn list_recently_updated_repos(&self) -> anyhow::Result<Vec<GithubRepository>> {
        let api = format!("{}/user/repos", self.github_api_url);
        let endpoint = format!("{}?per_page=100&sort=updated&visibility=public&affiliation=owner", &api);
        self.guarded_http_get(&endpoint).await
    }

    async fn upstream_repo(&self, repo: &GithubRepository) -> anyhow::Result<UpstreamRepository> {
        let endpoint = format!("{}/repos/{}", self.github_api_url, repo.full_name);
        let upstream = self.guarded_http_get::<GithubRepository>(&endpoint).await?;
        upstream.parent.ok_or(anyhow!("fork should have a parent repository"))
    }

    async fn compare_with_upstream(
        &self,
        fork: &GithubRepository,
        upstream: &UpstreamRepository,
    ) -> anyhow::Result<CommitsComparison> {
        let endpoint = format!(
            "{}/repos/{}/compare/{}:{}:{}...{}:{}:{}",
            self.github_api_url,
            fork.full_name,
            fork.owner.login,
            fork.name,
            fork.default_branch,
            upstream.owner.login,
            upstream.name,
            upstream.default_branch
        );

        self.guarded_http_get(&endpoint).await
    }
}

impl GithubClient {
    async fn guarded_http_get<T>(&self, endpoint: &str) -> anyhow::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let error_message = format!("failure : GET {} (<reason>)", endpoint);

        let http_response = self
            .http_client
            .get(endpoint)
            .send()
            .await
            .with_context(|| error_message.replace("<reason>", "networking error"))?;

        let status = http_response.status();
        let ok_response = http_response
            .error_for_status()
            .with_context(|| error_message.replace("<reason>", format!("http status = {} ", status).as_str()))?;

        let deserialized = ok_response
            .json::<T>()
            .await
            .with_context(|| error_message.replace("<reason>", "deserialization error"))?;

        Ok(deserialized)
    }
}
