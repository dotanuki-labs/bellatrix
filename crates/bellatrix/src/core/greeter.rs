// Copyright 2026 Dotanuki Labs
// SPDX-License-Identifier: AGPL-3.0-or-later

pub fn greet(name: &str) -> anyhow::Result<String> {
    Ok(format!("Hello, {name}!"))
}
