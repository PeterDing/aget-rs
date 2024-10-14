#![allow(dead_code)]

use std::{process::exit, str::FromStr, thread, time::Duration};

use time::{macros::format_description, UtcOffset};
use tracing_subscriber::fmt::time::OffsetTime;

use aget::{
    app::core::{bt::BtHandler, http::HttpHandler, m3u8::M3u8Handler},
    arguments::cmd_args::CmdArgs,
    common::tasks::TaskType,
    features::{args::Args, running::Runnable},
};

fn main() {
    let cmdargs = CmdArgs::new();
    let log_level = if cmdargs.debug() { "debug" } else { "error" };

    let app_name = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")).to_string();

    let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());
    let local_time = OffsetTime::new(
        UtcOffset::from_hms(8, 0, 0).unwrap(),
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:2]"),
    );

    let log_level = tracing::Level::from_str(log_level).unwrap();
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_max_level(log_level)
        .with_timer(local_time)
        .init();

    tracing::debug!("===== Aget-rs {}: begin =====", app_name);
    tracing::debug!("Args: {:?}", cmdargs);

    let tasktype = cmdargs.task_type();
    for i in 0..cmdargs.retries() + 1 {
        if i != 0 {
            println!("Retry {}", i);
        }

        let result = match tasktype {
            TaskType::HTTP => {
                let httphandler = HttpHandler::new(&cmdargs).unwrap();
                httphandler.run()
            }
            TaskType::M3U8 => {
                let m3u8handler = M3u8Handler::new(&cmdargs).unwrap();
                m3u8handler.run()
            }
            TaskType::BT => {
                let bthandler = BtHandler::new(&cmdargs);
                bthandler.run()
            }
        };

        if let Err(err) = result {
            tracing::error!("Error: {:?}", err);
            // Retry
            let retrywait = cmdargs.retry_wait();
            thread::sleep(Duration::from_secs(retrywait));
            continue;
        } else {
            // Success
            return;
        }
    }

    // All retries fail
    exit(1);
}
