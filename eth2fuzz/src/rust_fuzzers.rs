use failure::{Error, ResultExt};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use strum::IntoEnumIterator;

use crate::env::{corpora_dir, state_dir};
use crate::fuzzers::{write_fuzzer_target, FuzzerQuit};
use crate::targets::{prepare_targets_workspace, Targets};
use crate::utils::copy_dir;

/***********************************************
name: honggfuzz-rs
github: https://github.com/rust-fuzz/honggfuzz-rs
***********************************************/

pub struct FuzzerHfuzz {
    /// Fuzzer name.
    pub name: String,
    /// Source code / template dir
    pub dir: PathBuf,
    /// Workspace dir
    pub work_dir: PathBuf,
    /// Internal workspace dir
    pub workspace_dir: PathBuf,
    /// timeout
    pub timeout: Option<i32>,
    /// thread
    pub thread: Option<i32>,
}

impl FuzzerHfuzz {
    /// Check if hfuzz is installed
    pub fn is_available() -> Result<(), Error> {
        let fuzzer_output = Command::new("cargo").arg("hfuzz").arg("version").output()?;
        if !fuzzer_output.status.success() {
            bail!("hfuzz not available, install with `cargo install honggfuzz`");
        }
        Ok(())
    }

    /// Create a new FuzzerHfuzz
    pub fn new(timeout: Option<i32>, thread: Option<i32>) -> Result<FuzzerHfuzz, Error> {
        // Test if fuzzer engine installed
        FuzzerHfuzz::is_available()?;

        let cwd = env::current_dir().context("error getting current directory")?;
        let fuzzer = FuzzerHfuzz {
            name: "Honggfuzz".to_string(),
            dir: cwd.join("fuzzers").join("rust-honggfuzz"),
            work_dir: cwd.join("workspace").join("hfuzz"),
            workspace_dir: cwd.join("workspace").join("hfuzz").join("hfuzz_workspace"),
            timeout: timeout,
            thread: thread,
        };
        Ok(fuzzer)
    }

    fn prepare_fuzzer_workspace(&self) -> Result<(), Error> {
        let hfuzz_dir = &self.work_dir;
        fs::create_dir_all(&hfuzz_dir)
            .context(format!("unable to create {} dir", hfuzz_dir.display()))?;

        let src_dir = hfuzz_dir.join("src");
        fs::create_dir_all(&src_dir)
            .context(format!("unable to create {} dir", src_dir.display()))?;

        fs::copy(self.dir.join("Cargo.toml"), hfuzz_dir.join("Cargo.toml"))?;
        fs::copy(self.dir.join("template.rs"), hfuzz_dir.join("template.rs"))?;
        fs::copy(
            self.dir.join("simple_template.rs"),
            hfuzz_dir.join("simple_template.rs"),
        )?;
        fs::copy(self.dir.join("src").join("lib.rs"), src_dir.join("lib.rs"))?;
        Ok(())
    }

    pub fn run(&self, target: Targets) -> Result<(), Error> {
        // check if target is supported by this fuzzer
        // TODO - change to make it automatic
        if target.language() != "rust" {
            bail!("FuzzerHfuzz incompatible for this target");
        }

        let corpora_dir = corpora_dir()?.join(target.corpora());

        prepare_targets_workspace()?;
        // create hfuzz folder inside workspace/
        self.prepare_fuzzer_workspace()?;
        // write all fuzz targets inside hfuzz folder
        write_fuzzer_target(&self.dir, &self.work_dir, target)?;
        println!("[eth2diff] {}: {} created", self.name, target.name());

        let args = format!(
            "{} \
             {} \
             {} \
             {}",
            if let Some(t) = self.timeout {
                format!("--run_time {}", t)
            } else {
                "".into()
            },
            "-t 60",
            if let Some(n) = self.thread {
                // Set number of thread
                format!("-n {}", n)
            } else {
                "".into()
            },
            env::var("HFUZZ_RUN_ARGS").unwrap_or_default()
        );

        // Honggfuzz will first build than run the fuzzer using cargo
        let fuzzer_bin = Command::new("cargo")
            .args(&["+nightly", "hfuzz", "run", &target.name()])
            .env("HFUZZ_RUN_ARGS", &args)
            //.env("HFUZZ_BUILD_ARGS", "opt-level=3")
            .env("HFUZZ_INPUT", corpora_dir)
            .env(
                "ETH2FUZZ_BEACONSTATE",
                format!("{}", state_dir()?.display()),
            )
            .current_dir(&self.work_dir)
            .spawn()
            .context(format!(
                "error starting {} to run {}",
                self.name,
                target.name()
            ))?
            .wait()
            .context(format!(
                "error while waiting for {} running {}",
                self.name,
                target.name()
            ))?;

        if !fuzzer_bin.success() {
            Err(FuzzerQuit)?;
        }
        Ok(())
    }
}

/***********************************************
name: afl-rs
github: https://github.com/rust-fuzz/afl.rs
***********************************************/

pub struct FuzzerAfl {
    /// Fuzzer name.
    pub name: String,
    /// Source code / template dir
    pub dir: PathBuf,
    /// Workspace dir
    pub work_dir: PathBuf,
    /// Internal workspace dir
    pub workspace_dir: PathBuf,
    /// timeout
    pub timeout: Option<i32>,
    /// thread
    pub thread: Option<i32>,
}

impl FuzzerAfl {
    pub fn is_available() -> Result<(), Error> {
        let fuzzer_output = Command::new("cargo").arg("afl").arg("--version").output()?;
        if !fuzzer_output.status.success() {
            bail!("afl-rs not available, install with `cargo install honggfuzz`");
        }
        Ok(())
    }

    /// Create a new FuzzerAfl
    pub fn new(timeout: Option<i32>, thread: Option<i32>) -> Result<FuzzerAfl, Error> {
        // Test if fuzzer engine installed
        FuzzerAfl::is_available()?;

        let cwd = env::current_dir().context("error getting current directory")?;
        let fuzzer = FuzzerAfl {
            name: "Afl++".to_string(),
            dir: cwd.join("fuzzers").join("rust-afl"),
            work_dir: cwd.join("workspace").join("afl"),
            workspace_dir: cwd.join("workspace").join("afl").join("afl_workspace"),
            timeout: timeout,
            thread: thread,
        };
        Ok(fuzzer)
    }

    // TODO - simplify that
    fn prepare_fuzzer_workspace(&self) -> Result<(), Error> {
        let hfuzz_dir = &self.work_dir;
        fs::create_dir_all(&hfuzz_dir)
            .context(format!("unable to create {} dir", hfuzz_dir.display()))?;

        let src_dir = hfuzz_dir.join("src");
        fs::create_dir_all(&src_dir)
            .context(format!("unable to create {} dir", src_dir.display()))?;

        fs::copy(self.dir.join("Cargo.toml"), hfuzz_dir.join("Cargo.toml"))?;
        fs::copy(self.dir.join("template.rs"), hfuzz_dir.join("template.rs"))?;
        fs::copy(
            self.dir.join("simple_template.rs"),
            hfuzz_dir.join("simple_template.rs"),
        )?;
        fs::copy(self.dir.join("src").join("lib.rs"), src_dir.join("lib.rs"))?;
        Ok(())
    }

    /// Build single target with afl
    pub fn build_afl(&self, target: Targets) -> Result<(), Error> {
        prepare_targets_workspace()?;
        // create afl folder inside workspace/
        self.prepare_fuzzer_workspace()?;

        write_fuzzer_target(&self.dir, &self.work_dir, target)?;

        let build_cmd = Command::new("cargo")
            .args(&["+nightly", "afl", "build", "--bin", &target.name()]) // TODO: not sure we want to compile afl in "--release"
            .current_dir(&self.work_dir)
            .spawn()
            .context(format!(
                "error starting build for {} of {}",
                self.name,
                target.name()
            ))?
            .wait()
            .context(format!(
                "error while waiting for build for {} of {}",
                self.name,
                target.name()
            ))?;

        if !build_cmd.success() {
            Err(FuzzerQuit)?;
        }

        Ok(())
    }

    pub fn run(&self, target: Targets) -> Result<(), Error> {
        // check if target is supported by this fuzzer
        // TODO - change to make it automatic
        if target.language() != "rust" {
            bail!("FuzzerAfl incompatible for this target");
        }

        let dir = &self.work_dir;
        let corpora_dir = corpora_dir()?.join(target.corpora());

        self.build_afl(target)?;

        // TODO - modify to use same corpus than other fuzzer
        let corpus_dir = &self.workspace_dir;
        fs::create_dir_all(&corpus_dir)
            .context(format!("unable to create {} dir", corpus_dir.display()))?;

        // Determined if existing fuzzing session exist
        let queue_dir = corpus_dir.join("queue");
        let input_arg: &OsStr = if queue_dir.is_dir() && fs::read_dir(queue_dir)?.next().is_some() {
            "-".as_ref()
        } else {
            corpora_dir.as_ref()
        };

        let mut args: Vec<String> = Vec::new();
        args.push("+nightly".to_string());
        args.push("afl".to_string());
        args.push("fuzz".to_string());
        if let Some(t) = self.timeout {
            args.push(format!("-V {}", t));
        };

        // Run the fuzzer using cargo
        let fuzzer_bin = Command::new("cargo")
            .args(args)
            //.arg("-t 30000+" ) // increase timeout to let the fuzzer pick a valid beaconstate
            .arg("-m") // remove memory limit
            .arg("none")
            .arg("-i")
            .arg(&input_arg)
            .arg("-o")
            .arg(&corpus_dir)
            .args(&["--", &format!("./target/debug/{}", target.name())])
            .env(
                "ETH2FUZZ_BEACONSTATE",
                format!("{}", state_dir()?.display()),
            )
            // env variable to skip afl checking
            .env("AFL_SKIP_CPUFREQ", "1")
            .env("AFL_SKIP_CRASHES", "1")
            .env("AFL_I_DONT_CARE_ABOUT_MISSING_CRASHES", "1")
            .current_dir(&dir)
            .spawn()
            .context(format!(
                "error starting {:?} to run {}",
                self.name,
                target.name()
            ))?
            .wait()
            .context(format!(
                "error while waiting for {:?} running {}",
                self.name,
                target.name()
            ))?;

        if !fuzzer_bin.success() {
            Err(FuzzerQuit)?;
        }
        Ok(())
    }
}

/***********************************************
name: libfuzzer/cargo-fuzz
github: https://github.com/rust-fuzz/cargo-fuzz
***********************************************/

pub struct FuzzerLibfuzzer {
    /// Fuzzer name.
    pub name: String,
    /// Source code / template dir
    pub dir: PathBuf,
    /// Workspace dir
    pub work_dir: PathBuf,
    /// Internal workspace dir
    pub workspace_dir: PathBuf,
    /// timeout
    pub timeout: Option<i32>,
    /// thread
    pub thread: Option<i32>,
}

impl FuzzerLibfuzzer {
    pub fn is_available() -> Result<(), Error> {
        let fuzzer_output = Command::new("cargo")
            .arg("fuzz")
            .arg("--version")
            .output()?;
        if !fuzzer_output.status.success() {
            bail!("cargo-fuzz not available, install with `cargo install honggfuzz`");
        }
        Ok(())
    }

    /// Create a new FuzzerLibfuzzer
    pub fn new(timeout: Option<i32>, thread: Option<i32>) -> Result<FuzzerLibfuzzer, Error> {
        // Test if fuzzer engine installed
        FuzzerLibfuzzer::is_available()?;

        let cwd = env::current_dir().context("error getting current directory")?;
        let fuzzer = FuzzerLibfuzzer {
            name: "Libfuzzer".to_string(),
            dir: cwd.join("fuzzers").join("rust-libfuzzer"),
            work_dir: cwd.join("workspace").join("libfuzzer"),
            workspace_dir: cwd
                .join("workspace")
                .join("libfuzzer")
                .join("libfuzzer_workspace"),
            timeout: timeout,
            thread: thread,
        };
        Ok(fuzzer)
    }

    fn prepare_fuzzer_workspace(&self) -> Result<(), Error> {
        let from = &self.dir;
        let workspace = &self.work_dir;
        copy_dir(from.to_path_buf(), workspace.to_path_buf())?;
        Ok(())
    }

    pub fn run(&self, target: Targets) -> Result<(), Error> {
        // check if target is supported by this fuzzer
        // TODO - change to make it automatic
        if target.language() != "rust" {
            bail!("FuzzerLibfuzzer incompatible for this target");
        }

        prepare_targets_workspace()?;
        // create afl folder inside workspace/
        self.prepare_fuzzer_workspace()?;

        let fuzz_dir = self.work_dir.join("fuzz");
        fs::create_dir_all(&fuzz_dir)
            .context(format!("unable to create {} dir", fuzz_dir.display()))?;

        let target_dir = fuzz_dir.join("fuzz_targets");

        let _ = fs::remove_dir_all(&target_dir)
            .context(format!("error removing {}", target_dir.display()));
        fs::create_dir_all(&target_dir)
            .context(format!("unable to create {} dir", target_dir.display()))?;

        fs::create_dir_all(&fuzz_dir)
            .context(format!("unable to create {} dir", fuzz_dir.display()))?;
        //println!("{:?}", fuzz_dir);

        fs::copy(
            self.dir.join("fuzz").join("Cargo.toml"),
            fuzz_dir.join("Cargo.toml"),
        )?;

        // Add all targets to libfuzzer
        for target in Targets::iter().find(|x| x.language() == "rust") {
            write_libfuzzer_target(&self.work_dir, target)?;
        }

        let fuzz_dir = self.work_dir.join("fuzz");
        let corpus_dir = corpora_dir()?.join(target.corpora());

        // create arguments
        // corpora dir
        // max_time if provided (i.e. continuously fuzzing)
        let mut args: Vec<String> = Vec::new();
        args.push(format!("{}", &corpus_dir.display()));
        if let Some(timeout) = self.timeout {
            args.push("--".to_string());
            args.push(format!("-max_total_time={}", timeout));
        };

        // Launch the fuzzer using cargo
        let fuzzer_bin = Command::new("cargo")
            .env(
                "ETH2FUZZ_BEACONSTATE",
                format!("{}", state_dir()?.display()),
            )
            .args(&["+nightly", "fuzz", "run", &target.name()])
            .args(&args)
            .current_dir(&fuzz_dir)
            .spawn()
            .context(format!(
                "error starting {:?} to run {}",
                self.name,
                target.name()
            ))?
            .wait()
            .context(format!(
                "error while waiting for {:?} running {}",
                self.name,
                target.name()
            ))?;

        if !fuzzer_bin.success() {
            Err(FuzzerQuit)?;
        }
        Ok(())
    }
}

fn write_libfuzzer_target(fuzzer_dir: &PathBuf, target: Targets) -> Result<(), Error> {
    use std::io::Write;

    let fuzz_dir = fuzzer_dir.join("fuzz");
    let template_path = fuzzer_dir.join(target.template());

    let template = fs::read_to_string(&template_path).context(format!(
        "error reading template file {}",
        template_path.display()
    ))?;

    // use `cargo fuzz add` to add new bin inside Cargo.toml
    // and create fuzz_targets dir
    // and create target.rs
    let _ = Command::new("cargo")
        .args(&["+nightly", "fuzz", "add", &target.name()])
        .current_dir(&fuzzer_dir)
        .spawn()
        .context(format!("error adding {}", target.name()))?
        .wait()
        .context(format!("error while adding {}", target.name()));

    let target_dir = fuzz_dir.join("fuzz_targets");

    let path = target_dir.join(&format!("{}.rs", target.name()));

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .context(format!(
            "write_libfuzzer_target error writing fuzz target binary {}",
            path.display()
        ))?;

    let source = template.replace("###TARGET###", &target.name());
    file.write_all(source.as_bytes())?;
    Ok(())
}