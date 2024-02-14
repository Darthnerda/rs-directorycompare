#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] use std::hash::Hash;
use std::path::PathBuf;
// hide console window on Windows in release
use std::{ffi::OsString, path::Path};
use std::collections::{HashMap, HashSet};
use async_recursion::async_recursion;
use eframe::egui::scroll_area::ScrollBarVisibility;
use eframe::egui::{self, Response,ScrollArea};
use egui_extras::{TableBuilder, Column};
use rfd::FileDialog;
use std::sync::mpsc;
use std::{env, thread};


fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 550.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Directory Comparer!",
        options,
        Box::new(|cc| {
            Box::<MyApp>::default()
        }),
    )
}

struct MyApp {
    dir1Path: Option<PathBuf>,
    dir2Path: Option<PathBuf>,
    dir1_save_path: Option<PathBuf>,
    dir2_save_path: Option<PathBuf>,
    working: bool,
    result: Option<(Vec<(OsString, bool)>, Vec<(OsString, bool)>, HashMap<OsString, PathBuf>, HashMap<OsString, PathBuf>)>,
    sender: mpsc::Sender<std::io::Result<(Vec<OsString>, Vec<OsString>, HashMap<OsString, PathBuf>, HashMap<OsString, PathBuf>)>>,
    receiver: mpsc::Receiver<std::io::Result<(Vec<OsString>, Vec<OsString>, HashMap<OsString, PathBuf>, HashMap<OsString, PathBuf>)>>,
    work_thread: Option<std::thread::JoinHandle<()>>,
    copy_thread: Option<std::thread::JoinHandle<Vec<Option<std::io::Error>>>>,
    dir1_entries_selected: Selected,
    dir2_entries_selected: Selected
}

struct Selected {
    pub bool: bool,
    pub indeterminate: bool
}

impl Selected {
    fn new(bool: bool, indeterminate: bool) -> Selected {
        Selected { bool, indeterminate }
    }
}

impl Default for MyApp {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();

        Self {
            dir1Path: None,
            dir2Path: None,
            dir1_save_path: None,
            dir2_save_path: None,
            working: false,
            result: None,
            sender: sender,
            receiver: receiver,
            work_thread: None,
            copy_thread: None,
            dir1_entries_selected: Selected::new(false, false),
            dir2_entries_selected: Selected::new(false, false)
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Directory Comparer!");
            ui.columns(2, |uis| {
                for i in 0..2 {
                    let ui = &mut uis[i];

                    if i == 0 {
                        if !self.working && ui.button("+ Directory").clicked() {
                            self.dir1Path = FileDialog::new()
                                .set_title("Choose a Directory to Compare.")
                                .pick_folder();
                        }
                        if self.dir1Path.is_some() {
                            ui.label(format!("Directory 1: {:?}", self.dir1Path.as_mut().unwrap().as_mut_os_string()));
                        }
                    }

                    if i == 1 {
                        if !self.working && ui.button("+ Directory").clicked() {
                            self.dir2Path = FileDialog::new()
                                .set_title("Choose a Directory to Compare.")
                                .pick_folder();
                        }
                        if self.dir2Path.is_some() {
                            ui.label(format!("Directory 2: {:?}", self.dir2Path.as_mut().unwrap().as_mut_os_string()));
                        }
                    }
                }
            });

            if self.dir1Path.is_some() && self.dir2Path.is_some() {
                if !self.working && ui.button("Find Unique Files").clicked() {

                    self.result = None;
                    self.working = true;

                    let sndr = self.sender.to_owned();

                    let dir1 = self.dir1Path.as_mut().unwrap().as_mut_os_string().to_owned();
                    let dir2 = self.dir2Path.as_mut().unwrap().as_mut_os_string().to_owned();

                    self.work_thread = Some(thread::spawn(move || {
                        let result: Result<(Vec<OsString>, Vec<OsString>, HashMap<OsString, PathBuf>, HashMap<OsString, PathBuf>), std::io::Error> = tokio::runtime::Builder::new_multi_thread()
                            .enable_all()
                            .build()
                            .unwrap()
                            .block_on(find_diffs(&dir1, &dir2));
                        sndr.send(result).expect("Error: Work thread failed to send result to GUI thread. Ask Josh.");
                    }));

                }
            }

            let res: Result<(), String> = match self.receiver.try_recv() {
                Err(mpsc::TryRecvError::Empty) => Err("".to_string()),
                Err(mpsc::TryRecvError::Disconnected) => Err("Error: Work thread disconnected. Probably crashed. Ask Josh.".to_string()),
                Ok(r) => match r {
                    Err(e) => Err(e.to_string()),
                    Ok((lefts, rights, left_path_map, right_path_map)) => Ok({
                        let lv: Vec<(OsString, bool)> = lefts.into_iter().map(|st| (st, false)).collect();
                        let rv: Vec<(OsString, bool)> = rights.into_iter().map(|st| (st, false)).collect();
                        self.result = Some((lv, rv, left_path_map, right_path_map));
                        self.working = false;
                    })
                }
            };

            if res.is_err() {
                ui.label(res.err().unwrap());
            }

            if self.working {
                ui.label("Working on it...");
            }

            if self.result.is_some() {

                let (lefts, rights, left_path_map, right_path_map) = self.result.as_mut().unwrap();

                if self.dir1_save_path.is_some() || self.dir2_save_path.is_some() {
                    if ui.button("Begin copy of missing files.").clicked() {
                        if self.dir1_save_path.is_some() {
                            let target_dir = self.dir1_save_path.as_ref().unwrap().clone();
                            let right = rights.clone();
                            let path_map = left_path_map.clone();
                            // self.copy_thread = Some(thread::spawn(move || do_copy(right, path_map, target_dir)));
                            thread::spawn(move || do_copy(right, path_map, target_dir));
                        }
                        if self.dir2_save_path.is_some() {
                            let target_dir = self.dir2_save_path.as_ref().unwrap().clone();
                            let left = lefts.clone();
                            let path_map = left_path_map.clone();
                            // self.copy_thread = Some(thread::spawn(move || do_copy(left, path_map, target_dir)));
                            thread::spawn(move || do_copy(left, path_map, target_dir));
                        }
                    }
                }

                TableBuilder::new(ui)
                    .striped(true)
                    .column(Column::auto_with_initial_suggestion(500.0).resizable(true).at_least(200.0))
                    .column(Column::remainder())
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            select_all_checkbox(ui, lefts, &mut self.dir1_entries_selected, "Directory 1");
                            if !self.working && ui.button("Set destination for files missing from directory 1.").clicked() {

                                self.dir1_save_path = FileDialog::new()
                                    .set_title("Choose where files should be copied.")
                                    .pick_folder();
                            }
                            if self.dir1_save_path.is_some() {
                                ui.label(format!("Directory 2: {:?}", self.dir1_save_path.as_ref().unwrap()));
                            }
                            // ui.heading("Directory 1");
                        });
                        header.col(|ui| {
                            select_all_checkbox(ui, rights, &mut self.dir2_entries_selected, "Directory 2");
                            if !self.working && ui.button("Set destination for files missing from directory 2.").clicked() {

                                self.dir2_save_path = FileDialog::new()
                                    .set_title("Choose where files should be copied.")
                                    .pick_folder();
                            }
                            if self.dir2_save_path.is_some() {
                                ui.label(format!("Directory 2: {:?}", self.dir2_save_path.as_ref().unwrap()));
                            }
                            // ui.heading("Directory 2");
                        });
                    })
                    .body(|mut body| {
                        for i in 0..std::cmp::max(lefts.len(), rights.len()) {
                            body.row(30.0, |mut row| {
                                
                                row.col(|ui| {
                                    if i < lefts.len() {
                                        let (filename, should_transfer) = &mut lefts[i];
                                        ui.checkbox(should_transfer, filename.to_string_lossy()).handle_checkbox_change(&mut self.dir1_entries_selected);
                                    }
                                });
                                
                                
                                row.col(|ui| {
                                    if i < rights.len() {
                                        let (filename, should_transfer) = &mut rights[i];
                                        ui.checkbox(should_transfer, filename.to_string_lossy()).handle_checkbox_change(&mut self.dir2_entries_selected);
                                    }
                                });
                            });
                        }
                    });
            }
        });
    }
}

fn do_copy(to_copy: Vec<(OsString, bool)>, path_map: HashMap<OsString, PathBuf>, target_dir: PathBuf) -> Vec<Option<std::io::Error>> {
    to_copy.iter()
        .filter(| (_, should_copy) | *should_copy)
        .map(| (filename, _) | {
            let source_path = path_map.get(filename).expect("Error: Path map does not exhaustively contain all filenames. Ask Josh.");
            let target_path = Path::new(&target_dir).join(filename);
            // println!("{}, {}", source_path.display(), target_path.display());
            std::fs::copy(source_path, target_path)
        })
        // .map(| (filename, _) | path_map.get(filename).expect("Error: Path map does not exhaustively contain all filenames. Ask Josh.") )
        // .map(| source_path | {
        //     let mut target_path = PathBuf::from(&target_dir);
        //     target_path.push(&source_path);
        //     // if let Ok(suffix) = source_path.strip_prefix(&dpath) {
        //     // } else {}
        //     println!("{}, {}", source_path.display(), target_path.display());
        //     // std::fs::copy(source_path, target_path)
        // })
        .map(| result | result.err())
        .collect()
}

trait Handleable {
    fn handle_checkbox_change(&self, selected: &mut Selected);
}

impl Handleable for egui::Response {
    fn handle_checkbox_change(&self, selected: &mut Selected) {
        if self.changed() {
            selected.indeterminate = true;
            selected.bool = true;
        }
    }
}

fn select_all_checkbox(ui: &mut egui::Ui, all: &mut Vec<(OsString, bool)>, selected: &mut Selected, text: impl Into<egui::WidgetText>) {
    let checkbox = egui::Checkbox::new(&mut selected.bool, text).indeterminate(selected.indeterminate);
    if ui.add(checkbox).changed() {
        all.iter_mut().for_each(| (_, checked) | *checked = selected.bool);
        selected.indeterminate = false;
    }
}


async fn find_diffs(dir1: &OsString, dir2: &OsString) -> std::io::Result<(Vec<OsString>, Vec<OsString>, HashMap<OsString, PathBuf>, HashMap<OsString, PathBuf>)> {
    let mut names1: HashSet<(OsString, u64)> = HashSet::with_capacity(10000);
    let mut names2: HashSet<(OsString, u64)> = HashSet::with_capacity(10000);

    let mut left_path_map: HashMap<OsString, PathBuf> = HashMap::with_capacity(10000);
    let mut right_path_map: HashMap<OsString, PathBuf> = HashMap::with_capacity(10000);

    let fut1 = collect_filenames(dir1, &mut names1, &mut left_path_map);
    let fut2 = collect_filenames(dir2, &mut names2, &mut right_path_map);

    let _ = tokio::join!(fut1, fut2);

    let lefts: Vec<OsString> = names1.difference(&names2).map(|(nm, _)| nm.to_owned()).collect();
    let rights: Vec<OsString> = names2.difference(&names1).map(|(nm, _)| nm.to_owned()).collect();

    Ok((lefts, rights, left_path_map, right_path_map))
}

#[async_recursion(?Send)]
async fn collect_filenames<P: AsRef<Path>>(path: P, out: &mut HashSet<(OsString, u64)>, path_map: &mut HashMap<OsString, PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(path)? {
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
        out.insert((name.clone(), filesize));
        
        if path.is_dir() {
            collect_filenames(&path, out, path_map).await.ok();
        }
        path_map.insert(name, path);
    }
    Ok(())
}