// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use std::path::PathBuf;
use std::{
    ffi::OsStr,
    fs::{self, File, Metadata},
    io::Read,
    path::Path,
};

use anyhow::Context;
use kythera_lib::{pascal_case_split, Abi, WasmActor};
use walkdir::WalkDir;

/// A test structure composed by the target Actor and its multiple tests.
pub struct Test {
    pub actor: WasmActor,
    pub tests: Vec<WasmActor>,
}

/// Read the name of a file and its contents as a binary vector
fn read_file_data<P: AsRef<Path>>(path: P) -> anyhow::Result<(String, Vec<u8>)> {
    let path = path.as_ref();
    let mut file =
        File::open(path).with_context(|| format!("Could not open file {}", path.display()))?;

    // If we know the size of the file allocate preemptively its capacity.
    let file_size = file.metadata().as_ref().map(Metadata::len).unwrap_or(0);
    let mut content = Vec::with_capacity(file_size as usize);

    file.read_to_end(&mut content)
        .with_context(|| format!("Could not read file {}", path.display()))?;

    let file_name = path
        .file_name()
        .expect("Actor file name should be valid")
        .to_string_lossy()
        .into_owned();

    Ok((file_name, content))
}

/// Set the extension of a given path to .cbor
fn set_abi_extension<P: AsRef<Path>>(path: P) -> anyhow::Result<String> {
    let mut path_buf = PathBuf::from(path.as_ref());
    path_buf.set_extension("cbor");
    Ok(path_buf
        .into_os_string()
        .into_string()
        .map_err(|_| crate::error::Error::FailedConversion)?)
}

/// Create a WebAssembly actor from a binary and an Abi.
fn create_actor<P: AsRef<Path>>(binary_path: P) -> anyhow::Result<WasmActor> {
    let abi_path = derive_abi_path(&binary_path)?;

    let (file_name, bytecode) = get_file_data(binary_path)?;
    let abi: Abi = kythera_lib::from_slice(&get_file_data(abi_path)?.1)?;

    Ok(WasmActor::new(file_name, bytecode, abi))
}

/// Gather the target Actor file and its test files.
/// The rules for reading Actor files and it's matching tests are:
/// - All .wasm files that are at the root of the kythera input dir are actors.
/// - All .t.wasm files that are at the root of the kythera wasm dir are test actors.
/// - All .wasm files that are in .t dirs are test actors.
pub fn search_files<P: AsRef<Path>>(path: P) -> anyhow::Result<Vec<Test>> {
    // Search the root dir and find all the .wasm files there which may be target actors
    // or its matching test dirs and files.
    // Split into two lists, the first being the target Actors and the second
    // their matching test files and dirs.
    let (target_actor_paths, mut test_artifacts_paths): (Vec<String>, Vec<String>) =
        fs::read_dir(path)
            .context("Could not read the input path")?
            .into_iter()
            .filter_map(Result::ok)
            // Path::ends_with is different from String::ends_with,
            // Path::ends_with operates on the child, in this case
            // we don't know the name of the file so we can't operate on the child.
            .filter_map(|e| e.path().into_os_string().into_string().ok())
            .filter(|path| path.ends_with(".wasm") || path.ends_with(".t"))
            // Warn if not in Pascal case.
            .inspect(|path| {
                let filename = Path::new(path)
                    .file_name()
                    .and_then(OsStr::to_str)
                    .filter(|f| !pascal_case_split(f).is_empty());

                if filename.is_none() {
                    log::warn!("file {path} is not in PascalCase");
                }
            })
            .partition(|path| path.ends_with(".wasm") && !path.ends_with(".t.wasm"));

    let mut tests = vec![];
    for target_actor_path in target_actor_paths {
        // Get target actor.
        let Ok(target_actor) = create_actor(&target_actor_path) else {
            log::error!("Could not get target Actor for binary {target_actor_path}");
            continue;
        };

        let mut actor_tests = vec![];

        // iterate the remaining root entries looking for test files and dirs
        // if the entry is a matching test we remove it from the root list.
        // and add it to our target Actor tests.
        test_artifacts_paths.retain(|test_path| {
            let test_path = Path::new(test_path);
            let test_path_stem = test_path
                .file_stem()
                .and_then(OsStr::to_str)
                .expect("Test path file stem should be valid UTF-8");
            let main_actor_stem = Path::new(&target_actor_path)
                .file_stem()
                .and_then(OsStr::to_str)
                .expect("Target Actor file stem should be valid UTF-8");

            if !test_path_stem.starts_with(main_actor_stem) {
                return true;
            }

            if test_path.is_file() {
                let Ok(test) = create_actor(test_path) else {
                        log::error!("Could not read test file {}", test_path.display());
                        return false;
                };
                actor_tests.push(test);
            } else {
                // Traverse the directory and subdirs looking for test files.
                let subdir_tests = WalkDir::new(test_path)
                    .into_iter()
                    .filter_map(Result::ok)
                    .filter_map(|tp| tp.into_path().into_os_string().into_string().ok())
                    .filter(|tp| tp.ends_with(".wasm"))
                    .filter_map(|tp| match create_actor(&tp) {
                        Ok(actor_test) => Some(actor_test),
                        Err(err) => {
                            log::error!("Could not read test file {}: {err}", tp);
                            None
                        }
                    });

                actor_tests.extend(subdir_tests);
                actor_tests.sort();
            }
            false
        });
        tests.push(Test {
            actor: target_actor,
            tests: actor_tests,
        });
    }

    // If there were left out files in the root, it is tests
    // without a parent target Actor.
    for left in test_artifacts_paths {
        log::warn!("Test {left} not read, it is missing its Actor");
    }

    Ok(tests)
}

#[cfg(test)]
mod tests {
    use super::search_files;
    use kythera_lib::{to_vec, Abi, Method};
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;
    use tempfile::tempdir;

    // Util to create actor files on a given directory path.
    fn set_actors_in_dir(dir_path: &Path, actors: Vec<(&str, &Abi)>) -> () {
        for (name, abi) in actors {
            File::create(dir_path.join(name.to_owned() + ".wasm"))
                .unwrap()
                .sync_data()
                .unwrap();

            let mut actor_abi_file =
                File::create(dir_path.join(name.to_owned() + ".cbor")).unwrap();
            actor_abi_file.write_all(&to_vec(abi).unwrap()).unwrap();
            actor_abi_file.sync_data().unwrap();
        }
    }

    #[test]
    fn actor_without_abi() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path();

        File::create(dir_path.join("token.wasm"))
            .unwrap()
            .sync_data()
            .unwrap();

        let tests = search_files(dir_path).unwrap();
        assert_eq!(0, tests.len());
    }

    #[test]
    fn actor_with_test_file() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path();
        let target_actor_abi = Abi {
            methods: vec![
                Method {
                    number: 1,
                    name: String::from("Constructor"),
                },
                Method {
                    number: 80475954,
                    name: String::from("Transfer"),
                },
            ],
        };
        let test_actor_abi = Abi {
            methods: vec![
                Method {
                    number: 1,
                    name: String::from("Constructor"),
                },
                Method {
                    number: 3760293944,
                    name: String::from("TestTransfer"),
                },
            ],
        };

        // Create target & test actors files.
        set_actors_in_dir(
            dir_path,
            vec![("token", &target_actor_abi), ("token.t", &test_actor_abi)],
        );

        let tests = search_files(dir_path).unwrap();
        assert_eq!(1, tests.len());
        let test = &tests[0];
        assert_eq!("token.wasm", test.actor.name());
        assert_eq!(&target_actor_abi, test.actor.abi());
        assert_eq!(1, test.tests.len());
        assert_eq!("token.t.wasm", test.tests[0].name());
        assert_eq!(&test_actor_abi, test.tests[0].abi());
    }

    #[test]
    fn actor_with_test_dir() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path();
        let target_actor_abi = Abi {
            methods: vec![
                Method {
                    number: 1,
                    name: String::from("Constructor"),
                },
                Method {
                    number: 80475954,
                    name: String::from("Transfer"),
                },
            ],
        };
        let test_1_actor_abi = Abi {
            methods: vec![
                Method {
                    number: 1,
                    name: String::from("Constructor"),
                },
                Method {
                    number: 1759653442,
                    name: String::from("TestTransferOne"),
                },
            ],
        };
        let test_2_actor_abi = Abi {
            methods: vec![
                Method {
                    number: 1,
                    name: String::from("Constructor"),
                },
                Method {
                    number: 3608985348,
                    name: String::from("TestTransferTwo"),
                },
            ],
        };

        // Create target actor files
        set_actors_in_dir(dir_path, vec![("token", &target_actor_abi)]);

        // Create tests directory
        let subdir_path = dir_path.join("token.t");
        fs::create_dir(&subdir_path).unwrap();

        // Setup tests assets.
        set_actors_in_dir(
            subdir_path.as_path(),
            vec![("test1", &test_1_actor_abi), ("test2", &test_2_actor_abi)],
        );

        let tests = search_files(dir_path).unwrap();
        assert_eq!(1, tests.len());
        let test = &tests[0];
        assert_eq!("token.wasm", test.actor.name());
        assert_eq!(&target_actor_abi, test.actor.abi());
        assert_eq!(2, test.tests.len());
        assert_eq!("test1.wasm", test.tests[0].name());
        assert_eq!(&test_1_actor_abi, test.tests[0].abi());
        assert_eq!("test2.wasm", test.tests[1].name());
        assert_eq!(&test_2_actor_abi, test.tests[1].abi());
    }

    #[test]
    fn actor_with_sub_test_dirs() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path();

        let target_actor_abi = Abi {
            methods: vec![
                Method {
                    number: 1,
                    name: String::from("Constructor"),
                },
                Method {
                    number: 80475954,
                    name: String::from("Transfer"),
                },
            ],
        };
        let test_1_actor_abi = Abi {
            methods: vec![
                Method {
                    number: 1,
                    name: String::from("Constructor"),
                },
                Method {
                    number: 1759653442,
                    name: String::from("TestTransferOne"),
                },
            ],
        };
        let test_2_1_actor_abi = Abi {
            methods: vec![
                Method {
                    number: 1,
                    name: String::from("Constructor"),
                },
                Method {
                    number: 2417587449,
                    name: String::from("TestTransferTwoOne"),
                },
            ],
        };
        let test_2_2_actor_abi = Abi {
            methods: vec![
                Method {
                    number: 1,
                    name: String::from("Constructor"),
                },
                Method {
                    number: 3281576210,
                    name: String::from("TestTransferTwoTwo"),
                },
            ],
        };

        // Create target actor files.
        set_actors_in_dir(dir_path, vec![("token", &target_actor_abi)]);

        let subdir_path = dir_path.join("token.t");
        fs::create_dir(&subdir_path).unwrap();

        // Setup test 1 assets.
        set_actors_in_dir(subdir_path.as_path(), vec![("test1", &test_1_actor_abi)]);

        let subsubdir_path = subdir_path.join("test2");
        fs::create_dir(&subsubdir_path).unwrap();

        // Setup test 2 assets
        set_actors_in_dir(
            subsubdir_path.as_path(),
            vec![
                ("test2.1", &test_2_1_actor_abi),
                ("test2.2", &test_2_2_actor_abi),
            ],
        );

        let tests = search_files(dir_path).unwrap();
        assert_eq!(1, tests.len());
        let test = &tests[0];
        assert_eq!("token.wasm", test.actor.name());
        assert_eq!(&target_actor_abi, test.actor.abi());
        assert_eq!(3, test.tests.len());
        assert_eq!("test1.wasm", test.tests[0].name());
        assert_eq!(&test_1_actor_abi, test.tests[0].abi());
        assert_eq!("test2.1.wasm", test.tests[1].name());
        assert_eq!(&test_2_1_actor_abi, test.tests[1].abi());
        assert_eq!("test2.2.wasm", test.tests[2].name());
        assert_eq!(&test_2_2_actor_abi, test.tests[2].abi());
    }
}
