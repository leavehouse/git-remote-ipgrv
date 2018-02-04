use home;
use reqwest;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

const IPFS_DIR_ENV_VAR: &'static str = "IPFS_PATH";
const IPFS_DATA_FOLDER_NAME: &'static str = ".ipfs";
const API_FILE_NAME: &'static str = "api";

type Error = String;

fn default_ipfs_dir() -> Result<PathBuf, Error> {
    let mut home_dir = match home::home_dir() {
        None => return Err("Could not determine home directory".to_string()),
        Some(p) => p,
    };
    home_dir.push(IPFS_DATA_FOLDER_NAME);
    Ok(home_dir)
}

fn ipfs_data_dir_path() -> Result<PathBuf, Error> {
    match env::var(IPFS_DIR_ENV_VAR) {
        Ok(ref val) if val.len() > 0 => Ok(PathBuf::from(val)),
        _ => default_ipfs_dir(),
    }
}

pub struct Shell {
    client: reqwest::Client,
    addr: String,
}

impl Shell {
    pub fn new(addr: String) -> Shell {
        Shell {
            client: reqwest::Client::new(),
            addr: addr,
        }
    }

    fn new_local() -> Result<Shell, Error> {
        let client = reqwest::Client::new();
        let mut api_path = ipfs_data_dir_path()?;
        api_path.push(API_FILE_NAME);

        if !api_path.exists() {
            return Err(format!("API file at {:?} does not exist", api_path));
        }

        let mut api_file = File::open(api_path)
            .map_err(|e| format!("Error opening file: {}", e))?;

        let mut addr = String::new();
        api_file.read_to_string(&mut addr)
                .map_err(|e| format!("Error reading file: {}", e))?;

        Ok(Shell::new(addr.trim().to_string()))
    }
}


pub fn dag_put(data: &[u8], input_enc: &str, format: &str) -> Result<(), Error> {
    //let res = client.post(
    unimplemented!()
}
