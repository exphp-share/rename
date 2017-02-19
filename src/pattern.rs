

#[derive(Debug,Clone,PartialEq,Eq,PartialOrd,Ord,Hash)]
pub struct Pattern(Vec<Component>);

#[derive(Debug,Clone,PartialEq,Eq,PartialOrd,Ord,Hash)]
pub enum Subst {
	Named(String),
	Positional,
}

#[derive(Debug,Clone,PartialEq,Eq,PartialOrd,Ord,Hash)]
pub enum Component {
	Subst(Subst),
	Literal(String),
}

impl Pattern {
	fn make_regex<N,P,L>(&self, surround: (&str, &str), mut named: N, mut positional: P, mut literal: L) -> String
	 where N: FnMut(&str) -> String,
	       P: FnMut() -> String,
	       L: FnMut(&str) -> String {
		use self::Subst::*;
		use self::Component::*;

		let mut regex = String::new();
		regex += surround.0;
		for c in &self.0 {
			match *c {
				Subst(Named(ref name)) => regex += &named(name),
				Subst(Positional)      => regex += &positional(),
				Literal(ref s)         => regex += &literal(s),
			};
		}
		regex += surround.1;
		regex
	}

	pub fn source_regex(&self) -> String {
		self.make_regex(
			("^", "$"),
			|name| format!("(?P<{}>.*)", name),
			||     format!("(.*)"),
			::regex::escape,
		)
	}

	pub fn target_regex(&self) -> String {
		let mut indices = 1usize..;
		self.make_regex(
			("", ""),
			|name| format!("${{{}}}", name),
			||     format!("${{{}}}", indices.next().unwrap()),
			escape_regex_replacement,
		)
	}
}

// The regex crate doesn't have an escape function for replacement strings...
fn escape_regex_replacement(s: &str) -> String {
	// looks like every metasequence starts with $, so I think just escaping those should work
	s.replace("$", "$$")
}

pub fn parse(s: &str) -> Result<Pattern, ::combine::ParseError<&str>> {
	use ::combine::*;
	use ::combine::char::*;

	let positional = string("[]").map(|_| Subst::Positional);

	let named = token('[')
		.with(many1(letter()).expected("subst name"))
		.skip(token(']'))
		.map(Subst::Named);

	let subst = try(positional).or(try(named)).map(Component::Subst);

	let literal_char =
		try(satisfy(|c| c != '[' && c != ']'))
		.or(try(string("[[").map(|_| '[')))
		.or(try(string("]]").map(|_| ']')));

	let literal = many1(literal_char).map(Component::Literal);

	let component = try(subst).or(try(literal));
	let components = many1(component);
	let mut pattern = components.skip(eof()).map(Pattern);

	pattern.parse(s).map(|x| x.0)
}

#[cfg(test)]
mod tests {
	use super::*;

	macro_rules! assert_parse {
		($input: expr => X__X ) => {
			assert!(parse($input).is_err());
		};
		($input: expr => $($e:expr),*) => {
			assert_eq!(parse($input), Ok(Pattern(vec![$($e),*])));
		};
	}

	#[test]
	fn test_parser() {
		let named = |s: &'static str| Component::Subst(Subst::Named(s.to_string()));
		let pos   = || Component::Subst(Subst::Positional);
		let lit   = |s: &'static str| Component::Literal(s.to_string());
		assert_parse!("literal" => lit("literal"));
		assert_parse!("[named]" => named("named"));
		assert_parse!("[]" => pos());

		assert_parse!("[named]" => named("named"));
		assert_parse!("ab]]c[[def" => lit("ab]c[def"));
		assert_parse!("" => X__X);
		assert_parse!("[asbf-sf]" => X__X);
		assert_parse!("[asb" => X__X);
		assert_parse!("so[]me-[example]e" => lit("so"), pos(), lit("me-"), named("example"), lit("e"));
		// ... eh... and this is the point where I lose interest.
	}
}
