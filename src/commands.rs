use git2;
use std::io;

#[derive(Debug)]
pub enum ProcessError {
    IoError(io::Error),
    Git2Error(git2::Error),
    InvalidCommand(String),
}

impl From<io::Error> for ProcessError {
    fn from(e: io::Error) -> Self {
        ProcessError::IoError(e)
    }
}

impl From<git2::Error> for ProcessError {
    fn from(e: git2::Error) -> Self {
        ProcessError::Git2Error(e)
    }
}

fn log_and_print(s: &str) {
   debug!("git <- '{}'", s);
   println!("{}", s);
}

// Listen for commands coming in over stdin, respond to them by writing to
// stdout.
pub fn process_commands() -> Result<(), ProcessError> {
   let stdin = io::stdin();
   debug!("processing commands");
   let repo = git2::Repository::open_from_env()?;
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
           let refs = list(&repo)?;
           refs.iter().for_each(|r| log_and_print(r));
           log_and_print("");
       } else if command.starts_with("push ") {
           let src_dest = &command[5..];

           let refs = src_dest.split(":").collect::<Vec<_>>();
           push(&repo, refs[0], refs[1], src_dest.starts_with("+"));

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


fn list(repo: &git2::Repository) -> Result<Vec<String>, git2::Error> {
    let mut refs = Vec::new();
    for branch_result in repo.branches(Some(git2::BranchType::Local))? {
        let (branch, _) = branch_result?;
        refs.push(format!("? {}", branch.get()
                                        .name()
                                        .expect("Branch name is not utf-8")));
    }

    let head_ref = repo.find_reference("HEAD")?;
    let head_ref_type = head_ref.kind()
                            .expect("HEAD ref type is unknown");
    let head_target = match head_ref_type {
        git2::ReferenceType::Oid => {
            debug!("    head is oid");
            format!("{}", head_ref.target().unwrap())
        },
        git2::ReferenceType::Symbolic =>
            head_ref.symbolic_target()
                    .expect("HEAD symbolic target is not utf-8")
                    .to_string(),
    };

    refs.push(format!("{} HEAD", head_target));
    Ok(refs)
}

fn push(repo: &git2::Repository, src: &str, dest: &str, force: bool) -> Result<(), git2::Error> {
    // get reference associated with `src`, then get src's hash
    let src_ref = repo.find_reference(src)?.resolve()?;
    let src_hash = src_ref.target().unwrap();
    // check tracker for src_hash.
    // if it exists, return, because theres no need to push
    // else, push
    unimplemented!();
    // set `dest` to `src's hash in the tracekr
    // get the cid from the git hash
    // print out the cid/sha1 hash
}
