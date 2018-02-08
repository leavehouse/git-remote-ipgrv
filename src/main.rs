extern crate chrono;
extern crate fern;
extern crate git2;
extern crate hex;
extern crate home;
extern crate lmdb_zero as lmdb;
#[macro_use] extern crate log;
extern crate ipld_git;
extern crate multihash;
extern crate reqwest;
extern crate url;

use remote::Remote;
use std::{env, process};

mod ipfs_api;
mod remote;

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
       eprintln!("Usage: git-remote-ipgrv <remote> <url>");
       process::exit(1);
   }

   if let Err(e) = setup_logger() {
       eprintln!("Error setting up logger: {:?}", e);
       process::exit(1);
   }
   debug!("{:?}", args);

   if let Err(e) = run(args.into_iter().nth(3).unwrap()) {
       eprintln!("Error running helper: {:?}", e);
       process::exit(1);
   }
}

fn run(remote_hash: String) -> Result<(), remote::Error> {
   let remote = Remote::new()?;
   remote.process_commands()
}
