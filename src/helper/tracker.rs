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

    // `hash` should be the SHA-1 digest, a 20-byte-slice
    pub fn has_entry(&self, hash: &[u8]) -> Result<bool, lmdb::Error> {
        let env = self.db.env();
        let txn = lmdb::ReadTransaction::new(env)?;
        let access = txn.access();
        match access.get::<_, ()>(&self.db, hash) {
            Ok(_) => Ok(true),
            Err(e) => match e {
                lmdb::Error::Code(lmdb::error::NOTFOUND) => Ok(false),
                _ => Err(e),
            }
        }
    }

    pub fn add_entry(&self, hash: &[u8]) -> Result<(), lmdb::Error> {
        let env = self.db.env();
        let txn = lmdb::WriteTransaction::new(env)?;

        {
            let mut access = txn.access();
            access.put(&self.db, hash, &(), lmdb::put::Flags::empty())?;
        }

        txn.commit()
    }
}
