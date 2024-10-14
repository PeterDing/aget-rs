use std::io::{stdout, Stdout, Write};

use crate::{
    app::show::common::du_bars,
    common::{
        colors::{Black, Blue, Cyan, Green, Red, Yellow, RGB},
        errors::Result,
        liberal::ToDate,
        size::HumanReadable,
        terminal::terminal_width,
    },
};

pub struct BtShower {
    stdout: Stdout,
}

impl BtShower {
    pub fn new() -> BtShower {
        BtShower { stdout: stdout() }
    }

    pub fn print_msg(&mut self, msg: &str) -> Result<()> {
        writeln!(&mut self.stdout, "\n  {}", Yellow.italic().paint(msg))?;
        Ok(())
    }

    pub fn print_name(&mut self, name: &str) -> Result<()> {
        writeln!(&mut self.stdout, "\n{}: {}", Green.bold().paint("Torrent Name"), name,)?;
        Ok(())
    }

    pub fn print_files(&mut self, files: Vec<(&str, u64, bool)>) -> Result<()> {
        for (filename, length, included) in files {
            writeln!(
                &mut self.stdout,
                "{} {}: {} ({})",
                if included {
                    Green.bold().paint("✓")
                } else {
                    Red.bold().paint("✘")
                },
                Blue.bold().paint("File"),
                filename,
                length.human_readable(),
            )?;
        }
        Ok(())
    }

    pub fn print_status(
        &mut self,
        completed: u64,
        total: u64,
        eta: u64,
        down_rate: f64,
        up_rate: f64,
        uploaded: u64,
        live: usize,
        queued: usize,
    ) -> Result<()> {
        let percent = if total != 0 {
            completed as f64 / total as f64
        } else {
            0.0
        };

        let completed_str = completed.human_readable();
        let total_str = total.human_readable();
        let percent_str = format!("{:.2}", percent * 100.0);
        let down_rate_str = down_rate.human_readable();
        let up_rate_str = up_rate.human_readable();
        let uploaded_str = uploaded.human_readable();
        let eta_str = eta.date();

        // maximum info length is 41 e.g.
        // 571.9M/5.8G 9.63% ↓1.8M/s ↑192.1K/s(63.7M) eta: 49m peers: 22/102
        let info = format!(
            "{completed}/{total} {percent}% ↓{down_rate}/s ↑{up_rate}/s({uploaded}) eta: {eta} peers: {live}/{queued}",
            completed = completed_str,
            total = total_str,
            percent = percent_str,
            down_rate = down_rate_str,
            up_rate = up_rate_str,
            uploaded = uploaded_str,
            eta = eta_str,
        );

        // set default info length
        let info_length = 71;
        let miss = info_length - info.len();

        let terminal_width = terminal_width();
        let bar_length = terminal_width - info_length as u64 - 3;

        let (bar_done_str, bar_undone_str) = if total != 0 {
            let bar_done_length = (bar_length as f64 * percent) as u64;
            let bar_undone_length = bar_length - bar_done_length;
            du_bars(bar_done_length as usize, bar_undone_length as usize)
        } else {
            (" ".repeat(bar_length as usize), "".to_owned())
        };

        write!(
            &mut self.stdout,
            "\r{completed}/{total} {percent}% ↓{down_rate}/s ↑{up_rate}/s({uploaded}) eta: {eta} peers: {live}/{queued}{miss} {bar_done}{bar_undone}  ",
            // "\r{completed}/{total} {percent}% {down_rate}/s eta: {eta}{miss} {bar_done}{bar_undone}  ",
            completed = Red.bold().paint(completed_str),
            total = Green.bold().paint(total_str),
            percent = Yellow.bold().paint(percent_str),
            down_rate = Blue.bold().paint(down_rate_str),
            up_rate = RGB(0x66, 0x00, 0xcc).bold().paint(up_rate_str),
            uploaded = uploaded_str,
            eta = Cyan.bold().paint(eta_str),
            miss = " ".repeat(miss),
            bar_done = if total != 0 {
                Red.bold().paint(bar_done_str).to_string()
            } else {
                bar_done_str
            },
            bar_undone = if total != 0 {
                Black.bold().paint(bar_undone_str).to_string()
            } else {
                bar_undone_str
            }
        )?;

        self.stdout.flush()?;

        Ok(())
    }

    pub fn print_completed_file(&mut self, name: &str) -> Result<()> {
        writeln!(&mut self.stdout, "\n{}: {}", Green.italic().paint("Completed"), name,)?;
        Ok(())
    }
}
