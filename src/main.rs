use log::{debug, error};
use regex::Regex;
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::from_utf8;

// As of now(Jul 4, 2023), Archlinux still only has pre-built binaries for LLVM 15,
// CentOS 7 still only has LLVM 14 prebuilds. We will have to deal with older versions
// of clang.
const MAJOR_VERSIONS: &[u32] = &[16, 15, 14];

fn main() {
    env_logger::init();

    let mut args: Vec<String> = env::args().collect();
    if args.is_empty() {
        error!("Provide a binary name to locate for!");
        return;
    }
    let mut bin_name = Path::new(&args.remove(0))
        .file_name()
        .unwrap()
        .to_str()
        .expect("utf-8")
        .to_string();
    if bin_name == std::env!("CARGO_PKG_NAME") {
        bin_name = "clang".to_string();
    }
    let bin = find_bin(&bin_name, MAJOR_VERSIONS);
    debug!("Using {} from {}", bin_name, bin.display());
    let ignored_args = {
        let mut s: HashSet<&str> = HashSet::default();
        s.insert("-nostartfiles");
        s.insert("-Wno-nonnull-compare");
        s.insert("-Wno-dangling-pointer");
        s
    };
    let mut processed_args = vec![];
    for arg in args {
        if ignored_args.contains(arg.as_str()) {
            continue;
        }
        let re = Regex::new(r"--target=riscv(64|32)([^-]+)(-.+)").unwrap();
        if let Some(caps) = re.captures(&arg) {
            processed_args.push(format!("--target=riscv{}{}", &caps[1], &caps[3]));
            processed_args.push(format!("-march=rv{}{}", &caps[1], &caps[2]));
        } else {
            processed_args.push(arg);
        }
    }
    debug!("Processed args: {:?}", processed_args);
    let status = Command::new(bin)
        .args(processed_args)
        .status()
        .expect("running command!");
    std::process::exit(status.code().unwrap_or(-1));
}

fn find_bin(bin_name: &str, major_versions: &[u32]) -> PathBuf {
    for major_version in major_versions {
        let version_prefix = format!("{}.", major_version);
        // Check for LLVM installed for homebrew environment
        if let Some(prefix) = fetch_homebrew_prefix() {
            if let Some((bin, version)) =
                check_binary(&format!("{}/opt/llvm/bin/{}", prefix, bin_name))
            {
                if version.starts_with(&version_prefix) {
                    return bin;
                }
            }
            if let Some((bin, version)) = check_binary(&format!(
                "{}/opt/llvm@{}/bin/{}",
                prefix, major_version, bin_name
            )) {
                if version.starts_with(&version_prefix) {
                    return bin;
                }
            }
        }
        // Check default LLVM installation (most likely this is not what we want)
        if let Some((bin, version)) = check_binary(bin_name) {
            if version.starts_with(&version_prefix) {
                return bin;
            }
        }
        // Check binary with version suffix (apt installation on Ubuntu/Debian has
        // this suffix)
        if let Some((bin, version)) = check_binary(&format!("{}-{}", bin_name, major_version)) {
            if version.starts_with(&version_prefix) {
                return bin;
            }
        }
    }
    panic!(
        "Cannot find {} with major versions: {:?}, make sure you have LLVM properly installed!",
        bin_name, major_versions,
    );
}

fn check_binary(bin: &str) -> Option<(PathBuf, String)> {
    let path = if bin.contains('/') || bin.contains('\\') {
        bin.into()
    } else {
        which::which(bin).ok()?
    };
    let status = Command::new(&path).arg("--version").output().ok()?;
    let output = from_utf8(&status.stdout).ok()?;
    let re = Regex::new(r"version ([\S]+)").unwrap();
    let caps = re.captures(output)?;
    let version = caps.get(1)?;
    Some((path, version.as_str().to_string()))
}

fn fetch_homebrew_prefix() -> Option<String> {
    let status = Command::new("brew").arg("--prefix").output().ok()?;
    let output = from_utf8(&status.stdout).ok()?;
    Some(output.trim().to_string())
}
