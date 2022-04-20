#![allow(dead_code)]

#[macro_use]
mod common;

mod app;
mod arguments;
mod config;
mod features;

use std::{process::exit, thread, time::Duration};

use app::core::{http::HttpHandler, m3u8::M3u8Handler};
use arguments::cmd_args::CmdArgs;
use common::tasks::TaskType;
use features::{args::Args, running::Runnable};

use tracing_bunyan_formatter::BunyanFormattingLayer;
use tracing_bunyan_formatter::JsonStorageLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Registry;

fn main() {
    let cmdargs = CmdArgs::new();
    let log_level = if cmdargs.debug() { "debug" } else { "error" };

    let app_name = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")).to_string();
    let (non_blocking_writer, _guard) = tracing_appender::non_blocking(std::io::stdout());
    let bunyan_formatting_layer = BunyanFormattingLayer::new(app_name, non_blocking_writer);
    let subscriber = Registry::default()
        .with(EnvFilter::new(log_level))
        .with(JsonStorageLayer)
        .with(bunyan_formatting_layer);
    tracing::subscriber::set_global_default(subscriber).unwrap();

    tracing::debug!("Main: begin");
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
