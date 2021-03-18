use log::warn;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

pub fn walk_dir<F>(path: PathBuf, extension: &'static str, recursive: bool, handler: &F)
where
    F: Fn(&Path)
{
    let directory = match std::fs::read_dir(&path) {
        Ok(d) => d,
        Err(e) => {
            warn!(
                "Unable to read '{:#?}' content, the directory will be skipped : {}",
                &path, e
            );
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
                warn!(
                    "Unable to stat '{:#?}', this item will be skipped\n{}",
                    &item_path, err
                );
                continue;
            }
        };

        if metadata.is_file()  {
            if item_path.extension() == Some(OsStr::new(extension)){
                handler(&item_path);
            }
        } else if recursive {
            walk_dir(item_path, extension, recursive, handler);
        }
    }
}
