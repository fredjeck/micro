
use futures::{
    future::{BoxFuture},
    FutureExt,
};
use log::{error, info, warn};
use std::{path::PathBuf};
use std::{time::SystemTime};
use tokio::{
    fs,
    sync::mpsc::{Sender},
    task::JoinHandle,
    time::{sleep, Duration},
};

/// Spawns a process which periodically scouts for file changes.
/// Changes are passed to a handler function.
///
/// # Arguments
///
/// * `path` - Path to the directory which should be looked monitored
/// * `sender` - A channel to which modified paths will be sent to
/// * `recursive` - If true, will recursively look for changes in sub directories
/// * `poll_frequency` - Interval in milliseconds between file changes scans  
pub async fn make_fs_watcher(
    path: PathBuf,
    sender: Sender<String>,
    recursive: bool,
    poll_frequency: u64,
) -> JoinHandle<()> {
    info!("Watching {:#?} for changes", path);

    let metadata = match fs::metadata(&path).await{
        Ok(m) => m,
        Err(e) => panic!("Unable to watch '{:#?}' for changes, please make sure the path exits and points to a directory\n{}", path, e)
    };

    if !metadata.is_dir() {
        panic!(
            "make_fs_watcher can only watch directories and it looks like '{:#?}' isn't one",
            path
        );
    }
    let mut last_run = SystemTime::now();

    let task = tokio::task::spawn(async move {
        loop {
            scan_changes(path.clone(), last_run, sender.clone(), recursive).await;
            last_run = SystemTime::now();
            sleep(Duration::from_millis(poll_frequency)).await; 
        }
    });
    task
}

fn scan_changes(
    path: PathBuf,
    ref_time: std::time::SystemTime,
    sender: Sender<String>,
    recursive: bool,
) -> BoxFuture<'static, ()> {
    async move {
        let directory = match std::fs::read_dir(&path) { 
            Ok(d) => d,
            Err(e) => {
                warn!("Unable to read '{:#?}' content, the directory will be skipped : {}", &path, e);
                return ();
            }
        };

        for entry in directory {
            let item_path = match entry {
                Ok(p) => p.path(),
                Err(e) => {
                    warn!(
                        "An error occured while iterating through '{:#?}', the faulty item will be skipped : {}",
                        &path, e
                    );
                    continue;
                }
            };

            let metadata = match std::fs::metadata(&item_path) {
                Ok(md) => md,
                Err(err) => {
                    warn!("Unable to stat '{:#?}', this item will be skipped\n{}", &item_path, err);
                    continue;
                }
            };

            if metadata.is_file() {
                if let Ok(time) = metadata.modified() {
                    if time > ref_time {
                        if let Err(e) = sender.send(String::from(item_path.to_str().unwrap())).await
                        {
                            error!("Unable to notifty a detected change to '{:#?}' : {}", &item_path, e);
                        }
                    }
                }
            } else if recursive {
                scan_changes(item_path, ref_time, sender.clone(), recursive).await;
            }
        }
    }
    .boxed()
}
