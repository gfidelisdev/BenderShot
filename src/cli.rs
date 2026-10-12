use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "enhanced-shot")]
#[command(author, version, about = "High-performance screenshot capture and annotation tool for Linux Wayland", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Capture a selected screen region and open Editor Window (default)
    Region,

    /// Capture full screen and open Editor Window
    Full,

    /// Capture active window and open Editor Window
    Window,

    /// Run resident background daemon with GNOME top-bar indicator
    Daemon,

    /// Open an existing image file directly in the Editor Window
    Edit {
        /// Path to the image file to open
        path: PathBuf,
    },

    /// Toggle screen recording (starts recording or stops active recording and opens video editor)
    Record,

    #[command(hide = true)]
    RecordWorker,
}
