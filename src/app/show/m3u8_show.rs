use std::io::{stdout, Stdout, Write};

use crate::{
    app::show::common::bars,
    common::{
        colors::{Black, Blue, Green, Red, Yellow},
        errors::Result,
        size::HumanReadable,
        terminal::terminal_width,
    },
};

pub struct M3u8Shower {
    stdout: Stdout,
}

impl M3u8Shower {
    pub fn new() -> M3u8Shower {
        M3u8Shower { stdout: stdout() }
    }

    pub fn print_msg(&mut self, msg: &str) -> Result<()> {
        writeln!(&mut self.stdout, "\n  {}", Yellow.italic().paint(msg))?;
        Ok(())
    }

    pub fn print_file(&mut self, path: &str) -> Result<()> {
        writeln!(
            &mut self.stdout,
            // "\n     {}: {}",
            "\n{}: {}",
            Green.bold().paint("File"),
            path,
        )?;
        Ok(())
    }

    pub fn print_total(&mut self, total: u64) -> Result<()> {
        writeln!(
            &mut self.stdout,
            "{}: {}",
            Blue.bold().paint("Segments"),
            total,
        )?;
        Ok(())
    }

    pub fn print_concurrency(&mut self, concurrency: u64) -> Result<()> {
        writeln!(
            &mut self.stdout,
            "{}: {}\n",
            Yellow.bold().paint("concurrency"),
            concurrency,
        )?;
        Ok(())
    }

    pub fn print_status(&mut self, completed: u64, total: u64, rate: f64) -> Result<()> {
        let percent = completed as f64 / total as f64;

        let completed_str = completed.to_string();
        let total_str = total.to_string();
        let percent_str = format!("{:.2}", percent * 100.0);
        let rate_str = rate.human_readable();

        // maximum info length is `completed_str.len()` + `total_str.len()` + 19
        // e.g.
        //   100/1021 97.98% 1003.1B/s eta: 12s
        let info = format!(
            "{completed}/{total} {percent}% {rate}/s",
            completed = completed_str,
            total = total_str,
            percent = percent_str,
            rate = rate_str,
        );

        // set default info length
        let info_length = completed_str.len() + total_str.len() + 19;
        let miss = info_length - info.len();

        let terminal_width = terminal_width();
        let bar_length = terminal_width - info_length as u64 - 3;
        let process_bar_length = (bar_length as f64 * percent) as u64;
        let blank_length = bar_length - process_bar_length;

        let (bar, bar_right, bar_left) = bars();

        let bar_done_str = if process_bar_length > 0 {
            format!(
                "{}{}",
                bar.repeat((process_bar_length - 1) as usize),
                bar_right
            )
        } else {
            "".to_owned()
        };
        let bar_undone_str = if blank_length > 0 {
            format!("{}{}", bar_left, bar.repeat(blank_length as usize - 1))
        } else {
            "".to_owned()
        };

        write!(
            &mut self.stdout,
            "\r{completed}/{total} {percent}% {rate}/s{miss} {process_bar}{blank}  ",
            completed = Red.bold().paint(completed_str),
            total = Green.bold().paint(total_str),
            percent = Yellow.bold().paint(percent_str),
            rate = Blue.bold().paint(rate_str),
            miss = " ".repeat(miss),
            process_bar = Red.bold().paint(bar_done_str),
            blank = Black.bold().paint(bar_undone_str),
        )?;

        self.stdout.flush()?;

        Ok(())
    }
}
