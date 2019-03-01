#![allow(unused_variables)]
#![allow(dead_code)]

use std::process::exit;

use futures::Future;

use actix::{spawn, System};

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
mod util;

use crate::app::App;
use crate::core::CoreProcess;

fn main() {
    let sys = System::new("Aget");
    let app = App::new();
    match app.config() {
        Ok(config) => {
            let core_process = CoreProcess::new(config);

            match core_process {
                Ok(core_fut) => {
                    spawn(
                        core_fut
                            .map_err(|err| {
                                eprintln!("[{}:{}] {}", file!(), line!(), err);
                                System::current().stop();
                                ()
                            })
                            .map(|_| ()),
                    );
                }
                Err(err) => {
                    eprintln!("[{}:{}] {}", file!(), line!(), err);
                    exit(1);
                }
            }
            sys.run();
        }
        Err(err) => {
            eprintln!("[{}:{}] {}", file!(), line!(), err);
            exit(1);
        }
    }
}
