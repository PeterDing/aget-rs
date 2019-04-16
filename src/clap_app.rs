use clap::{crate_name, crate_version, App as ClapApp, AppSettings, Arg};

pub fn build_app() -> ClapApp<'static, 'static> {
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
            Arg::with_name("concurrent")
                .short("s")
                .long("concurrent")
                .help("The number of concurrent request e.g. -s 10")
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
