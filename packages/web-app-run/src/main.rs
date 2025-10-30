use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::os::unix::process::CommandExt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<OsString> = env::args_os().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <url> [args...]", args[0].to_string_lossy());
        std::process::exit(2);
    }

    let app_arg = args.remove(1);
    let forwarded_args: Vec<OsString> = args.split_off(1); 

    let browser_name = match get_default_browser_name() {
        Ok(name) if !name.is_empty() => name,
        Ok(_) | Err(_) => {
            eprintln!("Error: could not determine default web browser via xdg-settings.");
            std::process::exit(1);
        }
    };

    let supported_browsers = [
        "chromium",
        "google-chrome",
        "brave-browser",
        "microsoft-edge",
        "opera",
        "vivaldi",
        "helium-browser",
    ];

    let is_browser_supported = supported_browsers
        .iter()
        .any(|p| browser_name.starts_with(p));

    if !is_browser_supported {
        eprintln!("Error: your default browser is not supported.");
        std::process::exit(1);
    }

    let exec_cmd = find_exec_from_desktop(&browser_name)
        .unwrap_or_else(|| {
            eprintln!(
                "Could not find browser executable for {} in known locations.",
                browser_name
            );
            std::process::exit(3);
        });

    let mut cmd = Command::new("setsid");

    cmd.arg("uwsm-app").arg("--").arg(exec_cmd).arg(format!("--app={}", osstring_to_string(&app_arg)));

    for a in forwarded_args {
        cmd.arg(a);
    }

    // .exec() will replace the current process on success and terminate it
    let err = cmd.exec();

    let ioerr = err as io::Error;
    eprintln!("Failed to start the browser: {}", ioerr);
    std::process::exit(4);
}

fn get_default_browser_name() -> Result<String, io::Error> {
    let out = Command::new("xdg-settings")
        .arg("get")
        .arg("default-web-browser")
        .output()?;

    if !out.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("xdg-settings exited with {}", out.status),
        ));
    }

    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok(s)
}

fn find_exec_from_desktop(browser_desktop: &str) -> Option<String> {
    let home = env::var("HOME").ok();

    let candidate_dirs: Vec<PathBuf> = {
        let mut v = Vec::new();
        if let Some(h) = &home {
            v.push(PathBuf::from(format!("{}/.local/share/applications", h)));
            v.push(PathBuf::from(format!("{}/.nix-profile/share/applications", h)));
        } else {
            v.push(PathBuf::from("/.nix-profile/share/applications"));
        }
        v.push(PathBuf::from("/usr/share/applications"));
        v
    };

    for dir in candidate_dirs {
        let file_path = dir.join(browser_desktop);
        if file_path.exists() {
            if let Ok(contents) = fs::read_to_string(&file_path) {
                if let Some(exec_token) = parse_first_exec_token(&contents) {
                    return Some(exec_token);
                }
            }
        }
    }

    None
}

fn parse_first_exec_token(contents: &str) -> Option<String> {
    for line in contents.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("Exec=") {
            continue;
        }
        let after = &trimmed["Exec=".len()..];
        let token = after.split_whitespace().next().map(|s| s.to_string());
        if token.is_some() {
            return token;
        }
    }
    None
}

fn osstring_to_string(s: &OsString) -> String {
    s.to_string_lossy().to_string()
}
