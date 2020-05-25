#[cfg(target_os = "windows")]
pub fn bars() -> (&'static str, &'static str, &'static str) {
    // bar, bar_right, bar_left
    ("▓", "░", " ")
}

#[cfg(not(target_os = "windows"))]
pub fn bars() -> (&'static str, &'static str, &'static str) {
    // bar, bar_right, bar_left
    ("━", "╸", "╺")
}

/// Return done and undone bars' string
pub fn du_bars(bar_done_length: usize, bar_undone_length: usize) -> (String, String) {
    let (bar, bar_right, bar_left) = bars();

    let bar_done_str = if bar_done_length > 0 {
        if bar_undone_length > 0 {
            bar.repeat((bar_done_length - 1) as usize) + bar_right
        } else {
            // Remove bar_right when bar_undone_length is zero
            bar.repeat(bar_done_length as usize)
        }
    } else {
        "".to_owned()
    };

    let bar_undone_str = if bar_undone_length > 0 {
        bar_left.to_owned() + &bar.repeat(bar_undone_length as usize - 1)
    } else {
        "".to_owned()
    };

    (bar_done_str, bar_undone_str)
}
