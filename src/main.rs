#![allow(dead_code)]

#[macro_use]
mod common;

mod app;
mod arguments;
mod features;

use std::{thread, time::Duration};

use app::core::{http::HttpHandler, m3u8::m3u8::M3u8Handler};
use arguments::cmd_args::CmdArgs;
use common::{
    debug::{DEBUG, QUIET},
    tasks::TaskType,
};
use features::{args::Args, running::Runnable};

fn main() {
    let cmdargs = CmdArgs::new();

    // Set debug
    if cmdargs.debug() {
        unsafe {
            DEBUG = true;
        }
        debug!("Args", cmdargs);
    }

    // Set quiet
    if cmdargs.quiet() {
        unsafe {
            QUIET = true;
        }
    }

    debug!("Main: begin");

    let tasktype = cmdargs.task_type();
    for i in 0..cmdargs.retries() + 1 {
        if i != 0 {
            println!("Retry {}", i);
        }

        let result = match tasktype {
            TaskType::HTTP => {
                let mut httphandler = HttpHandler::new(&cmdargs).unwrap();
                httphandler.run()
            }
            TaskType::M3U8 => {
                let mut m3u8handler = M3u8Handler::new(&cmdargs).unwrap();
                m3u8handler.run()
            }
        };

        if let Err(err) = result {
            print_err!("Error", err);
            // Retry
            let retrywait = cmdargs.retry_wait();
            thread::sleep(Duration::from_secs(retrywait));
            continue;
        } else {
            // Success
            break;
        }
    }
}
