#![allow(unused_variables)]
#![allow(dead_code)]
#![recursion_limit = "256"]

use std::{process::exit, thread, time};

use actix_rt::{spawn, System};

#[macro_use]
mod util;

mod app;
mod chunk;
mod clap_app;
mod common;
mod core;
mod error;
mod printer;
mod request;
mod store;
mod task;

use crate::{
    app::App,
    core::CoreProcess,
    util::{DEBUG, QUIET},
};

static mut SUCCESS: bool = false;

fn main() {
    let app = App::new();
    match app.config() {
        Ok(config) => {
            // set verbose
            unsafe {
                DEBUG = config.debug;
                QUIET = config.quiet;
            }

            debug!("Input configuration", &config);

            let retry_wait = config.retry_wait;
            let max_retries = config.max_retries;
            for i in 0..(max_retries + 1) {
                if i > 0 {
                    print_err!("!!! Retry", i);
                    thread::sleep(time::Duration::from_secs(retry_wait));
                }

                let sys = System::new("Aget");

                debug!("Make CoreProcess task");

                let config_ = config.clone();
                spawn(async move {
                    let core_process = CoreProcess::new(config_);
                    if let Ok(mut core_fut) = core_process {
                        let result = core_fut.run().await;
                        if let Err(err) = result {
                            print_err!("core_fut fails", err);
                        } else {
                            unsafe {
                                SUCCESS = true;
                            }
                        }
                    } else {
                        print_err!("core_fut error", "");
                        exit(1);
                    }
                    debug!("main done");
                    System::current().stop();
                });

                if let Err(err) = sys.run() {
                    print_err!("System error", err);
                } else {
                    // check task state
                    unsafe {
                        if SUCCESS {
                            break;
                        }
                    }
                }

                debug!("!!! Can't be here");
            }
        }
        Err(err) => {
            print_err!("app config fails", err);
            exit(1);
        }
    }
}
