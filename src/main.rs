#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod cli;
mod hotkey;
mod input;
mod text;

use std::{env, thread};

use app::JustInputApp;
use cli::CliAction;

fn main() {
    if let Err(error) = run() {
        eprintln!("just-input: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    match cli::parse_args(env::args_os().skip(1))? {
        CliAction::Gui => run_gui().map_err(|error| error.to_string())?,
        CliAction::Help => {
            print!("{}", cli::help_text());
        }
        CliAction::Type(options) => {
            if !options.delay.is_zero() {
                thread::sleep(options.delay);
            }
            input::type_text(&options.text, options.interval, options.layout)
                .map_err(|error| error.to_string())?;
        }
    }

    Ok(())
}

fn run_gui() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([480.0, 420.0])
            .with_min_inner_size([360.0, 320.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Just Input",
        native_options,
        Box::new(|creation_context| Box::new(JustInputApp::new(creation_context))),
    )
}
