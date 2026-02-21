use windows::core::w as to_wide;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

use std::thread::sleep;
use std::time::Duration;

use crate::SharedState;

const COLOR: COLORREF = COLORREF(0x00FF00FF);

const LEN: i32 = 20;
const GAP: i32 = 4;

unsafe extern "system" fn define_window(
    window: HWND,
    message: u32,
    wide: WPARAM,
    long: LPARAM
) -> LRESULT {
    unsafe {
        DefWindowProcW(
            window,
            message,
            wide,
            long
        )
    }
}

pub fn start(state: SharedState) {
    unsafe {
        let module = GetModuleHandleW(None)
            .unwrap();

        let overlay_class = to_wide!("overlay");

        let background = CreateSolidBrush(COLOR);

        let class = WNDCLASSW {
            lpfnWndProc: Some(define_window),
            hInstance: module.into(),
            lpszClassName: overlay_class,
            hbrBackground: background,
            ..Default::default()
        };

        RegisterClassW(&class);

        let width = GetSystemMetrics(SM_CXSCREEN);
        let height = GetSystemMetrics(SM_CYSCREEN);

        let window = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            overlay_class,
            to_wide!(""),
            WS_POPUP | WS_VISIBLE,
            0,
            0,
            width,
            height,
            None,
            None,
            Some(module.into()),
            None,
        ).unwrap();

        SetLayeredWindowAttributes(
            window,
            COLOR,
            0,
            LWA_COLORKEY
        ).unwrap();

        loop {
            let mut message = MSG::default();

            while PeekMessageW(&mut message as *mut _, Some(window), 0, 0, PM_REMOVE).as_bool() {
                if message.message == WM_QUIT {
                    return;
                }

                _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }

            let (points, show) = {
                let state = state
                    .lock()
                    .unwrap();

                let points = state
                    .lanes
                    .iter()
                    .flatten()
                    .map(|point| (point.x, point.y, point.color))
                    .collect::<Vec<(i32, i32, [u8; 3])>>();

                (points, state.show_hit_points)
            };

            let context = GetDC(Some(window));
            let device = CreateCompatibleDC(Some(context));
            let bitmap = CreateCompatibleBitmap(
                context,
                width,
                height
            );

            let old_bitmap = SelectObject(
                device,
                bitmap.into()
            );

            let rect = &RECT {
                left: 0,
                top: 0,
                right: width,
                bottom: height,
            };

            let background = CreateSolidBrush(COLOR);

            FillRect(
                device,
                rect,
                background,
            );

            DeleteObject(background.into())
                .unwrap();

            if show {
                for &(x, y, color) in points.iter() {
                    let mut color = COLORREF(
                        color[0] as u32 |
                        (color[1] as u32) << 8 |
                        (color[2] as u32) << 16
                    );

                    if color == COLOR {
                        color = COLORREF(color.0 ^ 1);
                    }

                    let pen = CreatePen(PS_SOLID, 1, color);
                    let old_pen = SelectObject(device, pen.into());

                    MoveToEx(device, x - LEN, y, None).unwrap();
                    LineTo(device, x - GAP, y).unwrap();
                    MoveToEx(device, x + GAP, y, None).unwrap();
                    LineTo(device, x + LEN, y).unwrap();
                    MoveToEx(device, x, y - LEN, None).unwrap();
                    LineTo(device, x, y - GAP).unwrap();
                    MoveToEx(device, x, y + GAP, None).unwrap();
                    LineTo(device, x, y + LEN).unwrap();

                    SelectObject(device, old_pen);
                    DeleteObject(pen.into()).unwrap();
                }
            }

            BitBlt(
                context,
                0,
                0,
                width,
                height,
                Some(device),
                0,
                0,
                SRCCOPY
            ).unwrap();

            SelectObject(device, old_bitmap);
            DeleteObject(bitmap.into()).unwrap();
            DeleteDC(device).unwrap();
            ReleaseDC(Some(window), context);

            sleep(Duration::from_millis(16));
        }
    }
}