use std::time::Duration;

use eframe::egui::{self, Button, Color32, TextEdit};
use std::thread::sleep;

mod process;

enum Status {
    Unknown,
    Unproteced,
    Protected,
}

struct NvSsdGuard {
    proc_name: &'static str,
    proc_pid: u32,
    status: Status,
    status_color: Color32,
    lock: bool,
    log: String,
}

impl Default for NvSsdGuard {
    fn default() -> Self {
        Self {
            proc_name: "NVDisplay.Container.exe",
            proc_pid: 0,
            status: Status::Unknown,
            status_color: Color32::WHITE,
            lock: false,
            log: "log initialized\n".to_string(),
        }
    }
}

impl NvSsdGuard {
    fn set_status_unknown(&mut self) {
        self.status = Status::Protected;
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
                        if ui
                            .add_enabled(!self.lock, Button::new("Start Guard"))
                            .clicked()
                        {
                            self.lock = true;
                            self.append_log("starting guard\n");

                            // killing existing processses
                            let _killed = process::kill_processes_by_name(self.proc_name);

                            self.append_log("waiting for process to relaunch\n");
                            sleep(Duration::from_secs(10));

                            // Find PID
                            self.append_log("finding PID\n");
                            let pid = process::find_process_id_by_name(self.proc_name);
                            match pid {
                                0 => {
                                    self.append_log(
                                        format!("no {} PID found\n", self.proc_name).as_str(),
                                    );
                                    self.set_status_unknown();
                                    self.lock = false;
                                    return;
                                }
                                _ => {
                                    self.proc_pid = pid;
                                    self.set_status_protected();
                                    self.append_log(
                                        format!("found {} PID {}\n", self.proc_name, self.proc_pid)
                                            .as_str(),
                                    );
                                }
                            }

                            // Suspend PID
                            let (count, failed) = process::suspend_process(pid);
                            if failed {
                                print!("failed to suspend process\n");
                                self.set_status_unknown();
                                self.lock = false;
                                return;
                            }
                            self.append_log(format!("suspended {} threads\n", count).as_str());
                            self.append_log("finished starting guard\n");
                            self.lock = false;
                        }

                        ui.add_space(4.0);

                        // STOP
                        if ui
                            .add_enabled(!self.lock, Button::new("Stop Guard"))
                            .clicked()
                        {
                            self.lock = true;

                            // Find PID
                            self.append_log("stopping guard\n");
                            self.append_log("finding PID\n");
                            let pid = process::find_process_id_by_name(self.proc_name);
                            match pid {
                                0 => {
                                    self.append_log(
                                        format!("no {} PID found\n", self.proc_name).as_str(),
                                    );
                                    self.set_status_unknown();
                                    self.lock = false;
                                    return;
                                }
                                _ => {
                                    self.proc_pid = pid;
                                    println!("found {} PID {}\n", self.proc_name, self.proc_pid);
                                }
                            }

                            let (count, failed) = process::resume_process(self.proc_pid);
                            if failed {
                                self.append_log("failed to resume process\n");
                                self.set_status_unknown();
                                self.lock = false;
                                return;
                            }
                            self.append_log(format!("resumed {} threads\n", count).as_str());
                            self.set_status_unprotected();
                            self.append_log("finished stopping guard\n");
                            self.lock = false;
                        }

                        ui.add_space(300.0);

                        ui.label("Created by: Dave Rich");
                        ui.label("github.com/davidarich");
                    });
                });

                // center panel
                egui::CentralPanel::default_margins().show(ui, |ui| {
                    match &self.status {
                        Status::Unknown => {
                            ui.label(
                                egui::RichText::new("Status: Unknown")
                                    .color(self.status_color)
                                    .size(18.0),
                            );
                        }
                        Status::Protected => {
                            ui.label(
                                egui::RichText::new("Status: Protected")
                                    .color(self.status_color)
                                    .size(18.0),
                            );
                        }
                        Status::Unproteced => {
                            ui.label(
                                egui::RichText::new("Status: Unprotected")
                                    .color(self.status_color)
                                    .size(16.0),
                            );
                        }
                    }

                    ui.add_space(4.0);

                    ui.label(
                        egui::RichText::new(format!("Process ID: {}", self.proc_pid)).size(16.0),
                    );

                    // Log
                    ui.label(egui::RichText::new("Log:").size(16.0));
                    ui.add_space(4.0);
                    ui.add(TextEdit::multiline(&mut self.log));
                });
            });
        });
    }
}

fn main() -> eframe::Result {
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
