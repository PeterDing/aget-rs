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

    #[clap(short, long, help = r#"Request headers, e.g. -H "User-Agent: aget""#)]
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

// pub fn build_app<'a>() -> ClapApp<'a, 'a> {
//     ClapApp::new(crate_name!())
//         .version(crate_version!())
//         .global_setting(AppSettings::ColoredHelp)
//         .global_setting(AppSettings::DeriveDisplayOrder)
//         .global_setting(AppSettings::UnifiedHelpMessage)
//         .global_setting(AppSettings::HidePossibleValuesInHelp)
//         .about("Aget - Asynchronous Downloader")
//         .arg(
//             Arg::with_name("URL")
//                 .required(true)
//                 .empty_values(false)
//                 .multiple(false)
//                 .help("URL to request.")
//         )
//         .arg(
//             Arg::with_name("method")
//                 .short("X")
//                 .long("method")
//                 .default_value("GET")
//                 .multiple(false)
//                 .takes_value(true)
//                 .help("Request method, e.g. GET, POST.")
//         )
//         .arg(
//             Arg::with_name("header")
//                 .short("H")
//                 .long("header")
//                 .multiple(true)
//                 .takes_value(true)
//                 .help("Request headers, e.g. -H \"User-Agent: aget\".")
//         )
//         .arg(
//             Arg::with_name("data")
//                 .short("d")
//                 .long("data")
//                 .multiple(false)
//                 .takes_value(true)
//                 .help("Request with POST method with the data, e.g. -d \"a=b\".")
//         )
//         .arg(
//             Arg::with_name("out")
//                 .short("o")
//                 .long("out")
//                 .multiple(false)
//                 .takes_value(true)
//                 .help("The path of output for the request e.g. -o \"/path/to/file\".")
//         )
//         .arg(
//             Arg::with_name("concurrency")
//                 .short("s")
//                 .long("concurrency")
//                 .multiple(false)
//                 .takes_value(true)
//                 .help("The number of concurrency request [default: 10]")
//         )
//         .arg(
//             Arg::with_name("chunk-size")
//                 .short("k")
//                 .long("chunk-size")
//                 .multiple(false)
//                 .takes_value(true)
//                 .help("The interval length of each concurrent request [default: '50m']")
//         )
//         // `awc` does not support proxy
//         // .arg(
//         //     Arg::with_name("proxy")
//         //         .long("proxy")
//         //         .multiple(false)
//         //         .takes_value(true)
//         //         .help("proxy (http/https/socks4/socks5) e.g. -p http://localhost:1024")
//         // )
//         //
//         // Request timeout is the total time before a response must be received.
//         .arg(
//             Arg::with_name("timeout")
//                 .short("t")
//                 .long("timeout")
//                 .multiple(false)
//                 .takes_value(true)
//                 .help("Timeout(seconds) of request [default: 60]")
//         )
//         .arg(
//             Arg::with_name("dns-timeout")
//                 .short("n")
//                 .long("dns-timeout")
//                 .multiple(false)
//                 .takes_value(true)
//                 .help("DNS Timeout(seconds) of request [default: 10]")
//         )
//         .arg(
//             Arg::with_name("retries")
//                 .long("retries")
//                 .multiple(false)
//                 .takes_value(true)
//                 .help("The maximum times of retring [default: 5]")
//         )
//         .arg(
//             Arg::with_name("retry-wait")
//                 .long("retry-wait")
//                 .multiple(false)
//                 .takes_value(true)
//                 .help("The seconds between retries [default: 0]")
//         )
//         .arg(
//             Arg::with_name("type")
//                 .long("type")
//                 .default_value("auto")
//                 .multiple(false)
//                 .takes_value(true)
//                 .help("Task type, auto/http/m3u8 [default: auto]")
//         )
//         .arg(
//             Arg::with_name("debug")
//                 .long("debug")
//                 .help("Debug output. Print all trackback for debugging")
//         )
//         .arg(
//             Arg::with_name("quiet")
//                 .long("quiet")
//                 .help("Quiet mode. Don't show progress bar and task information. But still show
// the error information"),         )
//         .help_message("Print this help message.")
//         .version_message("Show version information.")
// }
