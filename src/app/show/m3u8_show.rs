use std::io::{stdout, Stdout, Write};

use crate::{
    app::show::common::du_bars,
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
        writeln!(&mut self.stdout, "{}: {}", Blue.bold().paint("Segments"), total,)?;
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

    pub fn print_status(&mut self, completed: u64, total: u64, length: u64, rate: f64) -> Result<()> {
        let percent = completed as f64 / total as f64;

        let completed_str = completed.to_string();
        let total_str = total.to_string();
        let length_str = length.human_readable();
        let percent_str = format!("{:.2}", percent * 100.0);
        let rate_str = rate.human_readable();

        // maximum info length is `completed_str.len()` + `total_str.len()` + 26
        // e.g.
        //   100/1021 97.98% 10m 1003.1B/s eta: 12s
        let info = format!(
            "{completed}/{total} {length} {percent}% {rate}/s",
            completed = completed_str,
            total = total_str,
            length = length_str,
            percent = percent_str,
            rate = rate_str,
        );

        // set default info length
        let info_length = total_str.len() * 2 + 26;
        let mut miss = info_length - info.len();

        let terminal_width = terminal_width();
        let bar_length = if terminal_width > info_length as u64 + 3 {
            terminal_width - info_length as u64 - 3
        } else {
            miss = 0;
            0
        };

        let bar_done_length = (bar_length as f64 * percent) as u64;
        let bar_undone_length = bar_length - bar_done_length;
        let (bar_done_str, bar_undone_str) = du_bars(bar_done_length as usize, bar_undone_length as usize);

        write!(
            &mut self.stdout,
            "\r{completed}/{total} {length} {percent}% {rate}/s{miss} {bar_done}{bar_undone}  ",
            completed = Red.bold().paint(completed_str),
            total = Green.bold().paint(total_str),
            length = Red.bold().paint(length_str),
            percent = Yellow.bold().paint(percent_str),
            rate = Blue.bold().paint(rate_str),
            miss = " ".repeat(miss),
            bar_done = Red.bold().paint(bar_done_str),
            bar_undone = Black.bold().paint(bar_undone_str),
        )?;

        self.stdout.flush()?;

        Ok(())
    }
}
