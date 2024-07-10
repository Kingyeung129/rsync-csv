use dotenv::dotenv;
use log::{error, info};
use notify::{event::{CreateKind, ModifyKind, DataChange, RenameMode}, Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use simple_logger::SimpleLogger;
use std::{env, process::Command, sync::mpsc::channel, time::Duration};

fn watch_for_file_changes(
    src_dir: String,
    dest_user: String,
    dest_host: String,
    dest_dir: String,
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
                        run_rsync(&src_dir, &dest_user, &dest_host, &dest_dir);
                    }
                },
                _ => (),
            },
            Err(e) => error!("Watch error: {:?}", e),
        }
    }

    Ok(())
}

fn run_rsync(src_dir: &str, dest_user: &str, dest_host: &str, dest_dir: &str) {
    let rsync_command = format!(
        "rsync -avz {} {}@{}:{}",
        src_dir, dest_user, dest_host, dest_dir
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

fn load_env_vars() -> (String, String, String, String) {
    // Load environment variables and set rsync src and dest paths
    dotenv().ok();
    let src_dir = env::var("SOURCE_DIR").unwrap();
    let dest_user = env::var("DEST_USER").unwrap();
    let dest_host = env::var("DEST_HOST").unwrap();
    let dest_dir = env::var("DEST_DIR").unwrap();
    (src_dir, dest_user, dest_host, dest_dir)
}

fn main() -> std::io::Result<()> {
    SimpleLogger::new().init().unwrap();
    let (src_dir, dest_user, dest_host, dest_dir) = load_env_vars();
    let _ = watch_for_file_changes(src_dir, dest_user, dest_host, dest_dir);
    Ok(())
}
