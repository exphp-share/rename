
macro_rules! eprint {
	($($arg:tt)*) => ({
		use std::io::{Write, stderr};
		let _ = write!(&mut stderr(), $($arg)*);
	});
}

macro_rules! eprintln {
	() => (eprint!("\n"));
	($fmt:expr) => (eprint!(concat!($fmt, "\n")));
	($fmt:expr, $($arg:tt)*) => (eprint!(concat!($fmt, "\n"), $($arg)*));
}
