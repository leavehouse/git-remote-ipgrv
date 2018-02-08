use git2;
use hex;
use ipld_git;
use lmdb;
use multihash;
use std::collections::VecDeque;
use std::env;
use std::fs;
use std::io;

pub use self::error::Error;
use ipfs_api;

mod error;
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

    pub fn list(&self, handler: &Handler) -> Result<Vec<String>, Error> {
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
        } else {
            let head_ref = self.repo.find_reference("HEAD")?;
            let head_ref_type = head_ref.kind()
                                        .expect("HEAD ref type is unknown");
            let head_target = match head_ref_type {
                git2::ReferenceType::Oid => format!("{}", head_ref.target().unwrap()),
                git2::ReferenceType::Symbolic =>
                    head_ref.symbolic_target()
                            .expect("HEAD symbolic target is not utf-8")
                            .to_string(),
            };

            refs.push(format!("{} HEAD", head_target));
        }
        Ok(refs)
    }

    // `src` is the local ref being pushed, `dest` is the remote ref?
    // Returns the hash that `src` points to
    pub fn push(&self, src: &str, dest: &str, force: bool) -> Result<Vec<u8>, Error> {
        // get reference associated with `src`, then get src's hash
        let src_ref = self.repo.find_reference(src)?.resolve()?;
        let src_hash: git2::Oid = src_ref.target().unwrap();
        debug!("    pushing, hash = {}", src_hash);

        let mut push_helper = PushHelper::new(&self.repo, &self.tracker);
        push_helper.push(src_hash)?;
        Ok(src_hash.as_bytes().to_vec())
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
               let src_dest = &command[5..];
               let refs = src_dest.split(":").collect::<Vec<_>>();
               command_batch.push(Command::Push(PushArgs {
                   src: refs[0].to_string(),
                   dest: refs[1].to_string(),
                   force: src_dest.starts_with("+"),
               }));
           } else if command.starts_with("fetch ") {
               let params = &command[5..];
               let parts = params.split(" ").collect::<Vec<_>>();
               command_batch.push(Command::Fetch(FetchArgs {
                   hash: parts[0].to_string(),
                   ref_name: parts[1].to_string(),
               }));
           } else if command == "" {
               for command in command_batch {
                   self.perform_batched_command(command)?;
               }
               log_and_print("");
               // TODO: don't return here? need to find some way to check if
               // if no more commands are coming
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
               Ok(())
           },
           Command::Fetch(args) => unimplemented!(),
        }
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

struct PushHelper<'a> {
    queue: VecDeque<git2::Oid>,
    repo: &'a git2::Repository,
    tracker: &'a tracker::Tracker,
}

impl<'a> PushHelper<'a> {
    fn new(repo: &'a git2::Repository, tracker: &'a tracker::Tracker) -> PushHelper<'a> {
        PushHelper {
            queue: VecDeque::new(),
            repo: repo,
            tracker: tracker,
        }
    }

    fn push(&mut self, hash: git2::Oid) -> Result<(), Error>{
        self.queue.push_back(hash);
        self.push_queue()
    }

    // push each of the objects in the queue into IPFS (as IPLD).
    fn push_queue(&mut self) -> Result<(), Error> {
        while let Some(oid) = self.queue.pop_front() {
            debug!("    pushing oid = {}", oid);

            if self.tracker.has_entry(oid.as_bytes())? {
                debug!("    already have this oid, skipping");
                continue;
            }

            let obj_bytes = self.push_object(oid)?;

            self.tracker.add_entry(oid.as_bytes())?;

            self.enqueue_links(&obj_bytes)?;
        }
        Ok(())
    }

    // Push git object into ipfs, returning the vector of bytes of the raw git
    // object.
    fn push_object(&mut self, oid: git2::Oid) -> Result<Vec<u8>, Error> {
        // read the git object into memory
        let odb = self.repo.odb()?;
        let odb_obj = odb.read(oid)?;
        let raw_obj = odb_obj.data();

        let mut full_obj = Vec::with_capacity(raw_obj.len() + 12);
        match odb_obj.kind() {
            git2::ObjectType::Blob => full_obj.extend_from_slice(b"blob "),
            git2::ObjectType::Tree => full_obj.extend_from_slice(b"tree "),
            git2::ObjectType::Commit => full_obj.extend_from_slice(b"commit "),
            git2::ObjectType::Tag => full_obj.extend_from_slice(b"tag "),
            _ => unimplemented!(),
        }
        full_obj.extend_from_slice(format!("{}", raw_obj.len()).as_bytes());
        full_obj.push(0);
        full_obj.extend_from_slice(raw_obj);

        // `put` the git object bytes onto the ipfs DAG.
        let api = ipfs_api::Shell::new_local().map_err(Error::ApiError)?;
        api.dag_put(&full_obj, "raw", "git").map_err(Error::ApiError)?;
        Ok(full_obj)
    }

    fn enqueue_links(&mut self, obj_bytes: &[u8]) -> Result<(), Error> {
        //let node = ipld_git::parse_object(obj_bytes).map_err(Error::IpldGitError)?;
        let node = match ipld_git::parse_object(obj_bytes) {
            Err(e) => return Err(Error::IpldGitError(e)),
            Ok(node) => node,
        };

        for link in node.links() {
            let link_multihash = multihash::decode(&link.cid.hash)?;
            debug!("        link digest: {:?}", link_multihash.digest);
            if self.tracker.has_entry(link_multihash.digest)? {
                debug!("        already have this link, skipping");
                continue;
            }
            self.queue.push_back(git2::Oid::from_bytes(link_multihash.digest)?)
        }
        Ok(())
    }
}
