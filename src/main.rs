use dotenv::dotenv;
use log::{error, info};
use notify::{event::{CreateKind, ModifyKind, DataChange, RenameMode}, Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use simple_logger::SimpleLogger;
use std::{env, process::Command, sync::mpsc::channel, time::Duration, path::Path};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, BufRead, BufReader};


fn watch_for_file_changes(
    src_dir: String,
    dest_user: String,
    dest_host: String,
    dest_dir: String,
    hashmap: HashMap<String, String>,
) -> notify::Result<()> {
    let (tx, rx) = channel();

    // Initialize watcher, set poll interval and watch path
    let mut watcher = RecommendedWatcher::new(
        tx,
        Config::default().with_poll_interval(Duration::from_secs(2)),
    )
    .unwrap();
    watcher.watch(src_dir.as_ref(), RecursiveMode::Recursive)?;

    for res in rx {
        match res {
            Ok(event) => match event.kind {
                EventKind::Create(CreateKind::File)
                | EventKind::Modify(ModifyKind::Data(DataChange::Any))
                | EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                    if event.paths[0].extension().and_then(|s| s.to_str()) == Some("csv") {
                        info!("CSV file event detected: {:?}", event);
                        let table_name = match_col_headers(event.paths[0].to_str().unwrap(), &hashmap);
                        info!("Table Name: {:?}", table_name);
                        if !table_name.unwrap().is_empty() {
                            run_rsync(&event.paths[0].to_str().unwrap(), &dest_user, &dest_host, &dest_dir);
                            delete_src_file(&event.paths[0].to_str().unwrap());
                        }
                    }
                },
                _ => (),
            },
            Err(e) => error!("Watch error: {:?}", e),
        }
    }

    Ok(())
}

fn match_col_headers(csv_path: &str, hashmap: &HashMap<String, String>) -> std::io::Result<String> {
    // match column header templates and returns the matching table name as a String
    if Path::new(csv_path).exists() {
        let csv_file = File::open(csv_path).unwrap();
        let reader = BufReader::new(csv_file);
        let csv_headers = reader.lines().next().unwrap_or_else(|| Ok(String::new()))?;
        info!("CSV Headers: {:?}", csv_headers);
        match hashmap.get(csv_headers.trim_end_matches(",")) {
            Some(table_name) => {
                println!("Match found, table name: {:?}", table_name);
                return Ok(table_name.to_string())
            },
            None => println!("No matching table header found. Ignoring csv file."),
        }
    }
    Ok(String::new())
}

fn delete_src_file(src_file: &str) {
    info!("Attempting to delete source file: {}", src_file);
    let result = Command::new("rm")
        .arg(src_file)
        .output()
        .expect("Failed to delete source file");
    if result.status.success() {
        info!("Deleted source file: {}", src_file);
    } else {
        error!("Error: {}", String::from_utf8_lossy(&result.stderr));
    }
}

fn run_rsync(src_file: &str, dest_user: &str, dest_host: &str, dest_dir: &str) {
    let rsync_command = format!(
        "rsync -aLvz --partial-dir=tmp {} {}@{}:{}",
        src_file, dest_user, dest_host, dest_dir
    );
    info!("Running rsync command: {}", rsync_command);
    let result = Command::new("sh")
        .arg("-c")
        .arg(&rsync_command)
        .output()
        .expect("Failed to execute rsync command");
    if result.status.success() {
        info!("Success: {}", String::from_utf8_lossy(&result.stdout));
    } else {
        error!("Error: {}", String::from_utf8_lossy(&result.stderr));
    }
}

fn load_env_vars() -> (String, String, String, String, String) {
    // Load environment variables and set rsync src and dest paths
    dotenv().ok();
    let src_dir = env::var("SOURCE_DIR").unwrap();
    let dest_user = env::var("DEST_USER").unwrap();
    let dest_host = env::var("DEST_HOST").unwrap();
    let dest_dir = env::var("DEST_DIR").unwrap();
    let template_dir = env::var("TEMPLATE_DIR").unwrap();
    (src_dir, dest_user, dest_host, dest_dir, template_dir)
}

fn load_headers(template_dir: String) -> std::io::Result<HashMap<String, String>> {
    // Load headers from template csv files and store in hashmap
    let mut table_headers: HashMap<String, String> = HashMap::new();
    let template_files = std::fs::read_dir(template_dir).unwrap();
    for template_file in template_files {
        let template_path = template_file?.path();
        match template_path.clone().file_stem() {
            Some(fname) => {
                match &fname.to_str() {
                    Some(v) => {
                        let table_name = v.strip_suffix("_template").unwrap().to_string();
                        let mut file = File::open(template_path).unwrap();
                        let mut headers = String::new();
                        let _ = file.read_to_string(&mut headers);
                        headers = headers.trim().to_string();
                        table_headers.insert(headers, table_name);
                    },
                    None => println!("Invalid File Name"),
                }
            },
            None => println!("No File Name"),
        }
    }
    Ok(table_headers)
}

fn main() -> std::io::Result<()> {
    SimpleLogger::new().init().unwrap();
    let (src_dir, dest_user, dest_host, dest_dir, template_dir) = load_env_vars();
    let hashmap = load_headers(template_dir)?;
    let _ = watch_for_file_changes(src_dir, dest_user, dest_host, dest_dir, hashmap);
    Ok(())
}