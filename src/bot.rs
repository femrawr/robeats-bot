use windows::Win32::Media::timeBeginPeriod;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use std::mem::size_of;
use std::array::from_fn;
use std::thread::{sleep, yield_now};
use std::time::{Duration, Instant};

use crate::{HitPoint, SharedState};

pub fn start(state: SharedState) {
    unsafe {
        timeBeginPeriod(1);
    }

    let mut keys_state = [false; 4];
    let mut clicked = [false; 4];
    let mut on_roblox = false;
    let mut roblox_timer = Instant::now();
    let mut iter: u32 = 0;
    let mut enabled = false;
    let mut roblox_only = true;
    let mut threshold: u8 = 160;
    let mut lanes: [Vec<HitPoint>; 4] = [vec![], vec![], vec![], vec![]];
    let mut keys: [u16; 4] = [0x5A, 0x58, 0xBE, 0xBF]; // z x . /
    let mut scan_interval: u32 = 2;
    let mut click_delay: u32 = 0;
    let mut ln_mode = true;
    let mut detect_time: [Option<Instant>; 4] = [None; 4];

    loop {
        iter = iter.wrapping_add(1);
        if iter & 63 == 0 {
            if let Ok(state) = state.try_lock() {
                enabled = state.enabled;
                ln_mode = state.hold_notes;
                roblox_only = state.roblox_check;
                threshold = state.threshold;
                scan_interval = state.scan_interval;
                click_delay = state.click_delay;
                keys = state.keys;

                for i in 0..4 {
                    lanes[i] = state.lanes[i].clone();
                }
            }
        }

        if roblox_timer.elapsed() > Duration::from_millis(200) {
            on_roblox = check_roblox();
            if let Ok(mut state) = state.try_lock() {
                state.on_roblox = on_roblox;
            }

            roblox_timer = Instant::now();
        }

        if !enabled || (roblox_only && !on_roblox) {
            for i in 0..4 {
                if keys_state[i] {
                    send_input(VIRTUAL_KEY(keys[i]), false);
                    keys_state[i] = false;
                }

                clicked[i] = false;
            }

            sleep(Duration::from_millis(5));
            continue;
        }

        let brightness = get_brightnesses(&lanes);
        let delay = Duration::from_millis(click_delay as u64);

        for i in 0..4 {
            let note = brightness[i] > threshold;

            if note && !keys_state[i] && !clicked[i] {
                match detect_time[i] {
                    None => {
                        detect_time[i] = Some(Instant::now());
                    }

                    Some(t) if t.elapsed() >= delay => {
                        send_input(VIRTUAL_KEY(keys[i]), true);
                        keys_state[i] = true;

                        if !ln_mode {
                            send_input(VIRTUAL_KEY(keys[i]), false);

                            keys_state[i] = false;
                            clicked[i] = true;
                            detect_time[i] = None;
                        }
                    }

                    _ => {}
                }
            } else if !note {
                if keys_state[i] {
                    send_input(VIRTUAL_KEY(keys[i]), false);
                    keys_state[i] = false;
                }

                detect_time[i] = None;
                clicked[i] = false;
            }
        }

        if scan_interval > 0 {
            sleep(Duration::from_millis(scan_interval as u64));
        } else {
            yield_now();
        }
    }
}

fn get_brightnesses(lanes: &[Vec<HitPoint>; 4]) -> [u8; 4] {
    let flat = lanes
        .iter()
        .flatten()
        .map(|point| (point.x, point.y))
        .collect::<Vec<(i32, i32)>>();

    if flat.is_empty() {
        return [0; 4];
    }

    let min_x = flat.iter().map(|point| point.0).min().unwrap();
    let max_x = flat.iter().map(|point| point.0).max().unwrap();
    let min_y = flat.iter().map(|point| point.1).min().unwrap();
    let max_y = flat.iter().map(|point| point.1).max().unwrap();

    let the_w = (max_x - min_x + 1).max(1);
    let the_h = (max_y - min_y + 1).max(1);

    unsafe {
        let context = GetDC(None);
        let device = CreateCompatibleDC(Some(context));
        let bitmap = CreateCompatibleBitmap(context, the_w, the_h);
        let old_bitmap = SelectObject(device, bitmap.into());
        let _ = BitBlt(device, 0, 0, the_w, the_h, Some(context), min_x, min_y, SRCCOPY);

        let result = from_fn(|lane| {
            let mut best = 0u8;

            for point in &lanes[lane] {
                let pixel = GetPixel(
                    device,
                    point.x - min_x,
                    point.y - min_y
                );

                if pixel.0 == 0xFFFFFFFF {
                    continue;
                }

                let r = (pixel.0 & 0xFF) as u8;
                let g = ((pixel.0 >> 8) & 0xFF) as u8;
                let b = ((pixel.0 >> 16) & 0xFF) as u8;
                best = best.max(r.max(g).max(b));
            }

            best
        });

        SelectObject(device, old_bitmap);
        DeleteObject(bitmap.into()).unwrap();
        DeleteDC(device).unwrap();
        ReleaseDC(None, context);

        result
    }
}

fn send_input(code: VIRTUAL_KEY, down: bool) {
    let map = unsafe {
        MapVirtualKeyW(code.0 as u32, MAPVK_VK_TO_VSC)
    } as u16;

    let mut flags = KEYEVENTF_SCANCODE;

    if !down {
        flags |= KEYEVENTF_KEYUP;
    }

    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: map,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    unsafe {
        SendInput(
            &[input],
            size_of::<INPUT>() as i32
        );
    }
}

fn check_roblox() -> bool {
    unsafe {
        let window = GetForegroundWindow();
        let mut buffer = [0u16; 256];
        let length = GetWindowTextW(window, &mut buffer);

        String::from_utf16_lossy(&buffer[..length as usize])
            .to_lowercase()
            .contains("roblox")
    }
}