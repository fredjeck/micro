use log::{info, warn};
use std::error::Error;
use std::fs;
use std::path::Path;
use std::thread::JoinHandle;
use std::time::SystemTime;
use std::{thread, time};

/// Spawns a process which periodically scouts for file changes.
/// Changes are passed to a handler function.
///
/// # Arguments
/// 
/// * `path` - Path to the directory which should be looked monitored
/// * `handler` - Function to which the path of changed items will be passed for action
/// * `recursive` - If true, will recursively look for changes in sub directories
/// * `poll_frequency` - Interval in milliseconds between file changes scans  
pub fn make_watcher(
    path: &'static Path,
    handler: fn(path: &Path),
    recursive: bool,
    poll_frequency: u64,
) -> JoinHandle<()> {
    info!("Watching {}", path.to_str().unwrap());
    let mut last_run = SystemTime::now();
    let child = thread::spawn(move || loop {

        let metadata = match fs::metadata(path){
            Ok(m) => m,
            Err(e) => panic!("Unable to monitor '{}' for changes, please make sure the path exits and points to a directory\n{}", path.to_str().unwrap(), e)
        };

        if !metadata.is_dir() {
            panic!("Unable to monitor '{}' for changes, please make sure the path exits and points to a directory", path.to_str().unwrap());
        }

        walk_dir(path, last_run, handler, recursive).unwrap();
        last_run = SystemTime::now();
        thread::sleep(time::Duration::from_millis(poll_frequency));
    });

    child
}

/// Recursively iterates through the items in a directorry scanning for changes
fn walk_dir(
    path: &Path,
    ref_time: std::time::SystemTime,
    handler: fn(path: &Path),
    recursive: bool,
) -> Result<(), Box<dyn Error>> {
    let contents = match fs::read_dir(path) {
        Ok(d) => d,
        Err(err) => {
            warn!("Unable to read '{}' contents\n{}", path.to_str().unwrap(), err);
            return Ok(());
        }
    };

    for entry in contents {
        let pathbuf = entry?.path();
        let filepath = pathbuf.as_path();

        let metadata = match fs::metadata(filepath) {
            Ok(md) => md,
            Err(err) => {
                warn!("Unable to stat '{}'\n{}", path.to_str().unwrap(), err);
                continue;
            }
        };

        if metadata.is_file() {
            if let Ok(time) = metadata.modified() {
                if time > ref_time {
                    handler(filepath);
                }
            }
        } else if recursive {
            walk_dir(filepath, ref_time, handler, recursive)?;
        }
    }

    Ok(())
}
