

#[derive(Debug,Clone,PartialEq,Eq,PartialOrd,Ord,Hash)]
pub struct SourcePattern(Vec<SourceComponent>);
#[derive(Debug,Clone,PartialEq,Eq,PartialOrd,Ord,Hash)]
pub struct TargetPattern(Vec<TargetComponent>);

#[derive(Debug,Clone,PartialEq,Eq,PartialOrd,Ord,Hash)]
pub enum SourceComponent {
	Subst(Subst, Glob),
	Literal(String),
}

#[derive(Debug,Clone,PartialEq,Eq,PartialOrd,Ord,Hash)]
pub enum TargetComponent {
	Subst(Subst),
	Literal(String),
}

#[derive(Debug,Clone,PartialEq,Eq,PartialOrd,Ord,Hash)]
pub enum Subst {
	Named(String),
	Positional,
}

#[derive(Debug,Copy,Clone,PartialEq,Eq,PartialOrd,Ord,Hash)]
pub enum Glob { Glob, GlobStar }

impl Glob {
	fn regex_str(self) -> &'static str {
		match self {
			Glob::Glob => "[^/]*",
			Glob::GlobStar => ".*",
		}
	}

	fn glob_str(self) -> &'static str {
		match self {
			Glob::Glob => "*",
			Glob::GlobStar => "**",
		}
	}
}

impl SourcePattern {
	pub fn regex(&self) -> ::regex::Regex {
		use self::Subst::*;
		use self::SourceComponent::*;

		let mut out = String::new();
		out += "^";
		for c in &self.0 {
			out += &match *c {
				Subst(Named(ref name), glob) => format!("(?P<{}>{})", name, glob.regex_str()),
				Subst(Positional, glob)      => format!("({})", glob.regex_str()),
				Literal(ref s)               => ::regex::escape(s),
			};
		}
		out += "$";
		::regex::Regex::new(&out).unwrap()
	}

	pub fn glob(&self) -> String {
		use self::SourceComponent::*;

		let mut out = String::new();
		for c in &self.0 {
			&match *c {
				Subst(_, glob) => out += glob.glob_str(),
				Literal(ref s) => out += &::glob::Pattern::escape(s),
			};
		}
		out
	}
}

impl TargetPattern {
	pub fn rep(&self) -> String {
		use self::Subst::*;
		use self::TargetComponent::*;

		let mut indices = 1usize..;

		let mut out = String::new();
		for c in &self.0 {
			out += &match *c {
				Subst(Named(ref name)) => format!("${{{}}}", name),
				Subst(Positional)      => format!("${{{}}}", indices.next().unwrap()),
				Literal(ref s)         => escape_regex_replacement(s),
			};
		}
		out
	}
}

// The regex crate doesn't have an escape function for replacement strings...
fn escape_regex_replacement(s: &str) -> String {
	// looks like every metasequence starts with $, so I think just escaping those should work
	s.replace("$", "$$")
}

// parser helpers
macro_rules! just { // parses nothing successfully and returns $x
	($x:expr) => { string("").map(|_| $x) };
}
macro_rules! bracket { // embed parser $p in-between two tokens (only keeping $p's output)
	($b:expr, $a:expr, $p:expr) => { token($b).with($p).skip(token($a)) };
}

pub type Error<'a> = ::combine::ParseError<::combine::State<&'a str>>;
pub type SourceResult<'a> = Result<SourcePattern, Error<'a>>;
pub type TargetResult<'a> = Result<TargetPattern, Error<'a>>;

// NOTE: Both input lifetimes are equal to facilitate reuse of some subparsers
pub fn parse<'a>(mut source: &'a str, mut target: &'a str) -> (SourceResult<'a>, TargetResult<'a>) {
	use ::combine::*;
	use ::combine::char::*;

	// Parsers are tricky to return, even with trait objects.
	// Luckily this is called at most once per program invocation so there's little
	// cost to doing all the parser construction that occurs in here.

	// We do both target and source in the same function for ease of organization and sharing.

	// Parsers which are shared between source and target are written as closures
	//  for the easy production of copies. (this is there the equal lifetime constraint arises)

	// ---
	// glob normalizes paths, so that it never outputs trailing slashes (resulting in no matches)
	// FIXME: This is a bandaid; we need to somehow match
	//         glob's normalization scheme in our regexes
	source = source.trim_right_matches('/');
	target = target.trim_right_matches('/');

	// Subst
	let subst_id = ||
		many1(letter()).map(Subst::Named)
		.or(just!(Subst::Positional));

	let glob_spec =
		token('*').with(
			token('*').map(|_| Glob::GlobStar)
			.or(just!(Glob::Glob))
		);

	let source_subst_specs =
		token(':').with(glob_spec)
		.or(just!(Glob::Glob));

	let source_subst =
		bracket!('[', ']', subst_id().and(source_subst_specs))
		.map(|(subst, specs)| SourceComponent::Subst(subst, specs));

	let target_subst =
		bracket!('[', ']', subst_id())
		.map(TargetComponent::Subst);

	// Literal text
	let literal_char = ||
		satisfy(|c| c != '[' && c != ']')
		.or(try(string("[[")).map(|_| '['))
		.or(try(string("]]")).map(|_| ']'));

	let source_literal = many1(literal_char()).map(SourceComponent::Literal);
	let target_literal = many1(literal_char()).map(TargetComponent::Literal);

	let source_component = try(source_subst).or(try(source_literal));
	let target_component = try(target_subst).or(try(target_literal));

	let mut source_pattern = many1(source_component).skip(eof()).map(SourcePattern);
	let mut target_pattern = many1(target_component).skip(eof()).map(TargetPattern);

	( source_pattern.parse(State::new(source)).map(|x| x.0)
	, target_pattern.parse(State::new(target)).map(|x| x.0)
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	macro_rules! assert_source {
		($input: expr => X__X ) => {
			assert!(parse_source($input).is_err());
		};
		($input: expr => $($e:expr),*) => {
			parse_source($input).unwrap_or_else(|e| panic!("{}", e));
			assert_eq!(parse_source($input), Ok(SourcePattern(vec![$($e),*])));
		};
	}

	macro_rules! assert_target {
		($input: expr => X__X ) => {
			assert!(parse_target($input).is_err());
		};
		($input: expr => $($e:expr),*) => {
			parse_target($input).unwrap_or_else(|e| panic!("{}", e));
			assert_eq!(parse_target($input), Ok(TargetPattern(vec![$($e),*])));
		};
	}

	fn parse_source(source: &str) -> Result<SourcePattern, Error> { parse(source, "x").0 }
	fn parse_target(target: &str) -> Result<TargetPattern, Error> { parse("x", target).1 }

	#[test]
	fn test_source() {
		use super::Glob::*;
		use super::Subst::*;
		use super::SourceComponent::*;
		let named = |s: &'static str, glob| Subst(Named(s.to_string()), glob);
		let pos   = |glob|                  Subst(Positional, glob);
		let lit   = |s: &'static str| Literal(s.to_string());

		assert_source!("literal" => lit("literal"));
		assert_source!("[named]" => named("named", Glob));
		assert_source!("[]" => pos(Glob));
		assert_source!("[named:*]" => named("named", Glob));
		assert_source!("[:*]" => pos(Glob));
		assert_source!("[named:**]" => named("named", GlobStar));
		assert_source!("[:**]" => pos(GlobStar));

		assert_source!("[named]" => named("named", Glob));
		assert_source!("ab]]c[[def" => lit("ab]c[def"));
		assert_source!("so[]me-[example]e" =>
			lit("so"), pos(Glob), lit("me-"), named("example", Glob), lit("e"));

		assert_source!("[:***]" => X__X);
		assert_source!("[:*a]" => X__X);
		assert_source!("[:a]" => X__X);
		assert_source!("[:]" => X__X);
		assert_source!("" => X__X);
		assert_source!("[asbf-sf]" => X__X);
		assert_source!("[asb" => X__X);
		assert_source!("[a1]" => X__X);

		// I could be here all day you know
		// Just try me
		assert_source!("[[]" => X__X);
		assert_source!("[]]" => X__X);
		assert_source!("[[[]" => lit("["), pos(Glob));
		assert_source!("[]]]" => pos(Glob), lit("]"));
	}

	#[test]
	fn test_target() {
		use super::Subst::*;
		use super::TargetComponent::*;
		let named = |s: &'static str| Subst(Named(s.to_string()));
		let pos   = ||                Subst(Positional);
		let lit   = |s: &'static str| Literal(s.to_string());

		assert_target!("literal" => lit("literal"));
		assert_target!("[named]" => named("named"));
		assert_target!("[]" => pos());
		assert_target!("[:*]" => X__X);
		assert_target!("[named:**]" => X__X);
		assert_target!("[:**]" => X__X);

		assert_target!("[named]" => named("named"));
		assert_target!("ab]]c[[def" => lit("ab]c[def"));
		assert_target!("so[]me-[example]e" =>
			lit("so"), pos(), lit("me-"), named("example"), lit("e"));

		assert_target!("[named:*]" => X__X);
		assert_target!("[:***]" => X__X);
		assert_target!("[:*a]" => X__X);
		assert_target!("[:a]" => X__X);
		assert_target!("" => X__X);
		assert_target!("[asbf-sf]" => X__X);
		assert_target!("[asb" => X__X);
		assert_target!("[abc]]" => X__X);

		assert_target!("[[]" => X__X);
		assert_target!("[]]" => X__X);
		assert_target!("[[[]" => lit("["), pos());
		assert_target!("[]]]" => pos(), lit("]"));
	}
}
