use futures::{Future, FutureExt, future::BoxFuture};
use log::{info, warn};
use std::{fs, pin::Pin};
use std::path::Path;
use std::time::SystemTime;
use std::{error::Error, ffi::OsStr, path::PathBuf};
use std::{thread, time};
use tokio::{
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
    time::{sleep, Duration},
};

trait PathChangeHandler {}

/// Spawns a process which periodically scouts for file changes.
/// Changes are passed to a handler function.
///
/// # Arguments
///
/// * `path` - Path to the directory which should be looked monitored
/// * `handler` - Function to which the path of changed items will be passed for action
/// * `recursive` - If true, will recursively look for changes in sub directories
/// * `poll_frequency` - Interval in milliseconds between file changes scans  
pub fn make_fs_watcher(
    path: PathBuf,
    handler: Sender<String>,
    recursive: bool,
    poll_frequency: u64,
) -> JoinHandle<()> {
    info!("Now watching '{}' for changes", path.to_str().unwrap());
    let mut last_run = SystemTime::now();

    let child = tokio::task::spawn(async move {
        loop {
            let metadata = match fs::metadata(&path){
                Ok(m) => m,
                Err(e) => panic!("Unable to watch '{:#?}' for changes, please make sure the path exits and points to a directory\n{}", path, e)
            };

            if !metadata.is_dir() {
                panic!("Unable to watch '{:#?}' for changes, please make sure the path exits and points to a directory", path);
            }

            scan_changes(&path, last_run, &handler, recursive);
            last_run = SystemTime::now();
            sleep(Duration::from_millis(poll_frequency)).await;
        }
    });

    child
}

/// Iterates through the elements nested under the provided path scanning for changes.
///
/// Handler function will be invoked for any element which modification timestamp is older than the given **ref_time**.
/// This function will return whenever all the items (and sub-items if recursive is set to true) are scanned.
async fn scan_changes(
    path: &Path,
    ref_time: std::time::SystemTime,
    handler: &Sender<String>,
    recursive: bool,
) -> Result<(), Box<dyn Error>> {
    let contents = match fs::read_dir(path) {
        Ok(d) => d,
        Err(err) => {
            warn!(
                "Unable to read '{}' contents\n{}",
                path.to_str().unwrap(),
                err
            );
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
                    handler.send(filepath.to_str().unwrap().to_owned());
                }
            }
        } else if recursive {
            scan_changes(filepath, ref_time, handler, recursive);
        }
    }

    Ok(())
}

/// Iterates through the elements nested under the provided path searching for files with the given extension.
///
/// Handler function will be invoked for any element which modification timestamp is older than the given **ref_time**.
/// This function will return whenever all the items (and sub-items if recursive is set to true) are scanned.
pub async fn walk_dir(
    path: &Path,
    extension: &str,
    handler: fn(path: &Path),
    recursive: bool,
) -> Pin<Box<dyn Future<Output=()>>> {
    let contents = match fs::read_dir(path) {
        Ok(d) => d,
        Err(err) => {
            warn!(
                "Unable to read '{}' contents\n{}",
                path.to_str().unwrap(),
                err
            );
        }
    };

    for entry in contents {
        let pathbuf = entry.unwrap.path();
        let filepath = pathbuf.as_path();

        let metadata = match fs::metadata(filepath) {
            Ok(md) => md,
            Err(err) => {
                warn!("Unable to stat '{}'\n{}", path.to_str().unwrap(), err);
                continue;
            }
        };

        if metadata.is_file() {
            if pathbuf.extension() == Some(OsStr::new(extension)) {
                handler(filepath);
            }
        } else if recursive {
            async move {
                walk_dir(filepath, extension, handler, recursive).await;
            }
            .boxed()
        }
    }

    ()
}
