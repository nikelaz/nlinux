// Cargo.toml dependencies:
// [dependencies]
// slint = "1.12.1"
// freedesktop-desktop-entry = "0.5"
// fuzzy-matcher = "0.3"
// 
// [build-dependencies]
// slint-build = "1.12.1"

slint::include_modules!();

use freedesktop_desktop_entry::{DesktopEntry, Iter};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use slint::{ModelRc, VecModel};
use std::process::Command;
use std::rc::Rc;
use std::borrow::Cow;

#[derive(Clone)]
struct AppInfo {
    name: String,
    exec: String,
    icon: String,
    description: String,
}

fn parse_desktop_files() -> Vec<AppInfo> {
    let mut apps = Vec::new();
    
    for path in Iter::new(freedesktop_desktop_entry::default_paths()) {
        if let Ok(bytes) = std::fs::read_to_string(&path) {
            if let Ok(desktop_entry) = DesktopEntry::decode(&path, &bytes) {
                if desktop_entry.no_display() {
                    continue;
                }
                
                let name = desktop_entry.name(None)
                    .unwrap_or(Cow::Borrowed("Unknown"))
                    .to_string();
                
                let exec = desktop_entry.exec()
                    .unwrap_or("")
                    .to_string();
                
                if exec.is_empty() {
                    continue;
                }
                
                let icon = desktop_entry.icon()
                    .unwrap_or("")
                    .to_string();
                
                let description = desktop_entry.comment(None)
                    .unwrap_or(Cow::Borrowed(""))
                    .to_string();
                
                apps.push(AppInfo {
                    name,
                    exec,
                    icon,
                    description,
                });
            }
        }
    }
    
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

fn launch_app(exec: &str) {
    // Clean up exec string (remove field codes like %f, %u, etc.)
    let clean_exec = exec
        .split_whitespace()
        .filter(|s| !s.starts_with('%'))
        .collect::<Vec<_>>()
        .join(" ");
    
    // Launch using sh for proper handling
    let _ = Command::new("sh")
        .arg("-c")
        .arg(&clean_exec)
        .spawn();
}

fn main() {
    let ui = AppWindow::new().unwrap();
    
    // Load all applications
    let all_apps = parse_desktop_files();
    let all_apps_rc = Rc::new(all_apps);
    
    // Initialize with all apps
    let app_model: Rc<VecModel<AppEntry>> = Rc::new(VecModel::default());
    for app in all_apps_rc.iter() {
        app_model.push(AppEntry {
            name: app.name.clone().into(),
            description: app.description.clone().into(),
            icon: app.icon.clone().into(),
        });
    }
    ui.set_apps(ModelRc::from(app_model.clone()));
    
    // Handle search
    let ui_weak = ui.as_weak();
    let all_apps_search = all_apps_rc.clone();
    ui.on_search(move |query| {
        let ui = ui_weak.unwrap();
        let app_model: Rc<VecModel<AppEntry>> = Rc::new(VecModel::default());
        
        if query.is_empty() {
            // Show all apps
            for app in all_apps_search.iter() {
                app_model.push(AppEntry {
                    name: app.name.clone().into(),
                    description: app.description.clone().into(),
                    icon: app.icon.clone().into(),
                });
            }
        } else {
            // Fuzzy search
            let matcher = SkimMatcherV2::default();
            let query_str = query.to_string();
            
            let mut scored_apps: Vec<_> = all_apps_search
                .iter()
                .filter_map(|app| {
                    matcher
                        .fuzzy_match(&app.name.to_lowercase(), &query_str.to_lowercase())
                        .map(|score| (app, score))
                })
                .collect();
            
            scored_apps.sort_by(|a, b| b.1.cmp(&a.1));
            
            for (app, _) in scored_apps {
                app_model.push(AppEntry {
                    name: app.name.clone().into(),
                    description: app.description.clone().into(),
                    icon: app.icon.clone().into(),
                });
            }
        }
        
        ui.set_apps(ModelRc::from(app_model));
    });
    
    // Handle app launch
    let ui_weak = ui.as_weak();
    let all_apps_launch = all_apps_rc.clone();
    ui.on_launch_app(move |name| {
        let name_str = name.to_string();
        if let Some(app) = all_apps_launch.iter().find(|a| a.name == name_str) {
            launch_app(&app.exec);
            ui_weak.unwrap().hide().unwrap();
        }
    });
    
    // Handle close
    let ui_weak = ui.as_weak();
    ui.on_close(move || {
        ui_weak.unwrap().hide().unwrap();
    });
    
    ui.run().unwrap();
}
