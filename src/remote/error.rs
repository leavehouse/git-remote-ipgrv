use git2;
use hex;
use ipld_git;
use lmdb;
use multihash;
use std::env;
use std::io;

use ipfs_api;

#[derive(Debug)]
pub enum Error {
    ApiError(ipfs_api::Error),
    EnvVarError(env::VarError),
    FromHexError(hex::FromHexError),
    Git2Error(git2::Error),
    IoError(io::Error),
    LmdbError(lmdb::Error),
    IpldGitError(ipld_git::Error),
    MultihashError(multihash::Error),
    InvalidCommand(String),
}

impl From<env::VarError> for Error {
    fn from(e: env::VarError) -> Self {
        Error::EnvVarError(e)
    }
}

impl From<hex::FromHexError> for Error {
    fn from(e: hex::FromHexError) -> Self {
        Error::FromHexError(e)
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

impl From<multihash::Error> for Error {
    fn from(e: multihash::Error) -> Self {
        Error::MultihashError(e)
    }
}
