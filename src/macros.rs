
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

/// Coerce a Result with `Display`-able errors into an Option, with warnings to stderr.
macro_rules! ok_or_warn {
	($result:expr) => { ok_or_warn!($result, "{}") };
	($result:expr, $($fmt:tt)+) => {
		$result.map(Some).unwrap_or_else(|e| { eprintln!($($fmt)+, e); None })
	};
}

/// Unwrap a Result with `Display`-able errors.
macro_rules! unwrap_display {
	($result:expr) => { unwrap_display!($result, "{}"); };
	($result:expr, $($fmt:tt)+) => {
		$result.unwrap_or_else(|e| panic!($($fmt)+, e))
	};
}

/// "Inspect" an empty option
macro_rules! warn_none {
	($option:expr, $($fmt:tt)+) => {
		$option.or_else(|| { eprintln!($($fmt)+); None })
	};
}

/// "Inspect" an empty option
macro_rules! try_some {
	($option:expr) => {{ try_some!($option => return None) }};
	($option:expr => $($diverging_expr:tt)+) => {{
		#[allow(non_snake_case)]
		let the_rhs_of__try_some__must_diverge;

		if let Some(inner) = $option {
			the_rhs_of__try_some__must_diverge = inner;
		} else { $($diverging_expr)+; }

		the_rhs_of__try_some__must_diverge
	}};
}
