mod editor;

use std::time::Instant;

use clap::Parser;
use color_eyre::Report;
use crossterm::terminal;

use editor::Editor;
use tracing_subscriber::EnvFilter;

struct RawModeGuard;
impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}

fn setup() -> Result<(), Report> {
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        std::env::set_var("RUST_LIB_BACKTRACE", "1")
    }
    color_eyre::install()?;

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    Ok(())
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    filename: Option<String>,
}

fn main() -> Result<(), Report> {
    setup()?;

    let args = Args::parse();

    {
        terminal::enable_raw_mode()?;
        let _raw_mode_guard = RawModeGuard;

        let mut editor = Editor::new();
        if let Some(filename) = &args.filename {
            editor.load_file(filename);
        };

        editor.run()?;
    }

    Ok(())
}
