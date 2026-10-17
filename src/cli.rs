use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "bendershot")]
#[command(author, version, about = "High-performance screenshot capture, screen recording, and annotation tool for Linux Wayland", long_about = None)]
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

    /// Open settings dialog for shortcuts and preferences
    #[command(alias = "config")]
    Settings,

    /// Open graphical setup wizard for shortcuts and autostart configuration
    #[command(alias = "setup")]
    Wizard,

    /// Non-interactive automated setup (enables default shortcuts, autostart, and daemon)
    AutoSetup,

    /// Check for updates on GitHub and offer to update to the latest release
    Update {
        /// Only check if an update is available without installing
        #[arg(long, short = 'c')]
        check: bool,

        /// Open graphical update dialog window
        #[arg(long, short = 'g')]
        gui: bool,

        /// Automatically proceed with update without interactive confirmation
        #[arg(long, short = 'y')]
        yes: bool,
    },

    #[command(hide = true)]
    RecordWorker,
}
