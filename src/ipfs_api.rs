use home;
use reqwest;
use url;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::PathBuf;

const IPFS_DIR_ENV_VAR: &'static str = "IPFS_PATH";
const IPFS_DATA_FOLDER_NAME: &'static str = ".ipfs";
const API_FILE_NAME: &'static str = "api";

pub type Error = String;

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
    url: String, // The URL of the API endpoint
}

impl Shell {
    pub fn new(addr: &str) -> Result<Shell, Error> {
        let parts = addr[1..].split('/').collect::<Vec<_>>();
        if parts.len() != 4 || parts[0] != "ip4" || parts[2] != "tcp" {
            return Err("Shell::new takes a multiaddr of the form \
                        '/ip4/<ip>/tcp/<port>'".to_string())
        }
        Ok(Shell {
            client: reqwest::Client::new(),
            url: format!("http://{}:{}", parts[1], parts[3]),
        })
    }

    pub fn new_local() -> Result<Shell, Error> {
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

        Shell::new(addr.trim())
    }

    pub fn dag_put(&self, data: &[u8], input_enc: &str, format: &str) -> Result<(), Error> {
        use reqwest::multipart::{Part, Form};
        use reqwest::header::{ContentDisposition, DispositionParam, DispositionType,
                              TransferEncoding};

        // TODO: construct equivalent of `Request` in go-ipfs-api
        let command = "dag/put";
        let base_url = format!("{}/api/v0/{}", self.url, command);
        let params = &[("input-enc", input_enc), ("format", format)];
        let request_url = url::Url::parse_with_params(&base_url, params)
            .map_err(|e| format!("Error building request URL: {}", e))?;


        let part = Part::reader(Cursor::new(data.to_vec()))
            .mime(reqwest::mime::APPLICATION_OCTET_STREAM);
        let form = Form::new().part("", part);
        let mut req_builder = self.client.post(request_url);

        req_builder.header(TransferEncoding::chunked());

        req_builder.multipart(form);
        let resp = req_builder.send()
            .map_err(|e| format!("Error sending request: {}", e))?;

        unimplemented!()
    }
}
