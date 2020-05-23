pub static mut DEBUG: bool = false;
pub static mut QUIET: bool = false;

#[macro_export]
macro_rules! print_err {
    ( $ctx:expr, $err:expr ) => {
        eprintln!("[{}:{}] {}: {}", file!(), line!(), $ctx, $err);
    };
}

#[macro_export]
macro_rules! debug {
    ( $title:expr, $msg:expr ) => {
        unsafe {
            if crate::common::debug::DEBUG {
                eprintln!("[{}:{}] {}: {:#?}", file!(), line!(), $title, $msg);
            }
        }
    };
    ( $title:expr ) => {
        unsafe {
            if crate::common::debug::DEBUG {
                eprintln!("[{}:{}] {}", file!(), line!(), $title);
            }
        }
    };
}
