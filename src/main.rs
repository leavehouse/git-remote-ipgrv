extern crate chrono;
extern crate fern;
extern crate git2;
#[macro_use] extern crate log;

use std::{env, process};

use commands::process_commands;

mod commands;

fn setup_logger() -> Result<(), fern::InitError> {
   fern::Dispatch::new()
       .format(|out, message, record| {
           out.finish(format_args!(
              "{}[{}][{}] {}",
              chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
              record.target(),
              record.level(),
              message
           ))
       })
       .level(log::LevelFilter::Debug)
       .chain(fern::log_file("git-remote-ipgrv.log")?)
       .apply()?;
    Ok(())
}

fn main() {
   let args: Vec<String> = env::args().collect();
   if args.len() !=  3 {
       println!("Usage: git-remote-ipgrv <remote> <url>");
       process::exit(1);
   }

   if let Err(e) = setup_logger() {
       println!("Error setting up logger: {:?}", e);
       process::exit(1);
   }
   debug!("{:?}", args);

   if let Err(e) = process_commands() {
       println!("Error processing: {:?}", e);
       process::exit(1);
   }
}
