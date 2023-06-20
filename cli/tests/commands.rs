use std::{fs::File, io::Write};

use assert_cmd::Command;
use kythera_actors::wasm_bin::test_actors::{
    BASIC_TARGET_ACTOR_BINARY, BASIC_TEST_ACTOR_BINARY, CHEATCODES_TEST_ACTOR_BINARY,
};
use kythera_cli::commands::gas_snapshot::MethodCost;
use kythera_lib::{to_vec, Abi, Method};
use predicates::str::contains;
use tempfile::{tempdir, TempDir};

const NO_MEMORY_WAT: &str = r#"
        ;; Mock invoke function
            (module
                (func (export "invoke") (param $x i32) (result i32)
                    (i32.const 1)
                )
            )
        "#;

#[test]
fn cli_is_callable() {
    let mut cmd = Command::cargo_bin("kythera").unwrap();
    cmd.arg("--version").assert().success();
}

fn create_target_and_test_actors(
    dir: &TempDir,
    actors_bin: &[Vec<u8>],
    actors_metadata: &[(&str, Abi)],
) {
    let dir_path = dir.path();

    if actors_bin.len() != actors_metadata.len() {
        panic!("Number of bin and metadata should be the same")
    }

    for (i, bin) in actors_bin.iter().enumerate() {
        let (name, abi) = &actors_metadata[i];
        let mut actor_file = File::create(dir_path.join(format!("{name}.wasm"))).unwrap();
        actor_file.write_all(bin).unwrap();
        actor_file.flush().unwrap();

        let mut abi_file = File::create(dir_path.join(format!("{name}.cbor"))).unwrap();
        abi_file.write_all(&to_vec(abi).unwrap()).unwrap();
        abi_file.flush().unwrap();
    }
}

#[test]
fn outputs_single_passed_tests() {
    let dir = tempdir().unwrap();

    create_target_and_test_actors(
        &dir,
        &[
            Vec::from(BASIC_TARGET_ACTOR_BINARY),
            Vec::from(BASIC_TEST_ACTOR_BINARY),
        ],
        &[
            (
                "Target",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: None,
                    methods: vec![
                        Method::new_from_name("HelloWorld").unwrap(),
                        Method::new_from_name("Caller").unwrap(),
                        Method::new_from_name("Origin").unwrap(),
                    ],
                },
            ),
            (
                "Target.t",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: Some(Method::new_from_name("Setup").unwrap()),
                    methods: vec![
                        Method::new_from_name("TestConstructorSetup").unwrap(),
                        Method::new_from_name("TestFailSuccess").unwrap(),
                    ],
                },
            ),
        ],
    );

    let mut cmd = Command::cargo_bin("kythera").unwrap();
    cmd.args(["test", "--path", &dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("\tRunning Tests for Actor : Target.wasm"))
        .stdout(contains("\t\tTesting 1 test files"))
        .stdout(contains("Target.t.wasm: testing 2 tests"))
        .stdout(contains("test TestConstructorSetup ... ok"))
        .stdout(contains("gas consumption"))
        .stdout(contains("test TestFailSuccess ... ok"))
        .stdout(contains("gas consumption"))
        .stdout(contains("test result: ok. 2 passed; 0 failed"));
}

#[test]
fn outputs_single_tests_failed() {
    let dir = tempdir().unwrap();

    create_target_and_test_actors(
        &dir,
        &[
            Vec::from(BASIC_TARGET_ACTOR_BINARY),
            Vec::from(BASIC_TEST_ACTOR_BINARY),
        ],
        &[
            (
                "Target",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: None,
                    methods: vec![
                        Method::new_from_name("HelloWorld").unwrap(),
                        Method::new_from_name("Caller").unwrap(),
                        Method::new_from_name("Origin").unwrap(),
                    ],
                },
            ),
            (
                "Target.t",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: Some(Method::new_from_name("Setup").unwrap()),
                    methods: vec![
                        Method::new_from_name("TestConstructorSetup").unwrap(),
                        Method::new_from_name("TestNonExistent").unwrap(),
                        Method::new_from_name("TestFailed").unwrap(),
                        Method::new_from_name("TestFailFailed").unwrap(),
                    ],
                },
            ),
        ],
    );

    let mut cmd = Command::cargo_bin("kythera").unwrap();
    cmd.args(["test", "--path", &dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("\tRunning Tests for Actor : Target.wasm"))
        .stdout(contains("\t\tTesting 1 test files"))
        .stdout(contains("Target.t.wasm: testing 4 tests"))
        .stdout(contains("test TestConstructorSetup ... ok"))
        .stdout(contains("gas consumption"))
        .stdout(contains("test TestNonExistent ... FAILED"))
        .stdout(contains("gas consumption"))
        .stdout(contains("test TestFailed ... FAILED"))
        .stdout(contains("gas consumption"))
        .stdout(contains("test TestFailFailed ... FAILED"))
        .stdout(contains("gas consumption"))
        .stdout(contains("failures:"))
        .stdout(contains("test TestNonExistent"))
        .stdout(contains("failed: message failed with backtrace:"))
        .stdout(contains(
            "00: f0104 (method 1410782223) -- Unknown method number (22)",
        ))
        .stdout(contains("test TestFailed"))
        .stdout(contains("failed: message failed with backtrace:"))
        .stdout(contains(
            "(method 1857827781) -- panicked at \'assertion failed: `(left == right)`",
        ))
        .stdout(contains("left: `2`,"))
        .stdout(contains(
            "right: `3`\', actors/test_actors/basic_test_actor/src/actor.rs:199:5 (24)",
        ))
        .stdout(contains("test TestFailFailed"))
        .stdout(contains("failed: test exited with exit code 0"))
        .stdout(contains("test result: FAILED. 1 passed; 3 failed"));
}

#[test]
fn outputs_single_error_target_file() {
    let dir = tempdir().unwrap();

    create_target_and_test_actors(
        &dir,
        &[
            wat::parse_str(NO_MEMORY_WAT).unwrap(),
            Vec::from(BASIC_TEST_ACTOR_BINARY),
        ],
        &[
            (
                "Target",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: None,
                    methods: vec![
                        Method::new_from_name("HelloWorld").unwrap(),
                        Method::new_from_name("Caller").unwrap(),
                        Method::new_from_name("Origin").unwrap(),
                    ],
                },
            ),
            (
                "Target.t",
                Abi {
                    constructor: None,
                    set_up: None,
                    methods: vec![],
                },
            ),
        ],
    );

    let mut cmd = Command::cargo_bin("kythera").unwrap();
    cmd.args(["test", "--path", &dir.path().to_str().unwrap()])
        .assert().success()
        .stdout(contains("\tRunning Tests for Actor : Target.wasm"))
        .stdout(contains(
            "Error: Constructor execution failed for actor: Target.wasm",
        ))
        .stdout(contains("Caused by: message failed with backtrace:"))
        .stdout(contains("00: f0103 (method 1) -- fatal error (10)"))
        .stdout(contains("caused by: [FATAL] Error: [from=f1d3nehuc4u3l5mn7hazppnogf3oe6l6ymaicbkhi, to=f0103, seq=0, m=1, h=0]: actor has no memory export"));
}

#[test]
fn outputs_single_error_test_file() {
    let dir = tempdir().unwrap();

    create_target_and_test_actors(
        &dir,
        &[
            Vec::from(BASIC_TARGET_ACTOR_BINARY),
            wat::parse_str(NO_MEMORY_WAT).unwrap(),
        ],
        &[
            (
                "Target",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: None,
                    methods: vec![
                        Method::new_from_name("HelloWorld").unwrap(),
                        Method::new_from_name("Caller").unwrap(),
                        Method::new_from_name("Origin").unwrap(),
                    ],
                },
            ),
            (
                "Target.t",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: Some(Method::new_from_name("Setup").unwrap()),
                    methods: vec![
                        Method::new_from_name("TestConstructorSetup").unwrap(),
                        Method::new_from_name("TestFailSuccess").unwrap(),
                    ],
                },
            ),
        ],
    );

    let mut cmd = Command::cargo_bin("kythera").unwrap();
    cmd.args(["test", "--path", &dir.path().to_str().unwrap()])
        .assert().success()
        .stdout(contains("\tRunning Tests for Actor : Target.wasm"))
        .stdout(contains("\t\tTesting 1 test files"))
        .stdout(contains("Target.t.wasm: testing 2 tests"))
        .stdout(contains("Error: Constructor execution failed for actor: Target.t.wasm"))
        .stdout(contains(
            "Error: Constructor execution failed for actor: Target.t.wasm",
        ))
        .stdout(contains("test result: FAILED. 0 passed; 0 failed"))
        .stdout(contains("Caused by: message failed with backtrace:"))
        .stdout(contains("00: f0104 (method 1) -- fatal error (10)"))
        .stdout(contains("caused by: [FATAL] Error: [from=f1d3nehuc4u3l5mn7hazppnogf3oe6l6ymaicbkhi, to=f0104, seq=1, m=1, h=0]: actor has no memory export"));
}

#[test]
fn outputs_multiple_passed_tests() {
    let dir = tempdir().unwrap();

    create_target_and_test_actors(
        &dir,
        &[
            Vec::from(BASIC_TARGET_ACTOR_BINARY),
            Vec::from(BASIC_TEST_ACTOR_BINARY),
            Vec::from(BASIC_TARGET_ACTOR_BINARY),
            Vec::from(BASIC_TEST_ACTOR_BINARY),
        ],
        &[
            (
                "FirstTarget",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: None,
                    methods: vec![
                        Method::new_from_name("HelloWorld").unwrap(),
                        Method::new_from_name("Caller").unwrap(),
                        Method::new_from_name("Origin").unwrap(),
                    ],
                },
            ),
            (
                "FirstTarget.t",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: Some(Method::new_from_name("Setup").unwrap()),
                    methods: vec![
                        Method::new_from_name("TestConstructorSetup").unwrap(),
                        Method::new_from_name("TestFailSuccess").unwrap(),
                    ],
                },
            ),
            (
                "SecondTarget",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: None,
                    methods: vec![
                        Method::new_from_name("HelloWorld").unwrap(),
                        Method::new_from_name("Caller").unwrap(),
                        Method::new_from_name("Origin").unwrap(),
                    ],
                },
            ),
            (
                "SecondTarget.t",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: Some(Method::new_from_name("Setup").unwrap()),
                    methods: vec![
                        Method::new_from_name("TestConstructorSetup").unwrap(),
                        Method::new_from_name("TestFailSuccess").unwrap(),
                    ],
                },
            ),
        ],
    );

    let mut cmd = Command::cargo_bin("kythera").unwrap();
    cmd.args(["test", "--path", &dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("\tRunning Tests for Actor : FirstTarget.wasm"))
        .stdout(contains("\t\tTesting 1 test files"))
        .stdout(contains("FirstTarget.t.wasm: testing 2 tests"))
        .stdout(contains("test TestConstructorSetup ... ok"))
        .stdout(contains("gas consumption"))
        .stdout(contains("test TestFailSuccess ... ok"))
        .stdout(contains("gas consumption"))
        .stdout(contains("test result: ok. 2 passed; 0 failed"))
        .stdout(contains("\tRunning Tests for Actor : SecondTarget.wasm"))
        .stdout(contains("\t\tTesting 1 test files"))
        .stdout(contains("SecondTarget.t.wasm: testing 2 tests"))
        .stdout(contains("test TestConstructorSetup ... ok"))
        .stdout(contains("gas consumption"))
        .stdout(contains("test TestFailSuccess ... ok"))
        .stdout(contains("gas consumption"))
        .stdout(contains("test result: ok. 2 passed; 0 failed"));
}

#[test]
fn outputs_multiple_failed_tests() {
    let dir = tempdir().unwrap();

    create_target_and_test_actors(
        &dir,
        &[
            Vec::from(BASIC_TARGET_ACTOR_BINARY),
            Vec::from(BASIC_TEST_ACTOR_BINARY),
            Vec::from(BASIC_TARGET_ACTOR_BINARY),
            Vec::from(BASIC_TEST_ACTOR_BINARY),
        ],
        &[
            (
                "FirstTarget",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: None,
                    methods: vec![
                        Method::new_from_name("HelloWorld").unwrap(),
                        Method::new_from_name("Caller").unwrap(),
                        Method::new_from_name("Origin").unwrap(),
                    ],
                },
            ),
            (
                "FirstTarget.t",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: Some(Method::new_from_name("Setup").unwrap()),
                    methods: vec![
                        Method::new_from_name("TestConstructorSetup").unwrap(),
                        Method::new_from_name("TestNonExistent").unwrap(),
                        Method::new_from_name("TestFailed").unwrap(),
                        Method::new_from_name("TestFailFailed").unwrap(),
                    ],
                },
            ),
            (
                "SecondTarget",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: None,
                    methods: vec![
                        Method::new_from_name("HelloWorld").unwrap(),
                        Method::new_from_name("Caller").unwrap(),
                        Method::new_from_name("Origin").unwrap(),
                    ],
                },
            ),
            (
                "SecondTarget.t",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: Some(Method::new_from_name("Setup").unwrap()),
                    methods: vec![
                        Method::new_from_name("TestConstructorSetup").unwrap(),
                        Method::new_from_name("TestNonExistent").unwrap(),
                        Method::new_from_name("TestFailed").unwrap(),
                        Method::new_from_name("TestFailFailed").unwrap(),
                    ],
                },
            ),
        ],
    );

    let mut cmd = Command::cargo_bin("kythera").unwrap();
    cmd.args(["test", "--path", &dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("\tRunning Tests for Actor : FirstTarget.wasm"))
        .stdout(contains("\t\tTesting 1 test files"))
        .stdout(contains("FirstTarget.t.wasm: testing 4 tests"))
        .stdout(contains("test TestConstructorSetup ... ok"))
        .stdout(contains("gas consumption"))
        .stdout(contains("test TestNonExistent ... FAILED"))
        .stdout(contains("gas consumption"))
        .stdout(contains("test TestFailed ... FAILED"))
        .stdout(contains("gas consumption"))
        .stdout(contains("test TestFailFailed ... FAILED"))
        .stdout(contains("gas consumption"))
        .stdout(contains("failures:"))
        .stdout(contains("test TestNonExistent"))
        .stdout(contains("failed: message failed with backtrace:"))
        .stdout(contains(
            "00: f0104 (method 1410782223) -- Unknown method number (22)",
        ))
        .stdout(contains("test TestFailed"))
        .stdout(contains("failed: message failed with backtrace:"))
        .stdout(contains(
            "(method 1857827781) -- panicked at \'assertion failed: `(left == right)`",
        ))
        .stdout(contains("left: `2`,"))
        .stdout(contains(
            "right: `3`\', actors/test_actors/basic_test_actor/src/actor.rs:199:5 (24)",
        ))
        .stdout(contains("test TestFailFailed"))
        .stdout(contains("failed: test exited with exit code 0"))
        .stdout(contains("test result: FAILED. 1 passed; 3 failed"))
        .stdout(contains("\tRunning Tests for Actor : SecondTarget.wasm"))
        .stdout(contains("\t\tTesting 1 test files"))
        .stdout(contains("SecondTarget.t.wasm: testing 4 tests"))
        .stdout(contains("test TestConstructorSetup ... ok"))
        .stdout(contains("gas consumption"))
        .stdout(contains("test TestNonExistent ... FAILED"))
        .stdout(contains("gas consumption"))
        .stdout(contains("test TestFailed ... FAILED"))
        .stdout(contains("gas consumption"))
        .stdout(contains("test TestFailFailed ... FAILED"))
        .stdout(contains("gas consumption"))
        .stdout(contains("failures:"))
        .stdout(contains("test TestNonExistent"))
        .stdout(contains("failed: message failed with backtrace:"))
        .stdout(contains(
            "00: f0104 (method 1410782223) -- Unknown method number (22)",
        ))
        .stdout(contains("test TestFailed"))
        .stdout(contains("failed: message failed with backtrace:"))
        .stdout(contains(
            "(method 1857827781) -- panicked at \'assertion failed: `(left == right)`",
        ))
        .stdout(contains("left: `2`,"))
        .stdout(contains(
            "right: `3`\', actors/test_actors/basic_test_actor/src/actor.rs:199:5 (24)",
        ))
        .stdout(contains("test TestFailFailed"))
        .stdout(contains("failed: test exited with exit code 0"))
        .stdout(contains("test result: FAILED. 1 passed; 3 failed"));
}

#[test]
fn outputs_gas_report() {
    let dir = tempdir().unwrap();

    create_target_and_test_actors(
        &dir,
        &[
            Vec::from(BASIC_TARGET_ACTOR_BINARY),
            Vec::from(BASIC_TEST_ACTOR_BINARY),
        ],
        &[
            (
                "Target",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: None,
                    methods: vec![
                        Method::new_from_name("HelloWorld").unwrap(),
                        Method::new_from_name("Caller").unwrap(),
                        Method::new_from_name("Origin").unwrap(),
                    ],
                },
            ),
            (
                "Target.t",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: Some(Method::new_from_name("Setup").unwrap()),
                    methods: vec![Method::new_from_name("TestMethodParameter").unwrap()],
                },
            ),
        ],
    );

    let mut cmd = Command::cargo_bin("kythera").unwrap();
    cmd.args([
        "test",
        "--path",
        &dir.path().to_str().unwrap(),
        "--gas-report",
    ])
    .assert()
    .success()
    .stdout(contains("Gas report"))
    .stdout(contains(
        "╭──────────────────────┬───────────┬───────────┬───────────┬───────────┬─────────╮",
    ))
    .stdout(contains(
        "│ Target.wasm contract ┆           ┆           ┆           ┆           ┆         │",
    ))
    .stdout(contains(
        "╞══════════════════════╪═══════════╪═══════════╪═══════════╪═══════════╪═════════╡",
    ))
    .stdout(contains(
        "│ Function Name        ┆ min       ┆ max       ┆ avg       ┆ median    ┆ # calls │",
    ))
    .stdout(contains(
        "├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌┤",
    ))
    .stdout(contains("│ Constructor"))
    .stdout(contains("│ HelloWorld"))
    .stdout(contains(
        "╰──────────────────────┴───────────┴───────────┴───────────┴───────────┴─────────╯",
    ));
}

#[test]
fn creates_gas_snapshot() {
    let dir = tempdir().unwrap();
    create_target_and_test_actors(
        &dir,
        &[
            Vec::from(BASIC_TARGET_ACTOR_BINARY),
            Vec::from(BASIC_TEST_ACTOR_BINARY),
        ],
        &[
            (
                "Target",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: None,
                    methods: vec![
                        Method::new_from_name("HelloWorld").unwrap(),
                        Method::new_from_name("Caller").unwrap(),
                        Method::new_from_name("Origin").unwrap(),
                    ],
                },
            ),
            (
                "Target.t",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: Some(Method::new_from_name("Setup").unwrap()),
                    methods: vec![Method::new_from_name("TestMethodParameter").unwrap()],
                },
            ),
        ],
    );

    let mut cmd = Command::cargo_bin("kythera").unwrap();
    let path = dir.path().join(".gas-snapshot");
    cmd.args([
        "snapshot",
        "--path",
        &dir.path().to_str().unwrap(),
        "--snap",
        path.to_str().unwrap(),
    ])
    .assert()
    .success();

    let file = File::open(path).unwrap();
    let mut rdr = csv::Reader::from_reader(file);
    let snapshot = rdr
        .deserialize::<MethodCost>()
        .into_iter()
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>();
    assert_eq!(snapshot.len(), 1);
    assert_eq!("Target.wasm::TestMethodParameter", snapshot[0].name);
    assert!(snapshot[0].passed)
}

#[test]
fn snapshot_check_success() {
    let dir = tempdir().unwrap();
    create_target_and_test_actors(
        &dir,
        &[
            Vec::from(BASIC_TARGET_ACTOR_BINARY),
            Vec::from(BASIC_TEST_ACTOR_BINARY),
        ],
        &[
            (
                "Target",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: None,
                    methods: vec![
                        Method::new_from_name("HelloWorld").unwrap(),
                        Method::new_from_name("Caller").unwrap(),
                        Method::new_from_name("Origin").unwrap(),
                    ],
                },
            ),
            (
                "Target.t",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: Some(Method::new_from_name("Setup").unwrap()),
                    methods: vec![Method::new_from_name("TestMethodParameter").unwrap()],
                },
            ),
        ],
    );

    let mut cmd = Command::cargo_bin("kythera").unwrap();
    let path = dir.path().join(".gas-snapshot");

    cmd.args([
        "snapshot",
        "--path",
        &dir.path().to_str().unwrap(),
        "--snap",
        path.to_str().unwrap(),
    ])
    .assert()
    .success();

    cmd = Command::cargo_bin("kythera").unwrap();
    cmd.args([
        "snapshot",
        "--path",
        &dir.path().to_str().unwrap(),
        "--check",
        path.to_str().unwrap(),
    ])
    .assert()
    .success();
}

#[test]
fn snapshot_check_fails_differences() {
    let dir = tempdir().unwrap();
    create_target_and_test_actors(
        &dir,
        &[
            Vec::from(BASIC_TARGET_ACTOR_BINARY),
            Vec::from(BASIC_TEST_ACTOR_BINARY),
        ],
        &[
            (
                "Target",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: None,
                    methods: vec![
                        Method::new_from_name("HelloWorld").unwrap(),
                        Method::new_from_name("Caller").unwrap(),
                        Method::new_from_name("Origin").unwrap(),
                    ],
                },
            ),
            (
                "Target.t",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: Some(Method::new_from_name("Setup").unwrap()),
                    methods: vec![Method::new_from_name("TestMethodParameter").unwrap()],
                },
            ),
        ],
    );

    let mut cmd = Command::cargo_bin("kythera").unwrap();
    let path2 = dir.path().join(".gas-snapshot2");
    let _file = File::create(&path2).unwrap();
    cmd.args([
        "snapshot",
        "--path", 
        &dir.path().to_str().unwrap(),
        "--check",
        path2.to_str().unwrap(),
    ])
    .assert()
    .failure()
    .stdout(contains(
        "No matching snapshot entry found for \"Target.wasm::TestMethodParameter\" in snapshot file",
    ));
}

#[test]
fn snapshpt_prints_diff_same_gas_usage() {
    let dir = tempdir().unwrap();
    create_target_and_test_actors(
        &dir,
        &[
            Vec::from(BASIC_TARGET_ACTOR_BINARY),
            Vec::from(BASIC_TEST_ACTOR_BINARY),
        ],
        &[
            (
                "Target",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: None,
                    methods: vec![
                        Method::new_from_name("HelloWorld").unwrap(),
                        Method::new_from_name("Caller").unwrap(),
                        Method::new_from_name("Origin").unwrap(),
                    ],
                },
            ),
            (
                "Target.t",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: Some(Method::new_from_name("Setup").unwrap()),
                    methods: vec![Method::new_from_name("TestMethodParameter").unwrap()],
                },
            ),
        ],
    );

    let mut cmd = Command::cargo_bin("kythera").unwrap();
    let path = dir.path().join(".gas-snapshot");
    cmd.args([
        "snapshot",
        "--path",
        &dir.path().to_str().unwrap(),
        "--snap",
        path.to_str().unwrap(),
    ])
    .assert()
    .success();

    cmd = Command::cargo_bin("kythera").unwrap();
    cmd.args([
        "snapshot",
        "--path",
        &dir.path().to_str().unwrap(),
        "--diff",
        path.to_str().unwrap(),
    ])
    .assert()
    .success()
    .stdout(contains(
        "Target.wasm::TestMethodParameter: gas used is the same",
    ));
}

#[test]
fn snapshot_prints_diff_more_gas_usage() {
    let dir = tempdir().unwrap();
    create_target_and_test_actors(
        &dir,
        &[
            Vec::from(BASIC_TARGET_ACTOR_BINARY),
            Vec::from(BASIC_TEST_ACTOR_BINARY),
        ],
        &[
            (
                "Target",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: None,
                    methods: vec![
                        Method::new_from_name("HelloWorld").unwrap(),
                        Method::new_from_name("Caller").unwrap(),
                        Method::new_from_name("Origin").unwrap(),
                    ],
                },
            ),
            (
                "Target.t",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: Some(Method::new_from_name("Setup").unwrap()),
                    methods: vec![Method::new_from_name("TestMethodParameter").unwrap()],
                },
            ),
        ],
    );

    let mut cmd = Command::cargo_bin("kythera").unwrap();
    let path = dir.path().join(".gas-snapshot");
    let path2 = dir.path().join(".gas-snapshot2");
    cmd.args([
        "snapshot",
        "--path",
        &dir.path().to_str().unwrap(),
        "--snap",
        path.to_str().unwrap(),
    ])
    .assert()
    .success();

    let file = File::open(path).unwrap();
    let mut rdr = csv::Reader::from_reader(file);
    let mut snapshot = rdr
        .deserialize::<MethodCost>()
        .into_iter()
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>();

    let more = snapshot[0].cost as f64 * 1.2;
    let total = (more - snapshot[0].cost as f64).trunc();
    snapshot[0].cost = more as u64;

    let file = File::create(&path2).unwrap();
    let mut wtr = csv::Writer::from_writer(file);
    for method in snapshot {
        wtr.serialize(method).unwrap();
    }
    wtr.flush().unwrap();

    cmd = Command::cargo_bin("kythera").unwrap();
    cmd.args([
        "snapshot",
        "--path",
        &dir.path().to_str().unwrap(),
        "--diff",
        path2.to_str().unwrap(),
    ])
    .assert()
    .success()
    .stdout(contains(
        "Target.wasm::TestMethodParameter: gas used is 20% more",
    ))
    .stdout(contains(format!("Total gas dif: {total}")));
}

#[test]
fn snapshot_prints_diff_less_gas_usage() {
    let dir = tempdir().unwrap();
    create_target_and_test_actors(
        &dir,
        &[
            Vec::from(BASIC_TARGET_ACTOR_BINARY),
            Vec::from(BASIC_TEST_ACTOR_BINARY),
        ],
        &[
            (
                "Target",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: None,
                    methods: vec![
                        Method::new_from_name("HelloWorld").unwrap(),
                        Method::new_from_name("Caller").unwrap(),
                        Method::new_from_name("Origin").unwrap(),
                    ],
                },
            ),
            (
                "Target.t",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: Some(Method::new_from_name("Setup").unwrap()),
                    methods: vec![Method::new_from_name("TestMethodParameter").unwrap()],
                },
            ),
        ],
    );

    let mut cmd = Command::cargo_bin("kythera").unwrap();
    let path = dir.path().join(".gas-snapshot");
    let path2 = dir.path().join(".gas-snapshot2");
    cmd.args([
        "snapshot",
        "--path",
        &dir.path().to_str().unwrap(),
        "--snap",
        path.to_str().unwrap(),
    ])
    .assert()
    .success();

    let file = File::open(path).unwrap();
    let mut rdr = csv::Reader::from_reader(file);
    let mut snapshot = rdr
        .deserialize::<MethodCost>()
        .into_iter()
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>();

    let less = snapshot[0].cost as f64 * 0.8;

    let total = (less - snapshot[0].cost as f64).floor();

    snapshot[0].cost = less as u64;

    let file = File::create(&path2).unwrap();
    let mut wtr = csv::Writer::from_writer(file);
    for method in snapshot {
        wtr.serialize(method).unwrap();
    }
    wtr.flush().unwrap();

    cmd = Command::cargo_bin("kythera").unwrap();
    cmd.args([
        "snapshot",
        "--path",
        &dir.path().to_str().unwrap(),
        "--diff",
        path2.to_str().unwrap(),
    ])
    .assert()
    .success()
    .stdout(contains(
        "Target.wasm::TestMethodParameter: gas used is 20% less",
    ))
    .stdout(contains(format!("Total gas dif: {total}")));
}

#[test]
fn outputs_log_cheatcode() {
    let dir = tempdir().unwrap();
    create_target_and_test_actors(
        &dir,
        &[
            Vec::from(BASIC_TARGET_ACTOR_BINARY),
            Vec::from(CHEATCODES_TEST_ACTOR_BINARY),
        ],
        &[
            (
                "Target",
                Abi {
                    constructor: Some(Method::new_from_name("Constructor").unwrap()),
                    set_up: None,
                    methods: vec![
                        Method::new_from_name("HelloWorld").unwrap(),
                        Method::new_from_name("Caller").unwrap(),
                        Method::new_from_name("Origin").unwrap(),
                    ],
                },
            ),
            (
                "Target.t",
                Abi {
                    constructor: None,
                    set_up: None,
                    methods: vec![Method::new_from_name("TestLog").unwrap()],
                },
            ),
        ],
    );

    let mut cmd = Command::cargo_bin("kythera").unwrap();
    cmd.args(["test", "--path", &dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("hello from actor"));
}
