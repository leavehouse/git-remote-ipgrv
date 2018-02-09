use git2;
use ipld_git;
use multihash;
use std::collections::VecDeque;

use ipfs_api;
use super::Error;
use super::tracker;

pub struct PushHelper<'a> {
    queue: VecDeque<git2::Oid>,
    repo: &'a git2::Repository,
    tracker: &'a tracker::Tracker,
}

impl<'a> PushHelper<'a> {
    pub fn new(repo: &'a git2::Repository, tracker: &'a tracker::Tracker) -> PushHelper<'a> {
        PushHelper {
            queue: VecDeque::new(),
            repo: repo,
            tracker: tracker,
        }
    }

    pub fn push(&mut self, hash: git2::Oid) -> Result<(), Error>{
        self.queue.push_back(hash);
        self.push_queue()
    }

    // push each of the objects in the queue into IPFS (as IPLD).
    fn push_queue(&mut self) -> Result<(), Error> {
        let api = ipfs_api::Shell::new_local().map_err(Error::ApiError)?;
        while let Some(oid) = self.queue.pop_front() {
            debug!("    pushing oid = {}", oid);

            if self.tracker.has_entry(oid.as_bytes())? {
                debug!("    already have this oid, skipping");
                continue;
            }

            let obj_bytes = self.push_object(oid, &api)?;

            self.tracker.add_entry(oid.as_bytes())?;

            self.enqueue_links(&obj_bytes)?;
        }
        Ok(())
    }

    // Push git object into ipfs, returning the vector of bytes of the raw git
    // object.
    fn push_object(&mut self, oid: git2::Oid, api: &ipfs_api::Shell) -> Result<Vec<u8>, Error> {
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
        api.dag_put(&full_obj, "raw", "git").map_err(Error::ApiError)?;
        Ok(full_obj)
    }

    fn enqueue_links(&mut self, obj_bytes: &[u8]) -> Result<(), Error> {
        let node = ipld_git::parse_object(obj_bytes).map_err(Error::IpldGitError)?;

        for link in node.links() {
            let link_multihash = multihash::decode(&link.cid.hash)?;
            if self.tracker.has_entry(link_multihash.digest)? {
                continue;
            }
            self.queue.push_back(git2::Oid::from_bytes(link_multihash.digest)?)
        }
        Ok(())
    }
}
