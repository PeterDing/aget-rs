/// Here, we handle about size.

const SIZES: [&'static str; 5] = ["B", "K", "M", "G", "T"];

/// Convert number to human-readable
pub trait HumanReadable {
    fn human_readable(&self) -> String;
}

impl HumanReadable for u64 {
    fn human_readable(&self) -> String {
        let mut num = *self as f64;
        for s in &SIZES {
            if num < 1024.0 {
                return format!("{:.1}{}", num, s);
            }
            num /= 1024.0;
        }
        return format!("{:.1}{}", num, SIZES[SIZES.len() - 1]);
    }
}

impl HumanReadable for f64 {
    fn human_readable(&self) -> String {
        let mut num = *self;
        for s in &SIZES {
            if num < 1024.0 {
                return format!("{:.1}{}", num, s);
            }
            num /= 1024.0;
        }
        return format!("{:.1}{}", num, SIZES[SIZES.len() - 1]);
    }
}
