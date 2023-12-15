use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct AgetCli {
    pub url: String,

    #[clap(
        short,
        long,
        default_value_t = String::from("GET"),
        help = "Request method, e.g. GET, POST"
    )]
    pub method: String,

    #[clap(
        short = 'H',
        long,
        help = r#"Request headers, e.g. -H "User-Agent: aget""#
    )]
    pub header: Option<Vec<String>>,

    #[clap(
        short,
        long,
        help = r#"Request with POST method with the data, e.g. -d "a=b""#
    )]
    pub data: Option<String>,

    #[clap(
        short = 's',
        long,
        help = "The number of concurrency request [default: 10]"
    )]
    pub concurrency: Option<u64>,

    #[clap(
        short = 'k',
        long,
        help = "The number ofinterval length of each concurrent request [default: '50m']"
    )]
    pub chunk_size: Option<String>,

    #[clap(short, long, help = "Timeout(seconds) of request [default: 60]")]
    pub timeout: Option<u64>,

    #[clap(long, help = "DNS Timeout(seconds) of request [default: 10]")]
    pub dns_timeout: Option<u64>,

    #[clap(long, help = "The maximum times of retring [default: 5]")]
    pub retries: Option<u64>,

    #[clap(long, help = "The seconds between retries [default: 0]")]
    pub retry_wait: Option<u64>,

    #[clap(
        long = "proxy",
        name = "PROXY",
        help = "[protocol://]host[:port] Use this proxy"
    )]
    pub proxy: Option<String>,

    #[clap(
        long = "type",
        name = "TYPE",
        default_value = "auto",
        help = "Task type, auto/http/m3u8"
    )]
    pub tp: String,

    #[clap(long, help = "Debug output. Print all trackback for debugging")]
    pub debug: bool,

    #[clap(
        long,
        help = "Quiet mode. Don't show progress bar and task information. But still show the error information"
    )]
    pub quiet: bool,

    #[clap(
        short,
        long,
        help = r#"The path of output for the request e.g. -o "/path/to/file""#
    )]
    pub out: Option<String>,
}
