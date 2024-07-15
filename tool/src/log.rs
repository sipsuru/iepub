use crate::arg::ArgOption;


pub(crate) static mut IS_LOG: bool = false;

pub(crate) fn is_enable_log(opts: &[ArgOption]) -> bool {
    for ele in opts {
        if ele.key == "l" {
            unsafe { IS_LOG = true };
            return true;
        }
    }
    false
}
#[macro_export]
macro_rules! msg {
    ( $($arg:tt)+) => {
        unsafe{
            if $crate::log::IS_LOG {
                println!($($arg)+)
            }
        }

    };
}