#![allow(unused_variables)]
#![allow(dead_code)]

use std::process::exit;

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

use crate::app::App;
use crate::core::CoreProcess;
use crate::util::{DEBUG, QUIET};

fn main() {
    let sys = System::new("Aget");
    let app = App::new();
    match app.config() {
        Ok(config) => {
            // set verbose
            unsafe {
                DEBUG = config.debug;
                QUIET = config.quiet;
            }

            debug!("Input configuration", &config);

            debug!("Make CoreProcess task");
            let core_process = CoreProcess::new(config);

            match core_process {
                Ok(core_fut) => {
                    spawn(
                        core_fut
                            .map_err(|err| {
                                print_err!("core_fut fails", err);
                                System::current().stop();
                                ()
                            })
                            .map(|_| ()),
                    );
                }
                Err(err) => {
                    print_err!("core_fut error", err);
                    exit(1);
                }
            }
            sys.run();
        }
        Err(err) => {
            print_err!("app config fails", err);
            exit(1);
        }
    }
}
