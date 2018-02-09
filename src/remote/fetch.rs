use flate2;
use flate2::write::ZlibEncoder;
use hex;
use ipld_git;
use multihash;
use std::collections::VecDeque;
use std::env;
use std::fs::{File, create_dir_all};
use std::io::Write;
use std::path::{Path, PathBuf};

use ipfs_api;
use super::Error;
use super::tracker;

pub struct FetchHelper<'a> {
    queue: VecDeque<String>,
    tracker: &'a tracker::Tracker,
}

impl<'a> FetchHelper<'a> {
    pub fn new(tracker: &'a tracker::Tracker) -> FetchHelper {
        FetchHelper {
            queue: VecDeque::new(),
            tracker: tracker,
        }
    }

    // `hash` is a hex representation of the hash being fetched
    pub fn fetch(&mut self, hash: String) -> Result<(), Error>{
        debug!("    FetchHelper::fetch, hash = {}", hash);
        self.queue.push_back(hash);
        self.fetch_queue()
    }

    // fetch each of the objects in the queue from IPFS
    fn fetch_queue(&mut self) -> Result<(), Error> {
        let api = ipfs_api::Shell::new_local().map_err(Error::ApiError)?;
        while let Some(hash) = self.queue.pop_front() {
            debug!("    fetching hash = {}", hash);

            let mut base_dir = PathBuf::from(env::var("GIT_DIR")?);
            base_dir.push("objects");
            let obj_file = prepare_object_path(&mut base_dir, &hash)?;

            if obj_file.exists() {
                continue;
            }

            let hash_bytes = hex::decode(hash)?;
            let obj_cid = ipld_git::util::sha1_to_cid(&hash_bytes)
                .map_err(Error::IpldGitError)?;
            debug!("    the corresponding cid = {}", obj_cid.to_string());
            let obj_bytes = api.block_get(&obj_cid.to_string())
                .map_err(Error::ApiError)?;

            // add all linked objects to the queue to be fetched next
            self.enqueue_links(&obj_bytes)?;


            let mut enc = ZlibEncoder::new(Vec::new(),
                                           flate2::Compression::default());
            enc.write(&obj_bytes)?;
            let compressed_bytes = enc.finish()?;

            let mut f = File::create(obj_file)?;
            f.write(&compressed_bytes)?;

            self.tracker.add_entry(&hash_bytes)?;
        }
        Ok(())
    }

    fn enqueue_links(&mut self, obj_bytes: &[u8]) -> Result<(), Error> {
        let node = ipld_git::parse_object(obj_bytes).map_err(Error::IpldGitError)?;

        for link in node.links() {
            let link_multihash = multihash::decode(&link.cid.hash)?;
            self.queue.push_back(hex::encode(link_multihash.digest))
        }
        Ok(())
    }
}

// each object stored in git object database is identified by the sha-1
// hash of its data. Objects are stored as individual files. The files are
// sharded into separate directories based on the first byte of the hash
// (e.g. object with hash 'fa937...' is stored at '.git/objects/fa/937...')
fn prepare_object_path<'a>(path: &'a mut PathBuf, hash: &str) -> Result<&'a Path, Error> {
    debug!("      prepare_object_path start, hash = {}", hash);
    let hash_dir = &hash[..2];
    path.push(hash_dir);
    create_dir_all(&path)?;
    path.push(&hash[2..]);
    Ok(path.as_path())
}
