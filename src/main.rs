use log::debug;
use regex::Regex;
use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::str::from_utf8;

const MAJOR_VERSION: u32 = 16;

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().skip(1).collect();
    let clang = find_clang(MAJOR_VERSION);
    debug!("Using clang from {}", clang.display());
    let mut processed_args = vec![];
    for arg in args {
        let re = Regex::new(r"--target=riscv(64|32)([^-]+)(-.+)").unwrap();
        if let Some(caps) = re.captures(&arg) {
            processed_args.push(format!("--target=riscv{}{}", &caps[1], &caps[3]));
            processed_args.push(format!("-march=rv{}{}", &caps[1], &caps[2]));
        } else {
            processed_args.push(arg);
        }
    }
    debug!("Processed args: {:?}", processed_args);
    let status = Command::new(clang)
        .args(processed_args)
        .status()
        .expect("running clang!");
    std::process::exit(status.code().unwrap_or(-1));
}

fn find_clang(major_version: u32) -> PathBuf {
    let version_prefix = format!("{}.", major_version);
    if let Some((bin, version)) = check_binary("clang") {
        if version.starts_with(&version_prefix) {
            return bin;
        }
    }
    if let Some((bin, version)) = check_binary(&format!("clang-{}", major_version)) {
        if version.starts_with(&version_prefix) {
            return bin;
        }
    }
    panic!("Cannot find clang with major version {}!", major_version);
}

fn check_binary(bin: &str) -> Option<(PathBuf, String)> {
    let path = which::which(bin).ok()?;
    let status = Command::new(&path).arg("--version").output().ok()?;
    let output = from_utf8(&status.stdout).ok()?;
    let re = Regex::new(r"clang version ([\S]+)").unwrap();
    let caps = re.captures(output)?;
    let version = caps.get(1)?;
    Some((path, version.as_str().to_string()))
}
