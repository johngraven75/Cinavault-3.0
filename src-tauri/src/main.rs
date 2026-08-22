#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Carry-forward compatibility markers for the Tauri v2 library entrypoint.
// The actual wiring lives in src-tauri/src/lib.rs:
// mod remote_connectivity;
// mod build_identity;
// remote_connectivity::configure
// remote_connectivity::start_remote_connectivity
// remote_connectivity::stop_remote_connectivity
// remote_connectivity::get_remote_connectivity_status
// build_identity::get_current_build_info()
/*
Some(true),
                            Some(true),
*/

fn main() {
    cinavault_premium_lib::run();
}
