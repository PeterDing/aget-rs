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
