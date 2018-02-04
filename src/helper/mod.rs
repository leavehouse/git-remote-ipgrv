use git2;
use lmdb;
use std::env;
use std::fs;
use std::io;

mod tracker;

#[derive(Debug)]
pub enum Error {
    EnvVarError(env::VarError),
    Git2Error(git2::Error),
    IoError(io::Error),
    LmdbError(lmdb::Error),
}

impl From<env::VarError> for Error {
    fn from(e: env::VarError) -> Self {
        Error::EnvVarError(e)
    }
}

impl From<git2::Error> for Error {
    fn from(e: git2::Error) -> Self {
        Error::Git2Error(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IoError(e)
    }
}

impl From<lmdb::Error> for Error {
    fn from(e: lmdb::Error) -> Self {
        Error::LmdbError(e)
    }
}

pub struct Helper {
    repo: git2::Repository,
    tracker: tracker::Tracker,
}

impl Helper {
    pub fn new() -> Result<Helper, Error> {
        let repo = git2::Repository::open_from_env()?;

        let mut db_path = env::var("GIT_DIR")?;
        // TODO: for windows, convert to std::path::Path, use join(), convert back to string?
        db_path.push_str("/ipgrv");
        fs::create_dir_all(&db_path)?;
        debug!("Helper::new(), db_path = {}", &db_path);
        let tracker = tracker::Tracker::new(&db_path)?;

        Ok(Helper {
            repo: repo,
            tracker: tracker,
        })
    }

    pub fn list(&self) -> Result<Vec<String>, Error> {
        let mut refs = Vec::new();
        let local_branches = self.repo.branches(Some(git2::BranchType::Local))?;
        for branch_result in local_branches {
            let (branch, _) = branch_result?;
            refs.push(format!("? {}", branch.get()
                                            .name()
                                            .expect("Branch name is not utf-8")));
        }

        let head_ref = self.repo.find_reference("HEAD")?;
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

    pub fn push(&self, src: &str, dest: &str, force: bool) -> Result<(), Error> {
        // get reference associated with `src`, then get src's hash
        let src_ref = self.repo.find_reference(src)?.resolve()?;
        let src_hash: git2::Oid = src_ref.target().unwrap();
        debug!("    pushing, hash = {}", src_hash);
        // TODO: check tracker for src_hash.
        // if it exists, return, because theres no need to push
        // else, push
        unimplemented!();
        // TODO: set `dest` to `src's hash in the tracekr

        // read the git object into memory
        let odb = self.repo.odb()?;
        let odb_obj = odb.read(src_hash)?;
        let raw_obj = odb_obj.data();

        // `put` the git object bytes onto the ipfs DAG.
    }
}
