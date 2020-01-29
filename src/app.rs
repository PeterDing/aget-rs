use std::{env, path::Path};

#[cfg(windows)]
use ansi_term::enable_ansi_support;

use clap::ArgMatches;

use percent_encoding::percent_decode;

use awc::http::Uri;

use crate::{
    clap_app::build_app,
    common::AGET_EXT,
    error::{ArgError, Result},
    util::{escape_nonascii, LiteralSize},
};

#[derive(Debug, Clone)]
pub struct Config {
    pub(crate) uri: String,
    pub(crate) method: String,
    pub(crate) headers: Vec<String>,
    pub(crate) data: Option<String>,
    pub(crate) path: String,
    pub(crate) concurrency: u64,
    pub(crate) chunk_length: u64,
    pub(crate) timeout: u64,
    pub(crate) max_retries: u32,
    pub(crate) retry_wait: u64,
    pub(crate) debug: bool,
    pub(crate) quiet: bool,
}

impl Config {
    pub fn new(
        uri: String,
        method: String,
        headers: Vec<String>,
        data: Option<String>,
        path: String,
        concurrency: u64,
        timeout: u64,
        chunk_length: u64,
        max_retries: u32,
        retry_wait: u64,
        debug: bool,
        quiet: bool,
    ) -> Config {
        Config {
            uri,
            method,
            headers,
            data,
            path,
            concurrency,
            timeout,
            chunk_length,
            max_retries,
            retry_wait,
            debug,
            quiet,
        }
    }
}

pub struct App {
    pub matches: ArgMatches<'static>,
}

impl App {
    pub fn new() -> App {
        #[cfg(windows)]
        let _ = enable_ansi_support();

        App {
            matches: Self::matches(),
        }
    }

    fn matches() -> ArgMatches<'static> {
        let args = env::args();
        let matches = build_app().get_matches_from(args);
        matches
    }

    pub fn config(&self) -> Result<Config, ArgError> {
        // uri
        let uri = self.matches.value_of("URL").map(escape_nonascii).unwrap();

        // path
        let path = if let Some(path) = self.matches.value_of("out") {
            path.to_string()
        } else {
            let uri = &uri.parse::<Uri>()?;
            let path = Path::new(uri.path());
            if let Some(file_name) = path.file_name() {
                percent_decode(file_name.to_str().unwrap().as_bytes())
                    .decode_utf8()
                    .unwrap()
                    .to_string()
            } else {
                return Err(ArgError::NoFilename);
            }
        };

        // check status of task
        let path_ = Path::new(&path);
        let mut file_name = path_.file_name().unwrap().to_os_string();
        file_name.push(AGET_EXT);
        let mut aget_path = path_.to_path_buf();
        aget_path.set_file_name(file_name);
        if path_.is_dir() {
            return Err(ArgError::PathIsDirectory);
        }
        if path_.exists() && !aget_path.as_path().exists() {
            return Err(ArgError::FileExists);
        }

        let path = path.to_string();

        // data
        let data = if let Some(data) = self.matches.value_of("data") {
            Some(data.to_string())
        } else {
            None
        };

        // method
        let method = if let Some(method) = self.matches.value_of("method") {
            method.to_string()
        } else {
            if data.is_some() {
                "POST".to_owned()
            } else {
                "GET".to_owned()
            }
        };

        // headers
        let headers = if let Some(headers) = self.matches.values_of("header") {
            headers.map(String::from).collect::<Vec<String>>()
        } else {
            Vec::new()
        };

        // concurrency
        let concurrency = if let Some(concurrency) = self.matches.value_of("concurrency") {
            concurrency.parse::<u64>()?
        } else {
            10
        };

        // chunk length
        let chunk_length = if let Some(chunk_length) = self.matches.value_of("chunk-length") {
            chunk_length.literal_size()?
        } else {
            1024 * 500 // 500k
        };

        // timeout
        let timeout = if let Some(timeout) = self.matches.value_of("timeout") {
            timeout.parse::<u64>()?
        } else {
            10
        };

        // maximum retries
        let max_retries = if let Some(max_retries) = self.matches.value_of("max_retries") {
            max_retries.parse::<u32>()?
        } else {
            5
        };

        let retry_wait = if let Some(retry_wait) = self.matches.value_of("retry_wait") {
            retry_wait.parse::<u64>()?
        } else {
            5
        };

        let debug = self.matches.is_present("debug");

        let quiet = self.matches.is_present("quiet");

        Ok(Config::new(
            uri,
            method,
            headers,
            data,
            path,
            concurrency,
            timeout,
            chunk_length,
            max_retries,
            retry_wait,
            debug,
            quiet,
        ))
    }
}

fn test_escape_nonascii() {
    let s = ":ss/s  来；】/ 【【 ? 是的 & 水电费=45 进来看";
    println!("{}", s);
    println!("{}", escape_nonascii(s));
}
