#[macro_export]
macro_rules! apply_if_some {
    ($cfg:expr, $o:expr) => {
        if let Some(v) = $o {
            $cfg = v
        }
    };
}
