use rfd::*;

use eframe::*;
use eframe::Frame as EFrame;
use eframe::egui::*;

use serde_json::{to_string_pretty, from_str};

use std::array::from_fn;
use std::fs::{write, read_to_string};

use crate::{Config, HitPoint, SharedState};

const FILE_EXT: &str = "rbb";

pub fn start(state: SharedState) {
    let viewport = ViewportBuilder::default()
        .with_inner_size([440.0, 500.0])
        .with_always_on_top();

    let options = NativeOptions {
        viewport: viewport,
        ..Default::default()
    };

    let creator = Box::new(|_: &CreationContext<'_>| {
        Ok(Box::new(Gui {
            state: state,
            linked: [true; 4],
            lane: None,
        }) as Box<dyn App>)
    });

    run_native("robeats bot", options, creator)
        .unwrap();
}

struct Gui {
    state: SharedState,
    linked: [bool; 4],
    lane: Option<usize>,
}

impl Gui {
    fn on_set_keybind(&mut self, context: &Context) {
        let Some(lane) = self.lane else {
            return
        };

        let mut selected = None;

        context.input(|state| {
            for event in &state.events {
                if selected.is_some() {
                    break;
                }

                match event {
                    Event::Key { key, pressed: true, .. } => {
                        if *key == Key::Escape {
                            selected = Some(None);
                        } else if let Some(code) = get_egui_code(*key) {
                            selected = Some(Some(code));
                        }
                    },

                    Event::Text(text) => {
                        if let Some(code) = text.chars().next().and_then(get_char_code) {
                            selected = Some(Some(code));
                        }
                    },

                    _ => {}
                }
            }
        });

        match selected {
            Some(Some(code)) => {
                self.state
                    .lock()
                    .unwrap()
                    .keys[lane] = code;

                self.lane = None;
            }

            Some(None) => {
                self.lane = None
            },

            None => {}
        }
    }

    fn on_export_config(&self) {
        let config = Config::get(&self.state.lock().unwrap());

        let path = FileDialog::new()
            .add_filter(FILE_EXT.to_uppercase(), &[FILE_EXT])
            .set_file_name(format!("config.{}", FILE_EXT))
            .save_file();

        let Some(path) = path else {
            return;
        };

        let Ok(json) = to_string_pretty(&config) else {
            return
        };

        write(path, json)
            .unwrap();
    }

    fn on_import_config(&self) {
        let path = FileDialog::new()
            .add_filter(FILE_EXT.to_uppercase(), &[FILE_EXT])
            .pick_file();

        let Some(path) = path else {
            return;
        };

        let Ok(data) = read_to_string(&path) else {
            return
        };

        let Ok(config) = from_str::<Config>(&data) else {
            return
        };

        let state = &mut self.state
            .lock()
            .unwrap();

        config.set(state);
    }
}

impl App for Gui {
    fn update(&mut self, context: &Context, _: &mut EFrame) {
        context.request_repaint();
        self.on_set_keybind(context);

        let mut do_export = false;
        let mut do_import = false;

        CentralPanel::default().show(context, |ui| {
            let mut state = self.state
                .lock()
                .unwrap();

            ui.horizontal(|ui| {
                ui.checkbox(&mut state.enabled, RichText::new("enabled").strong());
                ui.checkbox(&mut state.show_hit_points, "show hit points");
                ui.checkbox(&mut state.hold_notes, "hold long notes");

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if state.roblox_check && state.on_roblox {
                        ui.colored_label(Color32::from_rgb(0, 255, 0), "\u{25CF}");
                    } else if state.roblox_check {
                        ui.colored_label(Color32::from_rgb(255, 0, 0), "\u{25CF}");
                    }

                    ui.checkbox(&mut state.roblox_check, "roblox check");
                });
            });

            ui.separator();

            let mut threshold = state.threshold as f32;
            ui.add(Slider::new(&mut threshold, 0.0..= 255.0)
                .text("threshold")
            );

            let mut poll = state.scan_interval as f32;
            ui.add(Slider::new(&mut poll, 0.0..= 50.0)
                .text("scan interval")
                .suffix(" ms")
            );

            let mut delay = state.click_delay as f32;
            ui.add(Slider::new(&mut delay, 0.0..= 200.0)
                .text("click delay")
                .suffix(" ms")
            );

            state.threshold = threshold as u8;
            state.scan_interval = poll as u32;
            state.click_delay = delay as u32;

            ui.separator();

            let old  = from_fn::<_, 4, _>(|i| state.lanes[i].clone());
            let mut remove = None;
            let mut add = None;

            ScrollArea::vertical()
                .max_height(ui.available_height() - 44.0)
                .show(ui, |ui| {
                    for lane in 0..4 {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.linked[lane], "");
                            ui.strong(format!("lane {}", lane + 1));

                            let key_name = if self.lane == Some(lane) {
                                "...".to_string()
                            } else {
                                get_code_name(state.keys[lane])
                            };

                            if ui.button(&key_name).clicked() && self.lane.is_none() {
                                self.lane = Some(lane);
                            }

                            if ui.small_button("+").clicked() {
                                add = Some(lane);
                            }
                        });

                        for point in 0..state.lanes[lane].len() {
                            ui.horizontal(|ui| {
                                ui.add_space(28.0);
                                ui.label(format!("{}.", point + 1));
                                ui.add(DragValue::new(&mut state.lanes[lane][point].x).prefix("x: ").speed(1));
                                ui.add(DragValue::new(&mut state.lanes[lane][point].y).prefix("y: ").speed(1));
                                ui.color_edit_button_srgb(&mut state.lanes[lane][point].color);

                                if state.lanes[lane].len() > 1 && ui.small_button("\u{00D7}").clicked() {
                                    remove = Some((lane, point));
                                }
                            });
                        }

                        ui.add_space(2.0);
                    }
                });

            if let Some(lane) = add {
                let last = state.lanes[lane]
                    .last()
                    .unwrap()
                    .clone();

                state.lanes[lane].push(HitPoint {
                    x: last.x,
                    y: last.y - 30,
                    color: last.color,
                });
            }

            if let Some((lane, pt)) = remove {
                state.lanes[lane].remove(pt);
            }

            let lanes = &mut state.lanes;
            let linked = &self.linked;

            for i in 0..4 {
                for i2 in 0..lanes[i].len().min(old[i].len()) {
                    let x = lanes[i][i2].x - old[i][i2].x;
                    let y = lanes[i][i2].y - old[i][i2].y;
                    if (x == 0 && y == 0) || !linked[i] {
                        continue;
                    }

                    for i3 in 0..4 {
                        if !linked[i3] {
                            continue;
                        }

                        for i4 in 0..lanes[i3].len() {
                            if i3 == i && i4 == i2 {
                                continue;
                            }

                            lanes[i3][i4].x += x;
                            lanes[i3][i4].y += y;
                        }
                    }

                    return;
                }
            }

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("save config").clicked() {
                    do_export = true;
                }

                if ui.button("load config").clicked() {
                    do_import = true;
                }
            });
        });

        if do_export {
            self.on_export_config();
        }

        if do_import {
            self.on_import_config();
        }
    }
}

fn get_egui_code(key: Key) -> Option<u16> {
    use egui::Key::*;

    Some(match key {
        A => 0x41,
        B => 0x42,
        C => 0x43,
        D => 0x44,
        E => 0x45,
        F => 0x46,
        G => 0x47,
        H => 0x48,
        I => 0x49,
        J => 0x4A,
        K => 0x4B,
        L => 0x4C,
        M => 0x4D,
        N => 0x4E,
        O => 0x4F,
        P => 0x50,
        Q => 0x51,
        R => 0x52,
        S => 0x53,
        T => 0x54,
        U => 0x55,
        V => 0x56,
        W => 0x57,
        X => 0x58,
        Y => 0x59,
        Z => 0x5A,
        Num0 => 0x30, Num1 => 0x31, Num2 => 0x32, Num3 => 0x33,
        Num4 => 0x34, Num5 => 0x35, Num6 => 0x36, Num7 => 0x37,
        Num8 => 0x38, Num9 => 0x39,
        Space => 0x20,
        Enter => 0x0D,
        ArrowLeft => 0x25,
        ArrowUp => 0x26,
        ArrowRight => 0x27,
        ArrowDown => 0x28,

        _ => return None
    })
}

fn get_char_code(c: char) -> Option<u16> {
    Some(match c {
        'a'..='z' => (c as u8 - b'a' + 0x41) as u16,
        'A'..='Z' => (c as u8 - b'A' + 0x41) as u16,
        '0'..='9' => c as u16,
        '.' => 0xBE,
        '/' => 0xBF,
        ',' => 0xBC,
        ';' => 0xBA,
        '=' => 0xBB,
        '-' => 0xBD,
        '[' => 0xDB,
        ']' => 0xDD,
        '\\' => 0xDC,
        '\'' => 0xDE,
        '`' => 0xC0,
        ' ' => 0x20,
        _ => return None
    })
}

fn get_code_name(code: u16) -> String {
    match code {
        0x41..= 0x5A => String::from((code as u8) as char),
        0x30..= 0x39 => String::from((code as u8) as char),
        0xBE => ".".into(),
        0xBF => "/".into(),
        0xBC => ",".into(),
        0xBA => ";".into(),
        0xBB => "=".into(),
        0xBD => "-".into(),
        0xDE => "'".into(),
        0xDB => "[".into(),
        0xDD => "]".into(),
        0xDC => "\\".into(),
        0xC0 => "`".into(),
        0x20 => "Space".into(),
        0x0D => "Enter".into(),
        0x25 => "\u{2190}".into(),
        0x26 => "\u{2191}".into(),
        0x27 => "\u{2192}".into(),
        0x28 => "\u{2193}".into(),

        _ => format!("0x{:02X}", code)
    }
}