use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};

const FRAGMENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .remove(b'?')
    .remove(b'/')
    .remove(b':')
    .remove(b'=');

pub fn escape_nonascii(target: &str) -> String {
    utf8_percent_encode(target, FRAGMENT).to_string()
}

#[cfg(test)]
mod tests {
    use super::escape_nonascii;

    #[test]
    fn test_escape_nonascii() {
        let s = ":ss/s  来；】/ 【【 ? 是的 & 水电费=45 进来看";
        println!("{}", s);
        println!("{}", escape_nonascii(s));
    }
}
