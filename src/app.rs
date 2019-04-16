use std::env;
use std::path::Path;

use actix_web::http::Uri;
#[cfg(windows)]
use ansi_term::enable_ansi_support;
use clap::ArgMatches;

use crate::clap_app::build_app;
use crate::common::AGET_EXT;
use crate::error::{ArgError, Result};
use crate::util::LiteralSize;

#[derive(Debug)]
pub struct Config {
    pub uri: String,
    pub method: String,
    pub headers: Vec<String>,
    pub data: Option<String>,
    pub path: String,
    pub concurrent: u64,
    pub chunk_length: u64,
    pub debug: bool,
    pub quiet: bool,
}

impl Config {
    pub fn new(
        uri: String,
        method: String,
        headers: Vec<String>,
        data: Option<String>,
        path: String,
        concurrent: u64,
        chunk_length: u64,
        debug: bool,
        quiet: bool,
    ) -> Config {
        Config {
            uri,
            method,
            headers,
            data,
            path,
            concurrent,
            chunk_length,
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
        let uri = self.matches.value_of("URL").unwrap();

        // path
        let path = if let Some(path) = self.matches.value_of("out") {
            path.to_string()
        } else {
            let uri = uri.parse::<Uri>()?;
            let path = Path::new(uri.path());
            if let Some(file_name) = path.file_name() {
                file_name.to_str().unwrap().to_string()
            } else {
                return Err(ArgError::NoFilename);
            }
        };

        let uri = uri.to_string();

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

        // concurrent
        let concurrent = if let Some(concurrent) = self.matches.value_of("concurrent") {
            concurrent.parse::<u64>()?
        } else {
            10
        };

        // chunk length
        let chunk_length =
            if let Some(chunk_length) = self.matches.value_of("chunk-length") {
                chunk_length.literal_size()?
            } else {
                // 500k
                1024 * 500
            };

        let debug = self.matches.is_present("debug");

        let quiet = self.matches.is_present("quiet");

        Ok(Config::new(
            uri,
            method,
            headers,
            data,
            path,
            concurrent,
            chunk_length,
            debug,
            quiet,
        ))
    }
}
