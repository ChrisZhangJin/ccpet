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
            eprintln!("[ccpet] main window ready, click-through ON (hold Ctrl to drag)");

            // Position window at bottom-right corner of primary monitor
            if let Some(monitor) = window.current_monitor().ok().flatten() {
                let monitor_size = monitor.size();
                let scale = monitor.scale_factor();
                let win_w = 300.0;
                let win_h = 300.0;
                let x = (monitor_size.width as f64 / scale) - win_w - 20.0;
                let y = (monitor_size.height as f64 / scale) - win_h - 60.0;
                let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(x, y)));
            }

            // Start HTTP server on background thread
            let app_handle = app.handle().clone();
            thread::spawn(move || {
                start_http_server(app_handle);
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Frontend invokes this to move the pet window during a Ctrl+drag.
/// Coordinates are in physical pixels (Logical) and are absolute screen
/// positions, matching what `mousemove` reports on Windows.
#[tauri::command]
fn set_window_position(window: WebviewWindow, x: f64, y: f64) -> Result<(), String> {
    window
        .set_position(tauri::Position::Logical(tauri::LogicalPosition::new(x, y)))
        .map_err(|e| e.to_string())
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
