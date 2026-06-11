use crate::models::error::{AppError, AppResult};

#[tauri::command]
pub fn window_start_dragging(window: tauri::Window) -> AppResult<()> {
    window.start_dragging().map_err(|error| {
        AppError::new(
            "WINDOW_START_DRAGGING_FAILED",
            format!("Failed to start window dragging: {error}"),
            "Try dragging the window again from the top bar.",
        )
    })
}
