mod info;
mod mods;
mod picker;
mod tasks;
mod visuals;
use crate::{core::Manager, logger::Entry, mods::Mod};
use anyhow::{Context, Result};
use eframe::{
    egui::{FontData, FontDefinitions},
    epaint::FontFamily,
    NativeOptions,
};
use egui::{
    self, style::Margin, text::LayoutJob, Align, Align2, Color32, ComboBox, FontId, Frame, Id,
    Label, Layout, RichText, Sense, TextFormat, Ui, Vec2,
};
use flume::{Receiver, Sender};
use font_loader::system_fonts::FontPropertyBuilder;
use im::Vector;
use join_str::jstr;
use std::{collections::VecDeque, path::PathBuf, sync::Arc, thread};
use uk_mod::unpack::ModReader;

use self::picker::FilePickerState;

// #[inline(always)]
// fn common_frame() -> Frame {
//     Frame {
//         stroke: Stroke::new(0.1, Color32::DARK_GRAY),
//         inner_margin: Margin::same(4.),
//         fill: visuals::panel(),
//         ..Default::default()
//     }
// }

fn load_fonts(context: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let font_to_try = if cfg!(windows) {
        "Segoe UI".to_owned()
    } else {
        std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "font-name"])
            .output()
            .and_then(|o| {
                String::from_utf8(o.stdout)
                    .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Bah"))
            })
            .unwrap_or_else(|_| "Ubuntu".to_owned())
    };
    if let Some(system_font) =
        font_loader::system_fonts::get(&FontPropertyBuilder::new().family(&font_to_try).build())
    {
        fonts
            .font_data
            .insert("System".to_owned(), FontData::from_owned(system_font.0));
    }
    if let Some(system_font) = font_loader::system_fonts::get(
        &FontPropertyBuilder::new()
            .family(&font_to_try)
            .bold()
            .build(),
    )
    .or_else(|| {
        let property = FontPropertyBuilder::new()
            .family(&(font_to_try + " Bold"))
            .build();
        font_loader::system_fonts::get(&property)
    }) {
        fonts
            .font_data
            .insert("Bold".to_owned(), FontData::from_owned(system_font.0));
    }
    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, "System".to_owned());
    fonts
        .families
        .insert(FontFamily::Name("Bold".into()), vec!["Bold".to_owned()]);
    context.set_fonts(fonts);
}

pub enum Message {
    Log(Entry),
    CloseError,
    SelectOnly(usize),
    SelectAlso(usize),
    Deselect(usize),
    ClearSelect,
    StartDrag(usize),
    ClearDrag,
    MoveSelected(usize),
    FilePickerUp,
    FilePickerBack,
    FilePickerSet(Option<PathBuf>),
    ChangeProfile(String),
    SetFocus(FocusedPane),
    OpenMod(PathBuf),
    HandleMod(Mod),
    RequestOptions(Mod),
    InstallMod(Mod),
    AddMod(Mod),
    Error(anyhow::Error),
}

enum Tabs {
    Info,
    Install,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FocusedPane {
    ModList,
    FilePicker,
    None,
}

struct App {
    core: Arc<Manager>,
    channel: (Sender<Message>, Receiver<Message>),
    mods: Vector<Mod>,
    selected: Vector<Mod>,
    drag_index: Option<usize>,
    hover_index: Option<usize>,
    picker_state: FilePickerState,
    tab: Tabs,
    focused: FocusedPane,
    logs: VecDeque<Entry>,
    error: Option<anyhow::Error>,
    busy: bool,
    dirty: bool,
}

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
        load_fonts(&cc.egui_ctx);
        cc.egui_ctx.set_pixels_per_point(1.);
        let core = Arc::new(Manager::init().unwrap());
        let mods: Vector<_> = core.mod_manager().all_mods().map(|m| m.clone()).collect();
        let (send, recv) = flume::unbounded();
        crate::logger::LOGGER.set_sender(send.clone());
        crate::logger::LOGGER.flush_queue();
        log::info!("Logger initialized");
        Self {
            channel: (send, recv),
            selected: mods.front().cloned().into_iter().collect(),
            drag_index: None,
            hover_index: None,
            picker_state: Default::default(),
            mods,
            core,
            logs: VecDeque::new(),
            tab: Tabs::Info,
            focused: FocusedPane::None,
            error: None,
            busy: false,
            dirty: false,
        }
    }

    #[inline(always)]
    fn modal_open(&self) -> bool {
        self.error.is_some() || self.busy
    }

    fn do_update(&self, message: Message) {
        self.channel.0.send(message).unwrap();
    }

    fn do_task(
        &mut self,
        task: impl 'static + Send + Sync + FnOnce(Arc<Manager>) -> Result<Message>,
    ) {
        let sender = self.channel.0.clone();
        let core = self.core.clone();
        let task = Box::new(task);
        self.busy = true;
        thread::spawn(move || {
            sender
                .send(match task(core) {
                    Ok(msg) => msg,
                    Err(e) => Message::Error(e),
                })
                .unwrap();
        });
    }

    fn handle_update(&mut self, ctx: &eframe::egui::Context) {
        if let Ok(msg) = self.channel.1.try_recv() {
            match msg {
                Message::Log(entry) => {
                    self.logs.push_back(entry);
                }
                Message::CloseError => self.error = None,
                Message::SelectOnly(i) => {
                    let index = i.clamp(0, self.mods.len() - 1);
                    let mod_ = &self.mods[index];
                    if self.selected.contains(mod_) {
                        self.selected.retain(|m| m == mod_);
                    } else {
                        self.selected.clear();
                        self.selected.push_back(self.mods[index].clone());
                    }
                    self.drag_index = None;
                }
                Message::SelectAlso(i) => {
                    let index = i.clamp(0, self.mods.len() - 1);
                    let mod_ = &self.mods[index];
                    if !self.selected.contains(mod_) {
                        self.selected.push_back(mod_.clone());
                    }
                    self.drag_index = None;
                }
                Message::Deselect(i) => {
                    let index = i.clamp(0, self.mods.len() - 1);
                    let mod_ = &self.mods[index];
                    self.selected.retain(|m| m != mod_);
                    self.drag_index = None;
                }
                Message::ClearSelect => {
                    self.selected.clear();
                    self.drag_index = None;
                }
                Message::StartDrag(i) => {
                    if ctx.input().pointer.any_released() {
                        self.drag_index = None;
                    }
                    self.drag_index = Some(i);
                    let mod_ = &self.mods[i];
                    if !self.selected.contains(mod_) {
                        if !ctx.input().modifiers.ctrl {
                            self.selected.clear();
                        }
                        self.selected.push_back(mod_.clone());
                    }
                }
                Message::ClearDrag => {
                    self.drag_index = None;
                }
                Message::MoveSelected(dest_index) => {
                    if self.selected.len() == self.mods.len() {
                        return;
                    }
                    self.mods.retain(|m| !self.selected.contains(m));
                    for (i, selected_mod) in self.selected.iter().enumerate() {
                        self.mods.insert(dest_index + i, selected_mod.clone());
                    }
                    self.hover_index = None;
                    self.drag_index = None;
                }
                Message::FilePickerUp => {
                    let has_parent = self.picker_state.path.parent().is_some();
                    if has_parent {
                        self.picker_state
                            .history
                            .push(self.picker_state.path.clone());
                        self.picker_state
                            .set_path(self.picker_state.path.parent().unwrap().to_path_buf());
                    }
                }
                Message::FilePickerBack => {
                    if let Some(prev) = self.picker_state.history.pop() {
                        self.picker_state.set_path(prev);
                    }
                }
                Message::FilePickerSet(path) => {
                    let path = match path {
                        Some(path) => path,
                        None => self.picker_state.path_input.as_str().into(),
                    };
                    if path.is_dir() {
                        self.picker_state.selected = None;
                        self.picker_state
                            .history
                            .push(self.picker_state.path.clone());
                        self.picker_state.set_path(path);
                    }
                }
                Message::ChangeProfile(profile) => {
                    todo!("Change profile");
                }
                Message::SetFocus(pane) => {
                    self.focused = pane;
                }
                Message::OpenMod(path) => {
                    self.do_task(move |_| tasks::open_mod(&path));
                }
                Message::HandleMod(mod_) => {
                    self.busy = false;
                    log::debug!("{:?}", &mod_);
                    if mod_.meta.options.len() > 0 {
                        self.do_update(Message::RequestOptions(mod_));
                    } else {
                        self.do_update(Message::InstallMod(mod_));
                    }
                }
                Message::InstallMod(mod_) => {
                    self.do_task(move |core| {
                        let mods = core.mod_manager();
                        let mod_ = mods.add(&mod_.path)?.clone();
                        mods.save()?;
                        log::info!("Added mod {} to current profile", mod_.meta.name.as_str());
                        Ok(Message::AddMod(mod_))
                    });
                }
                Message::AddMod(mod_) => {
                    self.busy = false;
                    self.mods.push_back(mod_);
                }
                Message::RequestOptions(mod_) => {
                    todo!("Request options");
                }
                Message::Error(error) => {
                    log::error!("{:?}", &error);
                    self.busy = false;
                    self.error = Some(error);
                    ctx.request_repaint();
                }
            }
            ctx.request_repaint();
        }
    }

    fn render_menu(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            ui.set_enabled(!self.modal_open());
            ui.horizontal(|ui| {
                ui.menu_button("File", Self::file_menu);
                ui.menu_button("Edit", Self::edit_menu);
            });
        });
    }

    fn render_error(&mut self, ctx: &egui::Context) {
        if let Some(err) = self.error.as_ref() {
            egui::Window::new("Error")
                .collapsible(false)
                .anchor(Align2::CENTER_CENTER, Vec2::default())
                .auto_sized()
                .frame(Frame::window(&ctx.style()).inner_margin(Margin {
                    top: 0.,
                    left: 8.,
                    right: 8.,
                    bottom: 8.,
                }))
                .show(ctx, |ui| {
                    ui.add_space(8.);
                    ui.label(err.to_string());
                    ui.add_space(8.);
                    egui::CollapsingHeader::new("Details").show(ui, |ui| {
                        err.chain().for_each(|e| {
                            ui.label(RichText::new(e.to_string()).code());
                        });
                    });
                    ui.add_space(8.);
                    let width = ui.min_size().x;
                    ui.horizontal(|ui| {
                        ui.allocate_ui_with_layout(
                            Vec2::new(width, ui.min_size().y),
                            Layout::right_to_left(Align::Center),
                            |ui| {
                                if ui.button("OK").clicked() {
                                    self.do_update(Message::CloseError);
                                }
                                if ui.button("Copy").clicked() {
                                    ui.output().copied_text = err
                                        .chain()
                                        .map(|e| e.to_string())
                                        .collect::<Vec<_>>()
                                        .join("\n");
                                    egui::popup::show_tooltip(ctx, Id::new("copied"), |ui| {
                                        ui.label("Copied")
                                    });
                                }
                                ui.shrink_width_to_current();
                            },
                        );
                    });
                });
        }
    }

    fn render_busy(&self, ctx: &egui::Context) {
        if self.busy {
            egui::Window::new("Working")
                .fixed_size(ctx.used_size() / 2.)
                .anchor(Align2::CENTER_CENTER, Vec2::default())
                .collapsible(false)
                .frame(Frame::window(&ctx.style()).inner_margin(8.))
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.label("Processing…");
                        ui.label(
                            self.logs
                                .back()
                                .map(|l| l.args.as_str())
                                .unwrap_or_default(),
                        );
                    });
                });
        }
    }

    fn file_menu(ui: &mut Ui) {
        if ui.button("Open mod…").clicked() {
            // todo!("Open mod");
        }
    }

    fn edit_menu(ui: &mut Ui) {
        if ui.button("Settings").clicked() {
            todo!("Settings");
        }
    }

    fn render_profile_menu(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let current_profile = self
                .core
                .settings()
                .platform_config()
                .map(|c| c.profile.to_string())
                .unwrap_or_else(|| "Default".to_owned());
            ComboBox::from_id_source("profiles")
                .selected_text(&current_profile)
                .show_ui(ui, |ui| {
                    self.core.settings().profiles().for_each(|profile| {
                        if ui
                            .selectable_label(profile.as_str() == current_profile, profile.as_str())
                            .clicked()
                        {
                            self.do_update(Message::ChangeProfile(profile.into()));
                        }
                    });
                })
                .response
                .on_hover_text("Select Mod Profile");
            ui.button("🗑").on_hover_text("Delete Profile");
            ui.button("✚").on_hover_text("New Profile");
            ui.button("☰").on_hover_text("Manage Profiles…");
            ui.label(format!(
                "{} Mods / {} Active",
                self.mods.len(),
                self.mods.iter().filter(|m| m.enabled).count()
            ));
        });
    }

    fn render_log(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("log")
            .resizable(true)
            .show(ctx, |ui| {
                ui.set_enabled(!self.modal_open());
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        let mut job = LayoutJob::default();
                        self.logs.iter().for_each(|entry| {
                            job.append(
                                &jstr!("[{&entry.timestamp}] "),
                                0.,
                                TextFormat {
                                    color: Color32::GRAY,
                                    font_id: FontId::monospace(10.),
                                    ..Default::default()
                                },
                            );
                            job.append(
                                &jstr!("{&entry.level} "),
                                0.,
                                TextFormat {
                                    color: match entry.level.as_str() {
                                        "INFO" => visuals::GREEN,
                                        "WARN" => visuals::ORGANGE,
                                        "ERROR" => visuals::RED,
                                        "DEBUG" => visuals::BLUE,
                                        _ => visuals::YELLOW,
                                    },
                                    font_id: FontId::monospace(10.),
                                    ..Default::default()
                                },
                            );
                            job.append(
                                &entry.args,
                                1.,
                                TextFormat {
                                    color: Color32::WHITE,
                                    font_id: FontId::monospace(10.),
                                    ..Default::default()
                                },
                            );
                            job.append("\n", 0.0, Default::default());
                        });
                        let text = job.text.clone();
                        if ui
                            .add(Label::new(job).sense(Sense::click()))
                            .on_hover_text("Click to copy")
                            .clicked()
                        {
                            ui.output().copied_text = text;
                        }
                    });
            });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        self.handle_update(ctx);
        self.render_error(ctx);
        self.render_menu(ctx);
        egui::SidePanel::right("right_panel")
            .resizable(true)
            .max_width(ctx.used_size().x / 3.)
            .min_width(0.)
            .show(ctx, |ui| {
                ui.set_enabled(!self.modal_open());
                egui::ScrollArea::vertical()
                    .id_source("right_panel_scroll")
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui
                                .selectable_label(matches!(self.tab, Tabs::Info), "Mod Info")
                                .clicked()
                            {
                                self.tab = Tabs::Info;
                            }
                            if ui
                                .selectable_label(matches!(self.tab, Tabs::Install), "Install")
                                .clicked()
                            {
                                self.tab = Tabs::Install;
                            }
                        });
                        match self.tab {
                            Tabs::Info => {
                                if let Some(mod_) = self.selected.front() {
                                    info::render_mod_info(mod_, ui);
                                } else {
                                    ui.centered_and_justified(|ui| {
                                        ui.label("No mod selected");
                                    });
                                }
                            }
                            Tabs::Install => {
                                self.render_file_picker(ui);
                            }
                        }
                    });
                ui.allocate_space(ui.available_size());
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_enabled(!self.modal_open());
            self.render_profile_menu(ui);
            egui::ScrollArea::both()
                .id_source("mod_list")
                .show(ui, |ui| {
                    self.render_modlist(ui);
                });
        });
        self.render_busy(ctx);
        self.render_log(ctx);
    }
}

pub fn main() {
    crate::logger::init();
    log::debug!("Logger initialized");
    log::info!("Started ukmm");
    eframe::run_native(
        "U-King Mod Manager",
        NativeOptions::default(),
        Box::new(|cc| Box::new(App::new(cc))),
    );
}
