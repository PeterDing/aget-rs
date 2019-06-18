use std::io::{stdout, Stdout, Write};

use ansi_term::{
    Colour::{Blue, Cyan, Green, Red, Yellow},
    Style,
};

use crate::{
    error::{AgetError, Result},
    util::{terminal_width, SizeOfFmt, TimeOfFmt},
};

pub struct Printer {
    colors: Colors,
    terminal_width: u64,
    stdout: Stdout,
}

impl Printer {
    pub fn new() -> Printer {
        let terminal_width = terminal_width();
        Printer {
            colors: Colors::colored(),
            terminal_width,
            stdout: stdout(),
        }
    }

    pub fn print_msg(&mut self, msg: &str) -> Result<(), AgetError> {
        writeln!(&mut self.stdout, "\n  {}", self.colors.msg.paint(msg))?;
        Ok(())
    }

    pub fn print_header(&mut self, path: &str) -> Result<(), AgetError> {
        writeln!(
            &mut self.stdout,
            "\n  {}: {}",
            self.colors.file_header.paint("  File"),
            path,
        )?;
        Ok(())
    }

    pub fn print_length(&mut self, content_length: u64) -> Result<(), AgetError> {
        writeln!(
            &mut self.stdout,
            "  {}: {} ({})\n",
            self.colors.content_length_header.paint("Length"),
            content_length.sizeof_fmt(),
            content_length,
        )?;
        Ok(())
    }

    pub fn print_process(
        &mut self,
        completed_length: u64,
        total_length: u64,
        rate: f64,
        eta: u64,
    ) -> Result<(), AgetError> {
        let percent = completed_length as f64 / total_length as f64;

        let completed_length_str = completed_length.sizeof_fmt();
        let total_length_str = total_length.sizeof_fmt();
        let percent_str = format!("{:.2}", percent * 100.0);
        let rate_str = rate.sizeof_fmt();
        let eta_str = eta.timeof_fmt();

        // maximum info length is 41 e.g.
        //   1001.3k/1021.9m 97.98% 1003.1B/s eta: 12s
        let info = format!(
            "{completed_length}/{total_length} {percent}% {rate}/s eta: {eta}",
            completed_length = completed_length_str,
            total_length = total_length_str,
            percent = percent_str,
            rate = rate_str,
            eta = eta_str,
        );

        // set default info length
        let info_length = 41;
        let miss = info_length - info.len();

        let bar_length = self.terminal_width - info_length as u64 - 4;
        let process_bar_length = (bar_length as f64 * percent) as u64;
        let blank_length = bar_length - process_bar_length;

        let process_bar_str = if process_bar_length > 0 {
            format!("{}>", "=".repeat((process_bar_length - 1) as usize))
        } else {
            "".to_owned()
        };
        let blank_str = " ".repeat(blank_length as usize);

        write!(
            &mut self.stdout,
            "\r{completed_length}/{total_length} {percent}% {rate}/s eta: {eta}{miss} [{process_bar}{blank}] ",
            completed_length = self.colors.completed_length.paint(completed_length_str),
            total_length = self.colors.total_length.paint(total_length_str),
            percent = self.colors.percent.paint(percent_str),
            rate = self.colors.rate.paint(rate_str),
            eta = self.colors.eta.paint(eta_str),
            miss = " ".repeat(miss),
            process_bar = process_bar_str,
            blank = blank_str,
        )?;

        self.stdout.flush()?;

        Ok(())
    }
}

#[derive(Default)]
pub struct Colors {
    pub file_header: Style,
    pub content_length_header: Style,
    pub completed_length: Style,
    pub total_length: Style,
    pub percent: Style,
    pub rate: Style,
    pub eta: Style,
    pub msg: Style,
}

impl Colors {
    pub fn plain() -> Colors {
        Colors::default()
    }

    pub fn colored() -> Colors {
        Colors {
            file_header: Green.bold(),
            content_length_header: Blue.bold(),
            completed_length: Red.bold(),
            total_length: Green.bold(),
            percent: Yellow.bold(),
            rate: Blue.bold(),
            eta: Cyan.bold(),
            msg: Yellow.italic(),
        }
    }
}
