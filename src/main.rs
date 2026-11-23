#![windows_subsystem = "windows"]

use std::sync::Arc;
use std::time::Duration;

use eframe::egui::{self, Button, Color32, TextEdit};
use tokio::sync::Mutex;

mod privileges;
mod process;
mod task_scheduler;

const TASK_NAME: &str = "NV_SSD_Guard_AutoRun";

enum Status {
    Unknown,
    Unproteced,
    Protected,
}

struct AppState {
    proc_pid: u32,
    status: Status,
    status_color: Color32,
    lock: bool,
    log: String,
}

impl AppState {
    fn set_status_unknown(&mut self) {
        self.status = Status::Unknown;
        self.status_color = Color32::WHITE;
    }
    fn set_status_unprotected(&mut self) {
        self.status = Status::Unproteced;
        self.status_color = Color32::DARK_RED;
    }
    fn set_status_protected(&mut self) {
        self.status = Status::Protected;
        self.status_color = Color32::DARK_GREEN;
    }
    fn append_log(&mut self, entry: &str) {
        self.log.push_str(entry);
    }
}

struct NvSsdGuard {
    proc_name: &'static str,
    state: Arc<Mutex<AppState>>,
    rt: tokio::runtime::Runtime,
}

impl Default for NvSsdGuard {
    fn default() -> Self {
        Self {
            proc_name: "NVDisplay.Container.exe",
            state: Arc::new(Mutex::new(AppState {
                proc_pid: 0,
                status: Status::Unknown,
                status_color: Color32::WHITE,
                lock: false,
                log: "log initialized\n".to_string(),
            })),
            rt: tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap(),
        }
    }
}

async fn start_guard(name: &'static str, state: Arc<Mutex<AppState>>) {
    // kill existing processes (blocking Win32 call off the UI thread)
    let _killed = tokio::task::spawn_blocking(|| process::kill_processes_by_name(name))
        .await
        .unwrap();

    let mut s = state.lock().await;
    s.append_log(format!("killed {} process\n", name).as_str());
    s.append_log("waiting 30s for process to relaunch\n");
    drop(s);

    tokio::time::sleep(Duration::from_secs(30)).await;

    // Find PID
    let pid = process::find_process_id_by_name(name);
    let mut s = state.lock().await;
    s.append_log("finding PID\n");
    if pid == 0 {
        s.append_log(&format!("no {} PID found\n", name));
        s.set_status_unknown();
        s.lock = false;
        return;
    }
    s.proc_pid = pid;
    s.append_log(&format!("found {} PID {}\n", name, pid));
    drop(s);

    // Suspend PID
    let (count, failed) = process::suspend_process(pid);
    let mut s = state.lock().await;
    if failed {
        s.append_log("failed to suspend process\n");
        s.set_status_unknown();
        s.lock = false;
    } else {
        s.append_log(&format!("suspended {} threads\n", count));
        s.set_status_protected();
        s.append_log("finished starting guard\n");
        s.lock = false;
    }
}

async fn stop_guard(name: &'static str, state: Arc<Mutex<AppState>>) {
    // Find PID
    let pid = process::find_process_id_by_name(name);
    let mut s = state.lock().await;
    s.append_log("finding PID\n");
    if pid == 0 {
        s.append_log(&format!("no {} PID found\n", name));
        s.set_status_unknown();
        s.lock = false;
        return;
    }
    s.proc_pid = pid;
    s.append_log(&format!("found {} PID {}\n", name, pid));
    drop(s);

    // Resume PID
    let (count, failed) = process::resume_process(pid);
    let mut s = state.lock().await;
    if failed {
        s.append_log("failed to resume process\n");
        s.set_status_unknown();
        s.lock = false;
    } else {
        s.append_log(&format!("resumed {} threads\n", count));
        s.set_status_unprotected();
        s.append_log("finished stopping guard\n");
    }
    s.lock = false;
}

impl eframe::App for NvSsdGuard {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                ui.set_width(frame.winit_window().unwrap().inner_size().width as f32 - 24.0);
                ui.set_height(frame.winit_window().unwrap().inner_size().height as f32 - 24.0);

                // left panel
                egui::Panel::left("actions").show(ui, |ui| {
                    ui.vertical_centered_justified(|ui| {
                        ui.add_space(12.0);

                        // START
                        let start_enabled = !self.state.blocking_lock().lock;
                        if ui
                            .add_enabled(start_enabled, Button::new("Start Guard"))
                            .clicked()
                        {
                            let mut s = self.state.blocking_lock();
                            s.lock = true;
                            s.append_log("starting guard\n");
                            drop(s);

                            let state = Arc::clone(&self.state);
                            self.rt.spawn(start_guard(self.proc_name, state));
                        }

                        ui.add_space(4.0);

                        // STOP
                        let stop_enabled = !self.state.blocking_lock().lock;
                        if ui
                            .add_enabled(stop_enabled, Button::new("Stop Guard"))
                            .clicked()
                        {
                            let mut s = self.state.blocking_lock();
                            s.lock = true;
                            s.append_log("stopping guard\n");
                            drop(s);

                            let state = Arc::clone(&self.state);
                            self.rt.spawn(stop_guard(self.proc_name, state));
                        }
                        ui.add_space(4.0);

                        if ui.button("Run on Login").clicked() {
                            let (created, err) = task_scheduler::create_scheduled_task(TASK_NAME);
                            let mut s = self.state.blocking_lock();
                            if created {
                                s.append_log("run on login task created\n");
                            } else {
                                s.append_log(&format!("{}\n", err));
                            }
                        }
                        ui.add_space(4.0);

                        if ui.button("Remove Run on Login").clicked() {
                            let (deleted, err) = task_scheduler::delete_scheduled_task(TASK_NAME);
                            let mut s = self.state.blocking_lock();
                            if deleted {
                                s.append_log("run on login task removed\n");
                            } else {
                                s.append_log(&format!("{}\n", err));
                            }
                        }
                        ui.add_space(240.0);

                        ui.label("Created by: Dave Rich");
                        ui.label("github.com/davidarich");
                    });
                });

                // center panel
                egui::CentralPanel::default_margins().show(ui, |ui| {
                    let mut s = self.state.blocking_lock();

                    match &s.status {
                        Status::Unknown => {
                            ui.label(
                                egui::RichText::new("Status: Unknown")
                                    .color(s.status_color)
                                    .size(18.0),
                            );
                        }
                        Status::Protected => {
                            ui.label(
                                egui::RichText::new("Status: Protected")
                                    .color(s.status_color)
                                    .size(18.0),
                            );
                        }
                        Status::Unproteced => {
                            ui.label(
                                egui::RichText::new("Status: Unprotected")
                                    .color(s.status_color)
                                    .size(16.0),
                            );
                        }
                    }

                    ui.add_space(4.0);

                    ui.label(egui::RichText::new(format!("Process ID: {}", s.proc_pid)).size(16.0));

                    // Log
                    ui.label(egui::RichText::new("Log:").size(16.0));
                    ui.add_space(4.0);
                    ui.with_layout(
                        egui::Layout {
                            cross_align: egui::Align::LEFT,
                            cross_justify: true,
                            main_dir: egui::Direction::TopDown,
                            main_wrap: false,
                            main_align: egui::Align::TOP,
                            main_justify: true,
                        },
                        |ui| {
                            egui::ScrollArea::vertical()
                                .max_height(320.0)
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    let text_edit = TextEdit::multiline(&mut s.log)
                                        .clip_text(true)
                                        .desired_width(ui.available_width())
                                        .desired_rows(10)
                                        .interactive(false);
                                    ui.add(text_edit);
                                });
                        },
                    );
                });
            });
        });
    }
}

fn main() -> eframe::Result {
    // acquire SeDebugPrivilege to allow suspension of NVDisplay.Container
    privileges::acquire_debug_privilege();

    let options = eframe::NativeOptions {
        viewport: {
            let mut vb = egui::ViewportBuilder::default();
            vb = vb.with_inner_size([600.0, 440.0]);
            vb
        },
        ..Default::default()
    };

    eframe::run_native(
        "NV SSD Guard",
        options,
        Box::new(|_cc| Ok(Box::<NvSsdGuard>::default())),
    )
}
