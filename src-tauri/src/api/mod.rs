mod helpers;

pub mod auth_commands;
pub mod download_commands;
pub mod game_commands;
pub mod library_commands;
pub mod settings_commands;
pub mod source_commands;

// Re-export all commands for registration
pub use auth_commands::*;
pub use download_commands::*;
pub use game_commands::*;
pub use library_commands::*;
pub use settings_commands::*;
pub use source_commands::*;
