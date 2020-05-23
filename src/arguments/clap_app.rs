use clap::{crate_name, crate_version, App as ClapApp, AppSettings, Arg};

pub fn build_app<'a>() -> ClapApp<'a, 'a> {
    ClapApp::new(crate_name!())
        .version(crate_version!())
        .global_setting(AppSettings::ColoredHelp)
        .global_setting(AppSettings::DeriveDisplayOrder)
        .global_setting(AppSettings::UnifiedHelpMessage)
        .global_setting(AppSettings::HidePossibleValuesInHelp)
        .about("Aget - Asynchronous Downloader")
        .arg(
            Arg::with_name("URL")
                .help("URL to request.")
                .multiple(false)
                .required(true)
                .empty_values(false),
        )
        .arg(
            Arg::with_name("method")
                .short("X")
                .long("method")
                .help("Request method, e.g. GET, POST.")
                .default_value("GET")
                .multiple(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("header")
                .short("H")
                .long("header")
                .help("Request headers, e.g. -H \"User-Agent: aget\".")
                .multiple(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("data")
                .short("d")
                .long("data")
                .help("Request with POST method with the data, e.g. -d \"a=b\".")
                .multiple(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("out")
                .short("o")
                .long("out")
                .help("The path of output for the request e.g. -o \"/path/to/file\".")
                .multiple(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("concurrency")
                .short("s")
                .long("concurrency")
                .help("The number of concurrency request e.g. -s 10")
                .default_value("10")
                .multiple(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("chunk-length")
                .short("k")
                .long("chunk-length")
                .help("The interval length of each concurrent request e.g. -k 100k")
                .default_value("1m")
                .multiple(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("proxy")
                .long("proxy")
                .help("proxy (http/https/socks4/socks5) e.g. -p http://localhost:1024")
                .multiple(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("timeout")
                .short("t")
                .long("timeout")
                .help("Timeout(seconds) of request")
                .multiple(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("max-retries")
                .long("max-retries")
                .help("The maximum times of retring")
                .default_value("5")
                .multiple(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("retry-wait")
                .long("retry-wait")
                .help("The seconds between retries")
                .default_value("0")
                .multiple(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("type")
                .long("type")
                .default_value("http")
                .multiple(false)
                .help("Task type, http/m3u8"),
        )
        .arg(
            Arg::with_name("debug")
                .long("debug")
                .help("Debug output. Print all trackback for debugging"),
        )
        .arg(
            Arg::with_name("quiet")
                .long("quiet")
                .help("Quiet mode. Don't show progress bar and task information. But still show the error information"),
        )
        .help_message("Print this help message.")
        .version_message("Show version information.")
}
