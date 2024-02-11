macro_rules! abort {
    ($($arg:tt)*) => {{
        eprintln!($($arg)*);
        std::process::exit(1);
    }};
}

// Define a trait for extending Option and Result types
pub(crate) trait AbortableResult<T> {
    fn unwrap_abort(self) -> T;
}

// Implement the trait for Result
impl<T, E: std::fmt::Debug> AbortableResult<T> for Result<T, E> {
    fn unwrap_abort(self) -> T {
        match self {
            Ok(value) => value,
            Err(e) => abort!("{:?}", e),
        }
    }
}
