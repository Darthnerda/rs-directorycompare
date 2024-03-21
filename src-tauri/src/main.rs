// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::hash_map::{DefaultHasher, RandomState};
use std::path::PathBuf;
// hide console window on Windows in release
use std::{ffi::OsString, path::Path};
use std::collections::{HashMap, HashSet};
use async_recursion::async_recursion;
use futures::TryFutureExt;
use rfd::FileDialog;
use serde::Serialize;
use std::sync::mpsc;
use std::{env, thread};
use std::hash::{BuildHasher, Hash, Hasher};

fn main() -> std::io::Result<()> {
    tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![choose_folder, choose_file, find_diffs])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
  Ok(())
}

#[derive(Clone)]
#[derive(Serialize)]
struct FileInfo {
    filename: String,
    path: PathBuf,
    should_copy: bool,
    size: u64
}

#[derive(Serialize)]
struct CompInfo {
    left: Vec<FileInfo>,
    right: Vec<FileInfo>,
    left_path: PathBuf,
    right_path: PathBuf
}

struct MyApp {
    dir_1_path: Option<PathBuf>,
    dir_2_path: Option<PathBuf>,
    dir1_save_path: Option<PathBuf>,
    dir2_save_path: Option<PathBuf>,
    working: bool,
    copying: bool,
    result: Option<CompInfo>,
    sender: mpsc::Sender<std::io::Result<CompInfo>>,
    receiver: mpsc::Receiver<std::io::Result<CompInfo>>,
    work_thread: Option<std::thread::JoinHandle<()>>,
    left_copy_thread: Option<std::thread::JoinHandle<Vec<Option<std::io::Error>>>>,
    right_copy_thread: Option<std::thread::JoinHandle<Vec<Option<std::io::Error>>>>,
}

fn do_copy(to_copy: Vec<FileInfo>, source_dir: PathBuf, target_dir: PathBuf) -> Vec<Option<std::io::Error>> {
    to_copy.iter()
        .filter(| FileInfo { should_copy, .. } | *should_copy)
        .map(| FileInfo { filename, path: source_path, ..} | {
            let target_path = Path::new(&target_dir)
                .join(source_path.strip_prefix(&source_dir).expect("Error: Path of source file not in source directory. Ask Josh"));
            // println!("{}, {}", source_path.display(), target_path.display());
            // Create all parent directories of the destination path
            if let Some(parent_dir) = target_path.parent() {
                std::fs::create_dir_all(parent_dir)?;
            }
            std::fs::copy(source_path, target_path)
        })
        .map(| result | result.err())
        .collect()
}

struct FileInfoHasher;
impl BuildHasher for FileInfoHasher {
    type Hasher = DefaultHasher;

    fn build_hasher(&self) -> DefaultHasher {
        DefaultHasher::new()
    }
}

impl Hash for FileInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.filename.hash(state);
        self.size.hash(state);
    }
}

impl PartialEq for FileInfo {
    fn eq(&self, other: &FileInfo) -> bool {
        self.filename == other.filename && self.size == other.size
    }
}

impl Eq for FileInfo {}

#[tauri::command]
fn choose_folder(dialog_title: String) -> Result<PathBuf, String> {
  FileDialog::new()
    .set_title(dialog_title)
    .pick_folder()
    .ok_or("No such folder".to_string())
}

#[tauri::command]
fn choose_file(dialog_title: String) -> Result<PathBuf, String> {
  FileDialog::new()
    .set_title(dialog_title)
    .pick_file()
    .ok_or("No such file".to_string())
}

#[tauri::command]
async fn find_diffs(dir1: String, dir2: String) -> Result<CompInfo, String> {
    let mut names1: HashSet<FileInfo, FileInfoHasher> = HashSet::with_capacity_and_hasher(10000, FileInfoHasher);
    let mut names2: HashSet<FileInfo, FileInfoHasher> = HashSet::with_capacity_and_hasher(10000, FileInfoHasher);

    let fut1 = collect_filenames(&dir1, &mut names1);
    let fut2 = collect_filenames(&dir2, &mut names2);

    let _ = tokio::join!(fut1, fut2);

    let _lefts: Vec<FileInfo> = names1.difference(&names2).map(|fi| fi.to_owned()).collect();
    let _rights: Vec<FileInfo> = names2.difference(&names1).map(|fi| fi.to_owned()).collect();

    Ok(CompInfo {
        left: _lefts,
        right: _rights,
        left_path: PathBuf::from(dir1),
        right_path: PathBuf::from(dir2)
    })
} 

#[async_recursion]
async fn collect_filenames<P: AsRef<Path> + std::marker::Send>(path: P, out: &mut HashSet<FileInfo, FileInfoHasher>) -> Result<(), String> {
    for entry in std::fs::read_dir(path).map_err(|err| err.to_string())? {
        let entry = match entry {
            Err(_) => continue,
            Ok(e) => e
        };
        let path = entry.path();
        let name = entry.file_name();
        let mut filesize: u64;
        if let Ok(metadata) = entry.metadata() {
            filesize = metadata.len();
        } else {
            continue
        }
        
        if path.is_dir() {
            collect_filenames(&path, out).await.ok();
        }

        let string_name: String = name.into_string().or_else(|_: OsString| -> Result<String, String> {Err("Couldn't convert OsString of filename to UTF-8 String".to_string())})?;

        out.insert(FileInfo {filename: string_name, size: filesize, path: path, should_copy: false});
    }
    Ok(())
}