use chrono::{self, TimeZone};
use dotenv::dotenv;
use log::{error, info};
use notify::{
    event::{DataChange, ModifyKind},
    Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use simple_logger::SimpleLogger;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::{
    env,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::channel,
    time::Duration,
};

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

    // If watcher errors out, log error and return
    if let Err(err) = watcher.watch(src_dir.as_ref(), RecursiveMode::Recursive) {
        error!("Failed to watch directory: {:?}", err);
        Err(err)?;
    }

    for res in rx {
        match res {
            Ok(event) => {
                match event.kind {
                    EventKind::Modify(ModifyKind::Data(DataChange::Any)) => {
                        // let _  = create_metadata_file(event.paths[0].to_str().unwrap());
                        if event.paths[0].extension().and_then(|s| s.to_str()) == Some("csv") {
                            info!("CSV file event detected: {:?}", event);
                            let src_file_basename =
                                event.paths[0].file_name().unwrap().to_str().unwrap();
                            let match_result =
                                match_col_headers(event.paths[0].to_str().unwrap(), &hashmap);
                            match match_result {
                                Ok(table_name) => {
                                    if !table_name.is_empty() {
                                        let metadata_file = match create_metadata_file(
                                            event.paths[0].to_str().unwrap(),
                                        ) {
                                            Ok(file) => file,
                                            Err(e) => {
                                                error!("Error creating metadata file: {:?}", e);
                                                String::new()
                                            }
                                        };
                                        run_rsync(
                                            &event.paths[0].to_str().unwrap(),
                                            &metadata_file,
                                            &dest_user,
                                            &dest_host,
                                            &dest_dir,
                                            &table_name,
                                        );
                                    }
                                }
                                Err(e) => {
                                    error!("Error matching column headers: {:?}", e);
                                    match &event.paths[0].parent() {
                                    Some(log_dir) => log_upload_status(log_dir.to_str().unwrap(), format!("Upload failed! File: {src_file_basename} Reason: {e}").to_string()),
                                    None => error!("Failed to get parent directory of source file."),
                                }
                                }
                            }
                        }
                    }
                    _ => (),
                }
            }
            Err(e) => error!("Watch error: {:?}", e),
        }
    }

    Ok(())
}

fn match_col_headers(csv_path: &str, hashmap: &HashMap<String, String>) -> std::io::Result<String> {
    // Match column header templates and returns the matching table name as a String
    if Path::new(csv_path).exists() {
        let csv_file = File::open(csv_path)?;
        let binding = PathBuf::from(csv_path);
        let csv_file_basename = binding.file_name().unwrap().to_str().unwrap();
        let reader = BufReader::new(csv_file);
        let csv_headers = reader.lines().next().unwrap_or_else(|| Ok(String::new()))?;
        info!("CSV Headers: {:?}", csv_headers);
        match hashmap.get(csv_headers.trim_end_matches(",")) {
            Some(table_name) => {
                info!("Matching table headers found, table name: {:?}", table_name);
                return Ok(table_name.to_string());
            }
            None => {
                info!("No matching table headers found. Ignoring csv file.");
                match PathBuf::from(csv_path).parent() {
                    Some(log_dir) => log_upload_status(log_dir.to_str().unwrap(), format!("Upload failed! File: {csv_file_basename} Reason: No matching table headers found.").to_string()),
                    None => error!("Failed to get parent directory of source file."),
                }
            }
        }
    }
    Ok(String::new())
}

fn delete_src_file_and_metadata(src_file: &str, src_file_metadata: &str) {
    // Delete source file and metadata after rsync
    let files_to_remove = vec![src_file, src_file_metadata];
    info!(
        "Attempting to delete source file and metadata: {}, {}",
        src_file, src_file_metadata
    );
    for file in files_to_remove {
        match fs::remove_file(file) {
            Ok(_) => println!("Successfully removed {}", file),
            Err(e) => eprintln!("Failed to remove {}: {}", file, e),
        }
    }
}

fn log_upload_status(log_dir: &str, log_msg: String) {
    // Create an upload log file at specified log directory
    let log_file_path = Path::new(log_dir).join("upload.log");
    let log_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    match fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(log_file_path)
    {
        Ok(mut log_file) => match log_file.write(format!("{log_time} - {log_msg}\n").as_bytes()) {
            Ok(_) => info!("Upload log file updated successfully."),
            Err(e) => error!("Failed to write to upload log file. Error: {}", e),
        },
        Err(e) => error!("Failed to create upload log file. Error: {}", e),
    }
}

fn run_rsync(
    src_file: &str,
    src_file_metadata: &str,
    dest_user: &str,
    dest_host: &str,
    dest_dir: &str,
    table_name: &str,
) {
    // Run rsync command to sync csv files to destination host
    let mkdir_command = format!(
        "\"mkdir -p \"{}\" && rsync\"",
        PathBuf::from(dest_dir).join(table_name).display()
    );
    let rsync_command = format!(
        "rsync -aLvz --partial-dir=tmp --rsync-path={} \"{}\" \"{}\" {}@{}:{}",
        mkdir_command,
        src_file,
        src_file_metadata,
        dest_user,
        dest_host,
        PathBuf::from(dest_dir).join(table_name).display()
    );
    let binding = PathBuf::from(src_file);
    let src_file_basename = binding.file_name().unwrap().to_str().unwrap();
    info!("Running rsync command: {}", rsync_command);
    match Command::new("sh").arg("-c").arg(&rsync_command).output() {
        Ok(output) => {
            if output.status.success() {
                info!("Success: {}", String::from_utf8_lossy(&output.stdout));
                delete_src_file_and_metadata(src_file, src_file_metadata);
                match PathBuf::from(src_file).parent() {
                    Some(log_dir) => log_upload_status(
                        log_dir.to_str().unwrap(),
                        format!("Upload succeeded! File: {src_file_basename}").to_string(),
                    ),
                    None => error!("Failed to get source file parent directory"),
                }
            } else {
                let err_msg = String::from_utf8_lossy(&output.stderr);
                error!("Error: {}", err_msg);
                match PathBuf::from(src_file).parent() {
                    Some(log_dir) => log_upload_status(
                        log_dir.to_str().unwrap(),
                        format!("Upload failed! File: {src_file_basename} Reason: {err_msg}")
                            .to_string(),
                    ),
                    None => error!("Failed to get source file parent directory"),
                }
            }
        }
        Err(e) => error!("Failed to execute rsync command. Error: {}", e),
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
            Some(fname) => match &fname.to_str() {
                Some(v) => {
                    let table_name = v.strip_suffix("_template").unwrap().to_string();
                    let mut file = File::open(template_path).unwrap();
                    let mut headers = String::new();
                    let _ = file.read_to_string(&mut headers);
                    headers = headers.trim().to_string();
                    table_headers.insert(headers, table_name);
                }
                None => println!("Invalid File Name"),
            },
            None => println!("No File Name"),
        }
    }
    Ok(table_headers)
}

fn create_metadata_file(src_file: &str) -> std::io::Result<String> {
    // Create metadata file
    let attr = fs::metadata(src_file)?;
    let mut username: String = "".to_string();
    match Command::new("id")
        .arg("-u")
        .arg("-n")
        .arg(attr.uid().to_string())
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                username = String::from_utf8_lossy(&output.stdout)
                    .strip_suffix("\n")
                    .unwrap()
                    .to_string();
            } else {
                username = "".to_string();
            }
        }
        Err(e) => {
            error!("Failed to execute id command. Error: {}", e);
        }
    };
    let elapsed_secs = attr
        .created()?
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let upload_time = chrono::Local
        .timestamp_opt(elapsed_secs, 0)
        .unwrap()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let binding = PathBuf::from(src_file);
    let src_file_basename = binding.file_name().unwrap().to_string_lossy().to_string();
    let metadata_data = format!("{},{},{}", upload_time, username, src_file_basename);
    let metadata_file_path = format!("{}.metadata", src_file);
    info!(
        "Creating metadata file {:?} with metadata: {:?}",
        metadata_file_path, metadata_data
    );
    let mut metadata_file = match File::create(&metadata_file_path) {
        Ok(file) => file,
        Err(err) => {
            error!("Failed to create metadata file: {:?}", err);
            return Err(err)?;
        }
    };
    metadata_file.write_all(metadata_data.as_bytes())?;
    info!("Metadata file created successfully.");
    Ok(metadata_file_path)
}

fn main() -> std::io::Result<()> {
    SimpleLogger::new().init().unwrap();
    let (src_dir, dest_user, dest_host, dest_dir, template_dir) = load_env_vars();
    let hashmap = load_headers(template_dir)?;
    let _ = watch_for_file_changes(src_dir, dest_user, dest_host, dest_dir, hashmap);
    Ok(())
}
