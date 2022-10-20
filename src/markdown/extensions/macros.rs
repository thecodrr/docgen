#[macro_export]
macro_rules! html {
    ($($arg:tt)*) => {{
        Event::Html(CowStr::from(format!($($arg)*)))
    }};
}
