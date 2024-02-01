#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] use std::path::PathBuf;
// hide console window on Windows in release
use std::{ffi::OsString, path::Path};
use std::collections::HashSet;
use async_recursion::async_recursion;
use eframe::egui::{self, Response};
use rfd::FileDialog;
use std::sync::mpsc;
use std::thread;


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
    working: bool,
    result: Option<(Vec<OsString>, Vec<OsString>)>,
    sender: mpsc::Sender<std::io::Result<(Vec<OsString>, Vec<OsString>)>>,
    receiver: mpsc::Receiver<std::io::Result<(Vec<OsString>, Vec<OsString>)>>,
    work_thread: Option<std::thread::JoinHandle<()>>
}

impl Default for MyApp {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();

        Self {
            dir1Path: None,
            dir2Path: None,
            working: false,
            result: None,
            sender: sender,
            receiver: receiver,
            work_thread: None
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Directory Comparer!");
            if !self.working && ui.button("+ Directory").clicked() {
                self.dir1Path = FileDialog::new()
                    .set_title("Choose a Directory to Compare.")
                    .pick_folder();
            }
            if self.dir1Path.is_some() {
                ui.label(format!("Directory 1: {:?}", self.dir1Path.as_mut().unwrap().as_mut_os_string()));
            }
            if !self.working && ui.button("+ Directory").clicked() {
                self.dir2Path = FileDialog::new()
                    .set_title("Choose a Directory to Compare.")
                    .pick_folder();
            }
            if self.dir2Path.is_some() {
                ui.label(format!("Directory 2: {:?}", self.dir2Path.as_mut().unwrap().as_mut_os_string()));
            }

            if self.dir1Path.is_some() && self.dir2Path.is_some() {
                if !self.working && ui.button("Find Unique Files").clicked() {

                    self.result = None;
                    self.working = true;

                    let sndr = self.sender.to_owned();

                    let dir1 = self.dir1Path.as_mut().unwrap().as_mut_os_string().to_owned();
                    let dir2 = self.dir2Path.as_mut().unwrap().as_mut_os_string().to_owned();

                    self.work_thread = Some(thread::spawn(move || {
                        let result: Result<(Vec<OsString>, Vec<OsString>), std::io::Error> = tokio::runtime::Builder::new_multi_thread()
                            .enable_all()
                            .build()
                            .unwrap()
                            .block_on(find_diffs(&dir1, &dir2));
                        sndr.send(result);
                    }));

                }
            }

            let res: Result<(), String> = match self.receiver.try_recv() {
                Err(mpsc::TryRecvError::Empty) => Err("".to_string()),
                Err(mpsc::TryRecvError::Disconnected) => Err("Error: Work thread disconnected. Probably crashed. Ask Josh.".to_string()),
                Ok(r) => match r {
                    Err(e) => Err(e.to_string()),
                    Ok(r2) => Ok({
                        self.result = Some(r2);
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
                let (lefts, rights) = self.result.as_ref().unwrap();

                ui.columns(2, |uis| {
                    for i in 0..2 {
                        let ui = &mut uis[i];
                        
                        if i == 0 {
                            for s in lefts.iter() {
                                ui.label(format!("{:?}",s));
                            };
                        } else {
                            for s in rights.iter() {
                                ui.label(format!("{:?}",s));
                            };
                        }
                        
                    }
                });
            }


        });
    }
}


async fn find_diffs(dir1: &OsString, dir2: &OsString) -> std::io::Result<(Vec<OsString>, Vec<OsString>)> {
    let mut names1: HashSet<OsString> = HashSet::with_capacity(10000);
    let mut names2: HashSet<OsString> = HashSet::with_capacity(10000);

    let fut1 = collect_filenames(dir1, &mut names1);
    let fut2 = collect_filenames(dir2, &mut names2);

    let _ = tokio::join!(fut1, fut2);
    
    let diff: HashSet<_> = names1.symmetric_difference(&names2).collect();

    let lefts: Vec<OsString> = names1.difference(&names2).map(|nm| nm.to_owned()).collect();
    let rights: Vec<OsString> = names2.difference(&names1).map(|nm| nm.to_owned()).collect();

    Ok((lefts, rights))
}

#[async_recursion(?Send)]
async fn collect_filenames<P: AsRef<Path>>(path: P, out: &mut HashSet<OsString>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(path)? {
        let entry = match entry {
            Err(_) => continue,
            Ok(e) => e
        };
        let path = entry.path();
        let name = entry.file_name();
        out.insert(name);
        if path.is_dir() {
            collect_filenames(path, out).await.ok();
        }
    }
    Ok(())
}