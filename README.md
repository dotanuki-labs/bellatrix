# bellatrix

## What

`bellatrix` is a small application to keep all your forks up-to-date with upstreams.

## Using

It packages both as a CLI and a Cloudflare worker.

For the CLI

```bash
Usage: bellatrix <COMMAND>

Commands:
  check  Checks available updates for existing forks
  sync   Syncs forks with upstream
  help   Print this message or the help of the given subcommand(s)
```

`bellatrix` expects either a GitHub personal access token with sufficient privileges to run:

- the CLI expects a `GITHUB_TOKEN` environment variable
- the Cloudflare worker expects a `GITHUB_TOKEN` secret bound to the worker runtime

## Installing the CLI

We don't provision any binaries and we don't ship any crates to crates.io,
so you may install the CLI directly from GitHub

```bash
cargo install --git https://github.com/dotanuki-labs/canopus
```

## Deploying the Cloudflare Worker

Please check the requirements and
[set up your Cloudflare Worker project first](https://developers.cloudflare.com/workers/get-started/guide/).

Afterwards, set up your tooling:

```bash
# Required for packaging Cloudflare workers
rustup target add wasm32-unknown-unknown

# Ensure https://crates.io/crates/worker-build version in sync with current runtime
worker_version=$(grep "worker =" Cargo.toml | tr -d '"' | tr -d '=' | cut -d " " -f 3)
cargo install --locked worker-build@"$worker_version"
```

Last, to deploy your worker ([wrangler](https://developers.cloudflare.com/workers/wrangler/) set up)

```bash

worker-build --release crates/bellatrix-worker
wrangler deploy -c crates/bellatrix-worker/wrangler.toml
```

## License

This code is dual-licensed and actually might not entire match the expectations of existing
[definitions of open-source](https://opensource.org/osd).
This is not expected to change.

- Copyright ©2026 - Dotanuki Labs - [AGPLv3](https://choosealicense.com/licenses/agpl-3.0)
- Copyright ©2026 - Dotanuki Labs - [HLv3](https://firstdonoharm.dev/learn)
