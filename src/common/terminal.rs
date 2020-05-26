use term_size::dimensions;

const MIN_TERMINAL_WIDTH: u64 = 60;

pub fn terminal_width() -> u64 {
    if let Some((width, _)) = dimensions() {
        width as u64
    } else {
        // for envrionment in which atty is not available,
        // example, at ci of osx
        MIN_TERMINAL_WIDTH
    }
}
