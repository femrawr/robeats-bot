#![windows_subsystem = "windows"]

mod overlay;
mod gui;
mod bot;

use std::sync::{Arc, Mutex};
use std::thread::spawn;

use serde::{Deserialize, Serialize};

pub type SharedState = Arc<Mutex<Shared>>;

fn main() {
    let state = Arc::new(Mutex::new(Shared::new()));

    let state1 = state.clone();
    spawn(move || bot::start(state1));

    let state2 = state.clone();
    spawn(move || overlay::start(state2));

    gui::start(state);
}

#[derive(Clone, Serialize, Deserialize)]
pub struct HitPoint {
    pub x: i32,
    pub y: i32,
    pub color: [u8; 3]
}

impl HitPoint {
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
            color: [255, 0, 0]
        }
    }
}

pub struct Shared {
    pub lanes: [Vec<HitPoint>; 4],
    pub keys: [u16; 4],
    pub enabled: bool,
    pub show_hit_points: bool,
    pub hold_notes: bool,
    pub roblox_check: bool,
    pub on_roblox: bool,
    pub threshold: u8,
    pub scan_interval: u32,
    pub click_delay: u32
}

impl Shared {
    fn new() -> Self {
        Self {
            lanes: [
                vec![HitPoint::new(200, 600)],
                vec![HitPoint::new(350, 600)],
                vec![HitPoint::new(500, 600)],
                vec![HitPoint::new(650, 600)],
            ],
            keys: [0x5A, 0x58, 0xBE, 0xBF],
            enabled: false,
            show_hit_points: true,
            hold_notes: true,
            roblox_check: true,
            on_roblox: false,
            threshold: 160,
            scan_interval: 2,
            click_delay: 0
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub lanes: [Vec<HitPoint>; 4],
    pub keys: [u16; 4],
    pub enabled: bool,
    pub show_hit_points: bool,
    pub hold_notes: bool,
    pub roblox_check: bool,
    pub threshold: u8,
    pub scan_interval: u32,
    pub click_delay: u32
}

impl Config {
    pub fn get(state: &Shared) -> Self {
        Self {
            lanes: state.lanes.clone(),
            keys: state.keys,
            enabled: state.enabled,
            show_hit_points: state.show_hit_points,
            hold_notes: state.hold_notes,
            roblox_check: state.roblox_check,
            threshold: state.threshold,
            scan_interval: state.scan_interval,
            click_delay: state.click_delay
        }
    }

    pub fn set(&self, state: &mut Shared) {
        state.lanes = self.lanes.clone();
        state.keys = self.keys;
        state.enabled = self.enabled;
        state.show_hit_points = self.show_hit_points;
        state.hold_notes = self.hold_notes;
        state.roblox_check = self.roblox_check;
        state.threshold = self.threshold;
        state.scan_interval = self.scan_interval;
        state.click_delay = self.click_delay;
    }
}