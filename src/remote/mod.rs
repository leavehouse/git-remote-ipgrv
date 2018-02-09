use git2;
use hex;
use ipld_git;
use std::env;
use std::fs;
use std::io;

pub use self::error::Error;

mod error;
mod fetch;
mod push;
mod tracker;

fn log_and_print(s: &str) {
   debug!("git <- '{}'", s);
   println!("{}", s);
}

enum Command {
    Push(PushArgs),
    Fetch(FetchArgs),
}

struct PushArgs {
    pub src: String,
    pub dest: String,
    pub force: bool,
}

struct FetchArgs {
    hash: String,
    ref_name: String,
}

pub struct Remote {
    repo: git2::Repository,
    tracker: tracker::Tracker,
}

impl Remote {
    pub fn new() -> Result<Remote, Error> {
        let repo = git2::Repository::open_from_env()?;

        let mut db_path = env::var("GIT_DIR")?;
        // TODO: for windows, convert to std::path::Path, use join(), convert back to string?
        db_path.push_str("/ipgrv");
        fs::create_dir_all(&db_path)?;
        debug!("Remote::new(), db_path = {}", &db_path);
        let tracker = tracker::Tracker::new(&db_path)?;

        Ok(Remote {
            repo: repo,
            tracker: tracker,
        })
    }

    fn list(&self, handler: &Handler) -> Result<Vec<String>, Error> {
        let mut refs = Vec::new();
        let local_branches = self.repo.branches(Some(git2::BranchType::Local))?;
        for branch_result in local_branches {
            let (branch, _) = branch_result?;
            refs.push(format!("? {}", branch.get()
                                            .name()
                                            .expect("Branch name is not utf-8")));
        }

        // For a `git clone` there is (in general) no git directory, so we must
        // consult the hash passed into the program
        if refs.len() == 0 {
            refs.push(format!("{} refs/heads/master", handler.remote_hash()))
        }
        let head_ref = self.repo.find_reference("HEAD")?;
        let head_ref_type = head_ref.kind()
                                    .expect("HEAD ref type is unknown");
        let head_ref_string = match head_ref_type {
            git2::ReferenceType::Oid => format!("{} HEAD", head_ref.target().unwrap()),
            git2::ReferenceType::Symbolic => format!("@{} HEAD",
                head_ref.symbolic_target()
                        .expect("HEAD symbolic target is not utf-8")
                        .to_string()),
        };
        refs.push(head_ref_string);
        Ok(refs)
    }

    // `src` is the local ref being pushed, `dest` is the remote ref?
    // Returns the hash that `src` points to
    fn push(&self, src: &str, dest: &str, force: bool) -> Result<Vec<u8>, Error> {
        // get reference associated with `src`, then get src's hash
        let src_ref = self.repo.find_reference(src)?.resolve()?;
        let src_hash: git2::Oid = src_ref.target().unwrap();
        debug!("    pushing, hash = {}", src_hash);

        let mut push_helper = push::PushHelper::new(&self.repo, &self.tracker);
        push_helper.push(src_hash)?;
        Ok(src_hash.as_bytes().to_vec())
    }

    fn fetch(&self, hash: String, ref_name: String) -> Result<(), Error> {
        debug!("    fetching, hash = {}, ref_name = {}", hash, ref_name);
        let mut fetch_helper = fetch::FetchHelper::new(&self.tracker);
        fetch_helper.fetch(hash)?;
        Ok(())
    }

    // Listen for commands coming in over stdin, respond to them by writing to
    // stdout.
    pub fn process_commands(&mut self, handler: &Handler) -> Result<(), Error> {
       let stdin = io::stdin();
       let mut command_batch = Vec::new();
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
               log_and_print("fetch");
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
               let refs = self.list(handler)?;
               refs.iter().for_each(|r| log_and_print(r));
               log_and_print("");
           } else if command.starts_with("push ") {
               let src_dest = &command[(4+1)..];
               let refs = src_dest.split(":").collect::<Vec<_>>();
               command_batch.push(Command::Push(PushArgs {
                   src: refs[0].to_string(),
                   dest: refs[1].to_string(),
                   force: src_dest.starts_with("+"),
               }));
           } else if command.starts_with("fetch ") {
               let params = &command[(5+1)..];
               let parts = params.split(" ").collect::<Vec<_>>();
               command_batch.push(Command::Fetch(FetchArgs {
                   hash: parts[0].to_string(),
                   ref_name: parts[1].to_string(),
               }));
           } else if command == "" {
               for command in command_batch {
                   self.perform_batched_command(command)?;
               }
               // TODO: it's weird because for push, each push
               // should return an "ok" or "error" message, but for fetches
               // there's just a single blank line that's output. Consequence
               // of conflating two separate things here: blank line terminates
               // both a push batch and a fetch batch. Should probably separate
               // these two out
               log_and_print("");
               // TODO: don't return here? see above, but specifically you can
               // have multiple push batches. returning here does not handle
               // this correctly
               return Ok(())
           } else {
               return Err(Error::InvalidCommand(command.to_string()))
           }
       }
    }

    fn perform_batched_command(&mut self, command: Command) -> Result<(), Error> {
        match command {
           Command::Push(PushArgs { src, dest, force }) => {
               let src_hash = self.push(&src, &dest, force)?;
               eprintln!("Pushed to IPFS as:  ipld::{}",
                         hex::encode(&src_hash));
               eprintln!("Head CID is {}",
                         ipld_git::util::sha1_to_cid(&src_hash).unwrap());
               log_and_print(&format!("ok {}", src));
           },
           Command::Fetch(FetchArgs { hash, ref_name }) => {
               self.fetch(hash, ref_name)?;
           },
        }
        Ok(())
    }
}

pub struct Handler {
    remote_hash: String,
}

impl Handler {
    pub fn new(hash: String) -> Handler {
        Handler { remote_hash: hash }
    }
    pub fn remote_hash(&self) -> &str {
        &self.remote_hash
    }
}
