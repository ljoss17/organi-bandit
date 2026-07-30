// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use crate::commands::tauri_generate_schedule;
use crate::utils::parser::read_team_list;

pub mod commands;
pub mod errors;
pub mod impls;
pub mod traits;
pub mod types;
pub mod utils;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            read_team_list,
            tauri_generate_schedule
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
