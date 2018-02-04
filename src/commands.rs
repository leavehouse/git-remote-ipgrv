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

// Listen for commands coming in over stdin, respond to them by writing to
// stdout.
pub fn process_commands() -> Result<(), ProcessError> {
   let helper = helper::Helper::new()?;
   let stdin = io::stdin();
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
           helper.push(refs[0], refs[1], src_dest.starts_with("+"))?;

           eprintln!("Hey, so we got a PUSH command and are about to panic");
           unimplemented!()
       } else if command == "" {
           // TODO: this is technically wrong for the blank command to be
           // separate. What actually happens is git sends one or more batches
           // of `push` commands, each batch terminated by a blank command.
           // However, I'm not aware of any other command using blank lines,
           // so it is most convenient to separate it out like this. Also, 
           // currently we're just pushing things as they come rather than 
           // batching pushes together, since I don't see what difference it
           // makes.
       } else {
           return Err(ProcessError::InvalidCommand(command.to_string()))
       }
   }
}
