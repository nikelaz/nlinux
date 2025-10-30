use clap::Parser;
use dialoguer::{theme::ColorfulTheme, Input};
use dirs::home_dir;
use std::fs::{self, File};
use std::io::Write;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    app_name: Option<String>,
    app_url: Option<String>,
    icon_url: Option<String>,
    mime_types: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let theme = ColorfulTheme::default();

    let interactive = args.app_name.is_none() || args.app_url.is_none() || args.icon_url.is_none();

    let app_name = match args.app_name {
        Some(v) => v,
        None => Input::with_theme(&theme)
            .with_prompt("App name")
            .interact_text()?,
    };

    let app_url = match args.app_url {
        Some(v) => v,
        None => Input::with_theme(&theme)
            .with_prompt("App URL")
            .interact_text()?,
    };

    let icon_url = match args.icon_url {
        Some(v) => v,
        None => Input::with_theme(&theme)
            .with_prompt("Icon URL (PNG)")
            .interact_text()?,
    };

    let mime_types = args.mime_types.or_else(|| {
        Input::with_theme(&theme)
            .with_prompt("MIME types (optional)")
            .allow_empty(true)
            .interact_text()
            .ok()
    });

    let home = home_dir().ok_or("Could not determine home directory")?;
    let icons_dir = home.join(".local/share/applications/icons");
    let apps_dir = home.join(".local/share/applications");

    fs::create_dir_all(&icons_dir)?;
    fs::create_dir_all(&apps_dir)?;

    let icon_path = icons_dir.join(format!("{}.png", app_name));

    if icon_url.starts_with("http://") || icon_url.starts_with("https://") {
        println!("Downloading icon...");
        let response = reqwest::blocking::get(&icon_url)?;
        if !response.status().is_success() {
            eprintln!("Failed to download icon: {}", response.status());
            std::process::exit(1);
        }
        let bytes = response.bytes()?;
        fs::write(&icon_path, bytes)?;
    } else {
        fs::copy(&icon_url, &icon_path)?;
    }

    let desktop_file_path = apps_dir.join(format!("{}.desktop", app_name));
    let exec_command = format!("web-app-run {}", app_url);

    let mut desktop_file = File::create(&desktop_file_path)?;
    writeln!(
        desktop_file,
        "[Desktop Entry]\nVersion=1.0\nName={}\nComment={} Web App\nExec={}\nTerminal=false\nType=Application\nIcon={}\nStartupNotify=true",
        app_name,
        app_name,
        exec_command,
        icon_path.display()
    )?;

    if let Some(types) = mime_types {
        if !types.trim().is_empty() {
            writeln!(desktop_file, "MimeType={}", types)?;
        }
    }

    // Make executable
    let mut perms = fs::metadata(&desktop_file_path)?.permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o755);
    }
    fs::set_permissions(&desktop_file_path, perms)?;

    println!("\n✅ Created web app launcher: {}", desktop_file_path.display());
    if interactive {
        println!("You can now find '{}' in your app launcher.", app_name);
    }

    Ok(())
}
