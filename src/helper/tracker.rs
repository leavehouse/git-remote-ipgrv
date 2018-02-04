use lmdb;

pub struct Tracker {
    db: lmdb::Database<'static>,
}

impl Tracker {
    pub fn new(path: &str) -> Result<Tracker, lmdb::Error> {
        let env = unsafe {
            lmdb::EnvBuilder::new()?
                .open(path, lmdb::open::Flags::empty(), 0o600)?
        };
        // We use so-called "owned mode", where the database owns the environment
        let db = lmdb::Database::open(
            env, None, &lmdb::DatabaseOptions::defaults()).unwrap();
        Ok(Tracker { db: db })
    }
}
