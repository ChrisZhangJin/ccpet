use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use std::thread;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![set_window_position])
        .setup(|app| {
            // Get the main window and set click-through
            let window = app.get_webview_window("main").expect("failed to get main window");
            window.set_ignore_cursor_events(true).expect("failed to set ignore cursor events");
            eprintln!("[ccpet] main window ready, click-through ON (hold platform drag key to drag)");

            // Position window at bottom-right corner of the primary monitor.
            // macOS reserves additional space for the menu bar and Dock.
            if let Some(monitor) = window.current_monitor().ok().flatten() {
                let monitor_size = monitor.size();
                let scale = monitor.scale_factor();
                let win_w = 300.0;
                let win_h = 300.0;
                let bottom_inset = if cfg!(target_os = "macos") { 90.0 } else { 60.0 };
                let x = (monitor_size.width as f64 / scale) - win_w - 20.0;
                let y = (monitor_size.height as f64 / scale) - win_h - bottom_inset;
                let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(x, y)));
            }

            // Start HTTP server on background thread
            let app_handle = app.handle().clone();
            thread::spawn(move || {
                start_http_server(app_handle);
            });

            // On Windows, start a global Ctrl key watcher. Click-through windows
            // never get keyboard focus, so we poll the key state via Win32 API.
            #[cfg(target_os = "windows")]
            start_ctrl_watcher(app.handle().clone());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Frontend invokes this to move the pet window during a platform-modifier drag.
/// Coordinates are logical screen positions.
#[tauri::command]
fn set_window_position(window: WebviewWindow, x: f64, y: f64) -> Result<(), String> {
    window
        .set_position(tauri::Position::Logical(tauri::LogicalPosition::new(x, y)))
        .map_err(|e| e.to_string())
}

/// Windows-only: poll the global Ctrl key state via GetAsyncKeyState.
/// Click-through windows can't receive keyboard events because they never
/// get focus on Windows (unlike macOS where app-level keyboard dispatch works).
#[cfg(target_os = "windows")]
fn start_ctrl_watcher(app: AppHandle) {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    const VK_CONTROL: i32 = 0x11;

    thread::spawn(move || {
        let mut was_pressed = false;
        loop {
            let state = unsafe { GetAsyncKeyState(VK_CONTROL) };
            let is_pressed = (state & 0x8000) != 0;

            if is_pressed && !was_pressed {
                let _ = app.emit("drag-modifier-down", ());
            } else if !is_pressed && was_pressed {
                let _ = app.emit("drag-modifier-up", ());
            }

            was_pressed = is_pressed;
            thread::sleep(std::time::Duration::from_millis(16)); // ~60 Hz
        }
    });
    eprintln!("[ccpet] Windows global Ctrl watcher started (~60 Hz polling)");
}

fn start_http_server(app: AppHandle) {
    // Bind to 0.0.0.0 so the server is reachable from the LAN (e.g. another
    // machine's Claude Code can POST /bark). Localhost (127.0.0.1) traffic
    // is also accepted. Restrict access via Windows Firewall inbound rule.
    let server = tiny_http::Server::http("0.0.0.0:4242").expect("failed to start HTTP server on 0.0.0.0:4242");
    println!("ccpet HTTP server listening on 0.0.0.0:4242 (reachable from LAN)");

    for request in server.incoming_requests() {
        match (request.method(), request.url()) {
            (tiny_http::Method::Post, "/bark") => {
                eprintln!("[ccpet] HTTP POST /bark received");
                // Emit "action" event to frontend
                let app_for_emit = app.clone();
                match app_for_emit.emit("action", ()) {
                    Ok(_) => eprintln!("[ccpet] event 'action' emitted OK"),
                    Err(e) => eprintln!("[ccpet] event emit FAILED: {:?}", e),
                }
                let response = tiny_http::Response::from_string("OK")
                    .with_status_code(200)
                    .with_header(
                        tiny_http::Header::from_bytes(
                            &b"Access-Control-Allow-Origin"[..],
                            &b"*"[..],
                        ).unwrap()
                    );
                let _ = request.respond(response);
                eprintln!("[ccpet] response 200 sent");
            }
            (tiny_http::Method::Options, _) => {
                // CORS preflight
                let response = tiny_http::Response::from_string("")
                    .with_status_code(204)
                    .with_header(
                        tiny_http::Header::from_bytes(
                            &b"Access-Control-Allow-Origin"[..],
                            &b"*"[..],
                        ).unwrap()
                    )
                    .with_header(
                        tiny_http::Header::from_bytes(
                            &b"Access-Control-Allow-Methods"[..],
                            &b"POST, OPTIONS"[..],
                        ).unwrap()
                    )
                    .with_header(
                        tiny_http::Header::from_bytes(
                            &b"Access-Control-Allow-Headers"[..],
                            &b"Content-Type"[..],
                        ).unwrap()
                    );
                let _ = request.respond(response);
            }
            _ => {
                let response = tiny_http::Response::from_string("Not Found")
                    .with_status_code(404);
                let _ = request.respond(response);
            }
        }
    }
}
