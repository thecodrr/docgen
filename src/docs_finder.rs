use std::cmp::Ordering;
use std::ffi::OsStr;
use std::path::Path;
use std::sync::mpsc::channel;

use crate::config::Config;
use crate::Document;
use rayon::prelude::*;

use walkdir::WalkDir;

/// Loads the current state of the documentation from disk, returning the root
/// directory which contains all files and nested directories.
pub fn find(config: &Config) -> Vec<Document> {
    walk_dir(config.docs_dir(), config)
}

fn walk_dir<P: AsRef<Path>>(dir: P, config: &Config) -> Vec<Document> {
    let current_dir: &Path = dir.as_ref();

    let (sender, receiver) = channel();

    WalkDir::new(&current_dir)
        .follow_links(true)
        .into_iter()
        .par_bridge()
        .for_each_with(sender, |sender, entry| {
            if let Ok(entry) = entry {
                if entry.file_type().is_file() && entry.path().extension() == Some(OsStr::new("md"))
                {
                    let path = entry.path().strip_prefix(config.docs_dir()).unwrap();

                    sender
                        .send(Document::load(entry.path(), path, config.base_path()))
                        .unwrap();
                }
            }
        });

    let mut docs = vec![];

    receiver.iter().for_each(|doc| {
        docs.push(doc);
    });

    docs.par_sort_by(document_sort);

    docs
}

/// This is a special sort comparator that moves all README.md files to
/// to the top and positions all similarly nested directories together.
pub fn document_sort(b: &Document, a: &Document) -> Ordering {
    if a.path.ends_with("README.md") && b.path.ends_with("README.md") {
        return a.path.cmp(&b.path);
    } else if a.path.ends_with("README.md") {
        return Ordering::Less;
    } else if b.path.ends_with("README.md") {
        return Ordering::Greater;
    }

    return alphanumeric_sort::compare_path(&b.path, &a.path);
}
