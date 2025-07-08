use serde::Serialize;
use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
};
use toml::Value;
use walkdir::WalkDir;

#[derive(Serialize)]
struct DependencyEntry {
    project: String,
    section: String,
    name: String,
    version: String,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let root = args.get(1).map(String::as_str).unwrap_or(".");
    let format = args.get(2).map(String::as_str).unwrap_or("json");

    let root_path = Path::new(root);
    let mut entries: Vec<DependencyEntry> = Vec::new();
    let mut visited_projects = HashSet::new();

    for entry in WalkDir::new(root_path).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.is_file() && path.file_name() == Some("Cargo.toml".as_ref()) {
            let project_path = path.display().to_string();

            if visited_projects.insert(project_path.clone()) {
                if let Ok(content) = fs::read_to_string(path) {
                    if let Ok(toml) = content.parse::<Value>() {
                        // handle normal dependencies
                        for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
                            if let Some(deps) = toml.get(section) {
                                collect_external_deps(&project_path, section, deps, &mut entries);
                            }
                        }

                        // also handle workspace.dependencies if present
                        if let Some(workspace) = toml.get("workspace") {
                            if let Some(ws_deps) = workspace.get("dependencies") {
                                collect_external_deps(
                                    &project_path,
                                    "workspace.dependencies",
                                    ws_deps,
                                    &mut entries,
                                );
                            }
                        }
                    }
                }

                // Optional: Collect version from Cargo.lock (not resolving real graph)
                //let lock_path = path.with_file_name("Cargo.lock");
                //if lock_path.exists() {
                //    if let Ok(lock_content) = fs::read_to_string(&lock_path) {
                //        if let Ok(lock_toml) = lock_content.parse::<Value>() {
                //            if let Some(packages) =
                //                lock_toml.get("package").and_then(|v| v.as_array())
                //            {
                //                for pkg in packages {
                //                    if let (Some(name), Some(version)) =
                //                        (pkg.get("name"), pkg.get("version"))
                //                    {
                //                        entries.push(DependencyEntry {
                //                            project: project_path.clone(),
                //                            section: "Cargo.lock".to_string(),
                //                            name: name.to_string(),
                //                            version: version.to_string(),
                //                        });
                //                    }
                //                }
                //            }
                //        }
                //    }
                //}
            }
        }
    }

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&entries).unwrap());
            println!("found {} deps !", &entries.len());
        }
        "csv" => {
            println!("project,section,name,version");
            for entry in &entries {
                println!(
                    "{},{},{},{}",
                    entry.project, entry.section, entry.name, entry.version
                );
            }
            println!("found {} deps !", &entries.len());
        }
        "md" | "markdown" => {
            println!("| Project | Section | Dependency | Version |");
            println!("|---------|---------|------------|---------|");
            for entry in &entries {
                println!(
                    "| `{}` | `{}` | `{}` | `{}` |",
                    entry.project, entry.section, entry.name, entry.version
                );
            }
            println!("found {} deps !", &entries.len());
        }
        _ => {
            eprintln!(
                "❌ Unsupported format: '{}'. Use 'json', 'csv', or 'markdown'.",
                format
            );
        }
    }
}

fn collect_external_deps(
    project: &str,
    section: &str,
    deps: &Value,
    entries: &mut Vec<DependencyEntry>,
) {
    if let Some(table) = deps.as_table() {
        for (name, value) in table {
            // Skip local dependencies
            let is_local = match value {
                Value::Table(t) => t.contains_key("path"),
                _ => false,
            };

            let is_workspace = match value {
                Value::Table(t) => t.get("workspace").and_then(|v| v.as_bool()) == Some(true),
                _ => false,
            };

            if is_local || is_workspace {
                continue;
            }

            let version = match value {
                Value::String(v) => v.clone(),
                Value::Table(tbl) => tbl
                    .get("version")
                    .or_else(|| tbl.get("git")) // allow git deps too
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                _ => "unknown".to_string(),
            };

            entries.push(DependencyEntry {
                project: project.to_string(),
                section: section.to_string(),
                name: name.to_string(),
                version,
            });
        }
    }
}
