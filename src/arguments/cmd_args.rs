use std::{
    env, fmt,
    path::{Path, PathBuf},
    time::Duration,
};

#[cfg(windows)]
use ansi_term::enable_ansi_support;

use clap::{crate_version, ArgMatches};

use percent_encoding::percent_decode;

use crate::{
    arguments::clap_app::build_app,
    common::{
        bytes::bytes_type::BytesMut,
        character::escape_nonascii,
        errors::Error,
        liberal::ParseLiteralNumber,
        net::{net::parse_headers, Method, Uri},
        tasks::TaskType,
    },
    features::args::Args,
};

pub struct CmdArgs {
    matches: ArgMatches<'static>,
}

impl CmdArgs {
    pub fn new() -> CmdArgs {
        #[cfg(windows)]
        let _ = enable_ansi_support();

        let args = env::args();
        let inner = build_app();
        let matches = inner.get_matches_from(args);
        CmdArgs { matches }
    }
}

impl Args for CmdArgs {
    /// Path of output
    fn output(&self) -> PathBuf {
        if let Some(path) = self.matches.value_of("out") {
            PathBuf::from(path)
        } else {
            let uri = self.uri();
            let path = Path::new(uri.path());
            if let Some(file_name) = path.file_name() {
                PathBuf::from(
                    percent_decode(file_name.to_str().unwrap().as_bytes())
                        .decode_utf8()
                        .unwrap()
                        .to_string(),
                )
            } else {
                panic!("{:?}", Error::NoFilename);
            }
        }
    }

    /// Request method for http
    fn method(&self) -> Method {
        if let Some(method) = self.matches.value_of("method") {
            match method.to_uppercase().as_str() {
                "GET" => Method::GET,
                "POST" => Method::POST,
                _ => panic!(format!(
                    "{:?}",
                    Error::UnsupportedMethod(method.to_string())
                )),
            }
        } else {
            if self.data().is_some() {
                Method::POST
            } else {
                Method::GET
            }
        }
    }

    /// The uri of a task
    fn uri(&self) -> Uri {
        self.matches
            .value_of("URL")
            .map(escape_nonascii)
            .unwrap()
            .parse()
            .unwrap()
    }

    /// The data for http post request
    fn data(&self) -> Option<BytesMut> {
        self.matches.value_of("data").map(|d| BytesMut::from(d))
    }

    /// Request headers
    fn headers(&self) -> Vec<(String, String)> {
        let mut headers = if let Some(headers) = self.matches.values_of("header") {
            parse_headers(headers)
                .unwrap()
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<Vec<(String, String)>>()
        } else {
            vec![]
        };

        // Add default headers
        let default_headers = vec![(
            "user-agent".to_owned(),
            format!("aget/{}", crate_version!()),
        )];
        for (dk, dv) in default_headers {
            let mut has = false;
            for (k, _) in headers.iter() {
                if k.to_lowercase() == dk {
                    has = true;
                    break;
                }
            }
            if !has {
                headers.push((dk, dv));
            }
        }
        headers
    }

    /// Set proxy througth arg or environment variable
    ///
    /// The environment variables can be:
    /// http_proxy [protocol://]<host>[:port]
    ///        Sets the proxy server to use for HTTP.
    ///
    /// HTTPS_PROXY [protocol://]<host>[:port]
    ///        Sets the proxy server to use for HTTPS.
    ///
    /// ALL_PROXY [protocol://]<host>[:port]
    ///        Sets the proxy server to use if no protocol-specific proxy is set.
    ///
    /// Protocols:
    /// http://
    ///        an HTTP proxy
    /// https://
    ///        as HTTPS proxy
    /// socks4://
    /// socks4a://
    /// socks5://
    /// socks5h://
    ///        as SOCKS proxy
    fn proxy(&self) -> Option<String> {
        let p = self.matches.value_of("proxy").map(|i| i.to_string());
        if p.is_some() {
            return p;
        }

        if let Ok(p) = env::var("http_proxy") {
            return Some(p);
        }

        if let Ok(p) = env::var("HTTPS_PROXY") {
            return Some(p);
        }

        if let Ok(p) = env::var("ALL_PROXY") {
            return Some(p);
        }

        None
    }

    // Set request timeout
    //
    // Request timeout is the total time before a response must be received.
    // Default value is 5 seconds.
    fn timeout(&self) -> Duration {
        Duration::from_secs(
            self.matches
                .value_of("timeout")
                .map(|i| i.parse::<u64>().unwrap())
                .unwrap_or({
                    match self.task_type() {
                        TaskType::HTTP => 60,
                        TaskType::M3U8 => 30,
                    }
                }),
        )
    }

    fn dns_timeout(&self) -> Duration {
        Duration::from_secs(
            self.matches
                .value_of("dns-timeout")
                .map(|i| i.parse::<u64>().unwrap())
                .unwrap_or(10),
        )
    }

    fn keep_alive(&self) -> Duration {
        match self.task_type() {
            TaskType::HTTP => Duration::from_secs(60),
            TaskType::M3U8 => Duration::from_secs(10),
        }
    }

    fn lifetime(&self) -> Duration {
        match self.task_type() {
            TaskType::HTTP => Duration::from_secs(0),
            TaskType::M3U8 => Duration::from_secs(0),
        }
    }

    // Always return `true`
    fn disable_redirects(&self) -> bool {
        true
    }

    /// The number of concurrency
    fn concurrency(&self) -> u64 {
        self.matches
            .value_of("concurrency")
            .map(|i| i.parse::<u64>().unwrap())
            .unwrap_or(10)
    }

    /// The chunk size of each concurrency for http task
    fn chunk_size(&self) -> u64 {
        self.matches
            .value_of("chunk-size")
            .map(|i| i.literal_number().unwrap())
            .unwrap_or(1024 * 500) // 500k
    }

    /// The number of retry of a task, default is 5
    fn retries(&self) -> u64 {
        self.matches
            .value_of("retries")
            .map(|i| i.parse::<u64>().unwrap())
            .unwrap_or(5)
    }

    /// The internal of each retry, default is zero
    fn retry_wait(&self) -> u64 {
        self.matches
            .value_of("retry_wait")
            .map(|i| i.parse::<u64>().unwrap())
            .unwrap_or(0)
    }

    /// Task type
    fn task_type(&self) -> TaskType {
        self.matches
            .value_of("type")
            .map(|i| match i.to_lowercase().as_str() {
                "auto" => {
                    let uri = self.uri();
                    if uri.path().to_lowercase().ends_with(".m3u8") {
                        TaskType::M3U8
                    } else {
                        TaskType::HTTP
                    }
                }
                "http" => TaskType::HTTP,
                "m3u8" => TaskType::M3U8,
                _ => panic!(format!("{:?}", Error::UnsupportedTask(i.to_string()))),
            })
            .unwrap_or(TaskType::HTTP)
    }

    /// To debug mode, if it returns true
    fn debug(&self) -> bool {
        self.matches.is_present("debug")
    }

    /// To quiet mode, if it return true
    fn quiet(&self) -> bool {
        self.matches.is_present("quiet")
    }
}

impl fmt::Debug for CmdArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CmdArgs")
            .field("output", &self.output())
            .field("method", &self.method())
            .field("uri", &self.uri())
            .field("data", &self.data())
            .field("headers", &self.headers())
            .field("proxy", &self.proxy())
            .field("timeout", &self.timeout())
            .field("dns_timeout", &self.dns_timeout())
            .field("keep_alive", &self.keep_alive())
            .field("lifetime", &self.lifetime())
            .field("disable_redirects", &self.disable_redirects())
            .field("concurrency", &self.concurrency())
            .field("chunk_size", &self.chunk_size())
            .field("retries", &self.retries())
            .field("retry_wait", &self.retry_wait())
            .field("task_type", &self.task_type())
            .field("debug", &self.debug())
            .field("quiet", &self.quiet())
            .finish()
    }
}
