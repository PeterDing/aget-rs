use std::{
    fmt,
    path::{Path, PathBuf},
    time::Duration,
};

use clap::Parser;

#[cfg(windows)]
use ansi_term::enable_ansi_support;

use percent_encoding::percent_decode;

use crate::{
    arguments::clap_cli::AgetCli,
    common::{
        character::escape_nonascii,
        errors::Error,
        liberal::ParseLiteralNumber,
        net::{net::parse_headers, Method, Uri},
        tasks::TaskType,
    },
    config::Config,
    features::args::Args,
};

const DEFAULT_HEADERS: [(&str, &str); 1] = [(
    "user-agent",
    concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")),
)];

pub struct CmdArgs {
    cli: AgetCli,
    config: Config,
}

impl CmdArgs {
    pub fn new() -> CmdArgs {
        #[cfg(windows)]
        let _ = enable_ansi_support();

        CmdArgs {
            cli: AgetCli::parse(),
            config: Config::new(),
        }
    }
}

impl Args for CmdArgs {
    /// Path of output
    fn output(&self) -> PathBuf {
        if let Some(path) = self.cli.out.clone() {
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
        if self.cli.data.is_some() {
            return Method::POST;
        }
        match self.cli.method.to_uppercase().as_str() {
            "GET" => Method::GET,
            "POST" => Method::POST,
            _ => panic!(
                "{:?}",
                Error::UnsupportedMethod(self.cli.method.to_string())
            ),
        }
    }

    /// The uri of a task
    fn uri(&self) -> Uri {
        escape_nonascii(&self.cli.url)
            .parse()
            .expect("URL is unvalidable")
    }

    /// The data for http post request
    fn data(&self) -> Option<&str> {
        self.cli.data.as_deref()
    }

    /// Request headers
    fn headers(&self) -> Vec<(&str, &str)> {
        let mut headers = if let Some(ref headers) = self.cli.header {
            let v = parse_headers(headers.iter().map(|h| h.as_str())).unwrap();
            v.into_iter().collect::<Vec<(&str, &str)>>()
        } else {
            vec![]
        };

        if let Some(config_headers) = &self.config.headers {
            for (uk, uv) in config_headers.iter() {
                let mut has = false;
                for (k, _) in headers.iter() {
                    if k.to_lowercase() == *uk {
                        has = true;
                        break;
                    }
                }
                if !has {
                    headers.push((uk, uv));
                }
            }
        }

        // Add default headers
        for (dk, dv) in DEFAULT_HEADERS {
            let mut has = false;
            for (k, _) in headers.iter() {
                if k.to_lowercase() == *dk {
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
    fn proxy(&self) -> Option<&str> {
        None
    }

    // Set request timeout
    //
    // Request timeout is the total time before a response must be received.
    // Default value is 5 seconds.
    fn timeout(&self) -> Duration {
        let timeout = match self.cli.timeout {
            Some(timeout) => timeout,
            None => match self.task_type() {
                TaskType::HTTP => self.config.timeout.unwrap_or(60),
                TaskType::M3U8 => self.config.timeout.unwrap_or(30),
            },
        };

        Duration::from_secs(timeout)
    }

    fn dns_timeout(&self) -> Duration {
        Duration::from_secs(self.cli.dns_timeout.unwrap_or(10))
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
        self.cli
            .concurrency
            .unwrap_or_else(|| self.config.concurrency.unwrap_or(10))
    }

    /// The chunk size of each concurrency for http task
    fn chunk_size(&self) -> u64 {
        self.cli
            .chunk_size
            .as_deref()
            .map(|i| i.literal_number().unwrap())
            .unwrap_or_else(|| {
                self.config
                    .chunk_size
                    .as_ref()
                    .map(|i| i.as_str().literal_number().unwrap())
                    .unwrap_or(1024 * 1024 * 50)
            }) // 50m
    }

    /// The number of retry of a task, default is 5
    fn retries(&self) -> u64 {
        self.cli
            .retries
            .unwrap_or_else(|| self.config.retries.unwrap_or(5))
    }

    /// The internal of each retry, default is zero
    fn retry_wait(&self) -> u64 {
        self.cli
            .retry_wait
            .unwrap_or_else(|| self.config.retry_wait.unwrap_or(0))
    }

    /// Task type
    fn task_type(&self) -> TaskType {
        match self.cli.tp.as_str() {
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
            _ => panic!("{:?}", Error::UnsupportedTask(self.cli.tp.clone())),
        }
    }

    /// To debug mode, if it returns true
    fn debug(&self) -> bool {
        self.cli.debug
    }

    /// To quiet mode, if it return true
    fn quiet(&self) -> bool {
        self.cli.quiet
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
