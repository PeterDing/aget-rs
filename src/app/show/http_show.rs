use std::io::{stdout, Stdout, Write};

use crate::{
    app::show::common::du_bars,
    common::{
        colors::{Black, Blue, Cyan, Green, Red, Yellow},
        errors::Result,
        liberal::ToDate,
        size::HumanReadable,
        terminal::terminal_width,
    },
};

pub struct HttpShower {
    stdout: Stdout,
}

impl HttpShower {
    pub fn new() -> HttpShower {
        HttpShower { stdout: stdout() }
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
            "{}: {} ({})",
            Blue.bold().paint("Length"),
            total.human_readable(),
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

    pub fn print_status(&mut self, completed: u64, total: u64, rate: f64, eta: u64) -> Result<()> {
        let percent = completed as f64 / total as f64;

        let completed_str = completed.human_readable();
        let total_str = total.human_readable();
        let percent_str = format!("{:.2}", percent * 100.0);
        let rate_str = rate.human_readable();
        let eta_str = eta.date();

        // maximum info length is 41 e.g.
        //   1001.3k/1021.9m 97.98% 1003.1B/s eta: 12s
        let info = format!(
            "{completed}/{total} {percent}% {rate}/s eta: {eta}",
            completed = completed_str,
            total = total_str,
            percent = percent_str,
            rate = rate_str,
            eta = eta_str,
        );

        // set default info length
        let info_length = 41;
        let miss = info_length - info.len();

        let terminal_width = terminal_width();
        let bar_length = terminal_width - info_length as u64 - 3;
        let bar_done_length = (bar_length as f64 * percent) as u64;
        let bar_undone_length = bar_length - bar_done_length;

        let (bar_done_str, bar_undone_str) =
            du_bars(bar_done_length as usize, bar_undone_length as usize);

        write!(
            &mut self.stdout,
            "\r{completed}/{total} {percent}% {rate}/s eta: {eta}{miss} {bar_done}{bar_undone}  ",
            completed = Red.bold().paint(completed_str),
            total = Green.bold().paint(total_str),
            percent = Yellow.bold().paint(percent_str),
            rate = Blue.bold().paint(rate_str),
            eta = Cyan.bold().paint(eta_str),
            miss = " ".repeat(miss),
            bar_done = Red.bold().paint(bar_done_str),
            bar_undone = Black.bold().paint(bar_undone_str),
        )?;

        self.stdout.flush()?;

        Ok(())
    }
}
