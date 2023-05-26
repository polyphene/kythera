// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT
use std::error::Error;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;

const ACTORS: &[&str] = &["cheatcodes-actor"];

#[cfg(feature = "testing")]
const TEST_ACTORS: &[&str] = &[
    "basic-test-actor",
    "basic-target-actor",
    "builtins-test-actor",
    "cheatcodes-test-actor",
    "fail-test-actor",
];

const FILES_TO_WATCH: &[&str] = &["Cargo.toml", "src", "actors"];

fn main() -> Result<(), Box<dyn Error>> {
    // Cargo executable location.
    let cargo = std::env::var_os("CARGO").expect("no CARGO env var");

    let out_dir = std::env::var_os("OUT_DIR")
        .as_ref()
        .map(Path::new)
        .map(|p| p.join("bundle"))
        .expect("no OUT_DIR env var");
    println!("cargo:warning=out_dir: {:?}", &out_dir);

    let manifest_path =
        Path::new(&std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR unset"))
            .join("Cargo.toml");

    let mut files_to_watch = FILES_TO_WATCH.to_vec();

    if cfg!(feature = "testing") {
        files_to_watch = [files_to_watch, vec!["test_actors"]].concat();
    }

    for file in files_to_watch {
        println!("cargo:rerun-if-changed={}", file);
    }

    #[allow(unused_assignments, unused_mut)]
    let mut actors = ACTORS.to_vec();
    #[cfg(feature = "testing")]
    {
        actors = [ACTORS, TEST_ACTORS].concat();
    }

    // Cargo build command for all test_actors at once.
    let mut cmd = Command::new(cargo);
    cmd.arg("build")
        .args(actors.iter().map(|pkg| "-p=".to_owned() + pkg))
        .arg("--target=wasm32-unknown-unknown")
        .arg("--profile=wasm")
        .arg("--manifest-path=".to_owned() + manifest_path.to_str().unwrap())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // We are supposed to only generate artifacts under OUT_DIR,
        // so set OUT_DIR as the target directory for this build.
        .env("CARGO_TARGET_DIR", &out_dir)
        // As we are being called inside a build-script, this env variable is set. However, we set
        // our own `RUSTFLAGS` and thus, we need to remove this. Otherwise cargo favors this
        // env variable.
        .env_remove("CARGO_ENCODED_RUSTFLAGS");

    // Print out the command line we're about to run.
    println!("cargo:warning=cmd={:?}", &cmd);

    // Launch the command.
    let mut child = cmd.spawn().expect("failed to launch cargo build");

    // Pipe the output as cargo warnings. Unfortunately this is the only way to
    // get cargo build to print the output.
    let stdout = child.stdout.take().expect("no stdout");
    let stderr = child.stderr.take().expect("no stderr");
    let j1 = thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            println!("cargo:warning={:?}", line.unwrap());
        }
    });
    let j2 = thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            println!("cargo:warning={:?}", line.unwrap());
        }
    });

    j1.join().unwrap();
    j2.join().unwrap();

    let result = child.wait().expect("failed to wait for build to finish");
    if !result.success() {
        return Err("actor build failed".into());
    }

    Ok(())
}
