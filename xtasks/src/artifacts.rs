// Copyright 2026 Dotanuki Labs
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::ArtifactType;
use crate::utils::BuildEnvironment::{CI, Local};
use crate::utils::{docker_execution_arguments, evaluate_build_environment};
use anyhow::bail;
use sha2::{Digest, Sha256};
use std::{env, fs};
use walkdir::WalkDir;
use xshell::{Shell, cmd};

static DEFAULT_ARTIFACTS_DIR: &str = "artifacts";

pub fn assemble_artifacts(shell: &Shell, artifact_type: &ArtifactType) -> anyhow::Result<()> {
    match artifact_type {
        ArtifactType::Binaries => {
            shell.remove_path(DEFAULT_ARTIFACTS_DIR)?;
            shell.create_dir(DEFAULT_ARTIFACTS_DIR)?;
            build_targets(shell)?;
        },
        ArtifactType::Extras => extract_metadata(shell)?,
    }

    Ok(())
}

pub fn extract_metadata(shell: &Shell) -> anyhow::Result<()> {
    compute_sbom(shell)?;
    compute_checksums(shell)?;
    Ok(())
}

fn build_targets(shell: &Shell) -> anyhow::Result<()> {
    println!();
    println!("🔥 Building binaries");
    println!();

    match evaluate_build_environment() {
        CI => {
            println!("• Building on CI environment");
            let targets = evaluate_build_targets()?;

            for target in targets {
                cmd!(shell, "rustup target add {target}").run()?;
                cmd!(shell, "cargo build -p bellatrix --release --target {target}").run()?;
                let binary = format!("target/{target}/release/bellatrix");
                let destination = format!("{DEFAULT_ARTIFACTS_DIR}/bellatrix-{target}");
                shell.copy_file(&binary, &destination)?;
                cmd!(shell, "chmod +x {destination}").run()?;
            }
        },
        Local => {
            println!("• Building on local environment");
            cmd!(shell, "cargo build -p bellatrix --release").run()?;
        },
    };
    Ok(())
}

fn compute_sbom(shell: &Shell) -> anyhow::Result<()> {
    println!();
    println!("🔥 Extracting CycloneDX SBOM from project dependencies");
    println!();

    match evaluate_build_environment() {
        CI => {
            let (volume, image) = docker_execution_arguments();
            cmd!(shell, "docker run --rm -v {volume} {image} cyclonedx").run()?;
        },
        Local => {
            cmd!(shell, "cargo cyclonedx --format json").run()?;
        },
    };

    Ok(())
}

fn compute_checksums(shell: &Shell) -> anyhow::Result<()> {
    println!();
    println!("🔥 Computing checksums for binaries");
    println!();

    let checksums = WalkDir::new(DEFAULT_ARTIFACTS_DIR)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| {
            let path = entry.path().to_str().expect("cannot stringify path");
            path.contains("bellatrix-x86_64") || path.contains("bellatrix-aarch64")
        })
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| {
            let name = entry.file_name();
            let contents = fs::read(entry.path()).expect("cannot read target binary file as bytes");
            let digest = Sha256::digest(contents);
            format!("{} : {}", name.to_string_lossy(), hex::encode(digest))
        })
        .collect::<Vec<String>>()
        .join("\n");

    let checksums_file = format!("{DEFAULT_ARTIFACTS_DIR}/checksums.txt");
    shell.write_file(checksums_file, checksums)?;
    Ok(())
}

fn evaluate_build_targets() -> anyhow::Result<Vec<String>> {
    let runner_name = env::var("RUNNER_OS")?;
    let runner = match runner_name.as_str() {
        "Linux" | "macOS" => runner_name,
        _ => bail!("Unsupported runner : {}", runner_name),
    };

    let arch_name = env::var("RUNNER_ARCH")?;
    let arch = match arch_name.to_ascii_lowercase().as_str() {
        "x64" => "x86_64",
        "arm64" => "aarch64",
        _ => bail!("Unsupported architecture : {}", arch_name),
    };

    let targets = match (runner.to_ascii_lowercase().as_str(), arch) {
        ("linux", "x86_64") => vec!["x86_64-unknown-linux-musl"],
        ("linux", "aarch64") => vec!["aarch64-unknown-linux-musl"],
        ("macos", _) => vec!["x86_64-apple-darwin", "aarch64-apple-darwin"],
        (_, _) => bail!("Unsupported combination : {}-{}", arch, runner),
    };

    let targets = targets.into_iter().map(|t| t.to_string()).collect::<Vec<_>>();

    Ok(targets)
}
