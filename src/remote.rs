use hex;
use ipld_git;
use std::io;

use helper;

#[derive(Debug)]
pub enum ProcessError {
    IoError(io::Error),
    HelperError(helper::Error),
    InvalidCommand(String),
}

impl From<io::Error> for ProcessError {
    fn from(e: io::Error) -> Self {
        ProcessError::IoError(e)
    }
}

impl From<helper::Error> for ProcessError {
    fn from(e: helper::Error) -> Self {
        ProcessError::HelperError(e)
    }
}

fn log_and_print(s: &str) {
   debug!("git <- '{}'", s);
   println!("{}", s);
}

struct PushArgs {
    pub src: String,
    pub dest: String,
    pub force: bool,
}

pub struct Remote;

impl Remote {
    // Listen for commands coming in over stdin, respond to them by writing to
    // stdout.
    pub fn process_commands(&self) -> Result<(), ProcessError> {
       let helper = helper::Helper::new()?;
       let stdin = io::stdin();
       let mut push_batch = Vec::new();
       debug!("processing commands");
       loop {
           let mut command_line = String::new();
           stdin.read_line(&mut command_line)?;
           let command = command_line.trim_matches('\n');

           debug!(" -> {}", command);

           if command == "capabilities" {
               // "Lists the capabilities of the helper, one per line, ending with
               // a blank line."
               log_and_print("push");
               log_and_print("");
           } else if command.starts_with("list") {
               // list -
               // "Lists the refs, one per line, in the format '<value> <name>
               // [<attr> ...]'. The value may be a hex sha1 hash, '@<dest>' for a
               // symref, or '?' to indicate that the helper could not get the value
               // of the ref."
               //
               // list for-push -
               // used to prepare for a `git push`
               let refs = helper.list()?;
               refs.iter().for_each(|r| log_and_print(r));
               log_and_print("");
           } else if command.starts_with("push ") {
               let src_dest = &command[5..];

               let refs = src_dest.split(":").collect::<Vec<_>>();
               push_batch.push(PushArgs {
                   src: refs[0].to_string(),
                   dest: refs[1].to_string(),
                   force: src_dest.starts_with("+"),
               });
           } else if command == "" {
               for PushArgs { src, dest, force } in push_batch {
                   let src_hash = helper.push(&src, &dest, force)?;
                   eprintln!("Pushed to IPFS as:  ipld::{}", hex::encode(&src_hash));
                   eprintln!("Head CID is {}", ipld_git::util::sha1_to_cid(&src_hash).unwrap());
                   log_and_print(&format!("ok {}", src));
               }
               log_and_print("");
               // TODO: don't return here? need to find some way to check if
               // if no more commands are coming
               return Ok(())
           } else {
               return Err(ProcessError::InvalidCommand(command.to_string()))
           }
       }
    }
}
