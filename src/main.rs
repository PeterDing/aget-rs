#![allow(unused_variables)]
#![allow(dead_code)]

use std::{process::exit, thread, time};

use futures::Future;

use actix::{spawn, System};

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
                let core_process = CoreProcess::new(config.clone());

                match core_process {
                    Ok(core_fut) => {
                        spawn(
                            core_fut
                                .map_err(|err| {
                                    print_err!("core_fut fails", err);
                                    System::current().stop();
                                })
                                .map(|_| unsafe {
                                    SUCCESS = true;
                                }),
                        );
                    }
                    Err(err) => {
                        print_err!("core_fut error", err);
                        exit(1);
                    }
                }

                sys.run();

                // check task state
                unsafe {
                    if SUCCESS {
                        return;
                    }
                }
            }
        }
        Err(err) => {
            print_err!("app config fails", err);
            exit(1);
        }
    }
}
