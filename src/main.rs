
extern crate clap;
extern crate glob;
extern crate regex;
extern crate combine;
extern crate tabwriter;

#[macro_use]
mod macros;
mod pattern;
use pattern::{SourcePattern,TargetPattern};

use ::std::io::prelude::*;

// Ideas:
//
//  [name:*] is a glob
//  [name:**] is a glob star
//  [name:ANYTHING ELSE] treats the rhs as a regex on a single path component
//  [name:*:ANYTHING ELSE] is identical
//  [name:**:ANYTHING ELSE] matches '/' as well
//
// Issue with this ides:
//
//  [name:.*] could look like the "dot star" glob pattern to match hidden files
//            but is actually regex (so it is equivalent to *)
//  also: it would require a regex parser to correctly locate the final closing bracket. *shudders*
//        Thankfully the regex crate does provide the plumbing for this, but damn, man.
//
// Idea: should warn on overlap between source and target
// Idea: --color/-c to highlight wildcard matches

fn main() {
	use clap::{App,Arg};
	let matches = App::new("rename")
		.version("0.1")
		.author("Michael Lamparski <diagonaldevice@gmail.com>")
		.about("Mass rename files.")
		.arg(Arg::from_usage("-g, --glob")
			.help("Interpret each substitution in the source pattern as a glob and scan the local directory."))
		.arg(Arg::from_usage("-d, --dry-run")
			.help("Actually, a dry run is the default behavior. This flag only exists to override -D (allowing you to use an alias)."))
		.arg(Arg::from_usage("-D, --no-dry-run")
			.help("Required to actually do anything. Also known as the DO IT flag."))
		.arg(Arg::from_usage("<source> 'The pattern for input names'"))
		.arg(Arg::from_usage("<target> 'The pattern for destination names'"))
		.arg(Arg::from_usage("[path]... 'The files to rename. (for globs, it merely limits subtrees)'"))
		.get_matches()
		;

	let glob = matches.is_present("glob");
	let dry_run = DryFlags {
		maybe_not_dry: matches.is_present("no-dry-run"),
		very_much_dry: matches.is_present("dry-run"),
	};
	let source = matches.value_of("source").unwrap();
	let target = matches.value_of("target").unwrap();
	let paths = matches.values_of("path").map(|c| c.map(|s| s.to_string()).collect::<Vec<_>>());

	let path_sources = match glob {
		true  => match paths {
			None => PathSources::Glob,
			Some(_paths) => unimplemented!(), // Globs over restricted paths
		},
		false => PathSources::These(paths.expect("No paths provided!")),
	};

	let (source,target) = pattern::parse(source,target);
	let source = source.unwrap_or_else(|e| panic!("In source pattern: {}", e));
	let target = target.unwrap_or_else(|e| panic!("In target pattern: {}", e));

	doit(path_sources, dry_run, source, target)
}

#[derive(Debug,Clone,PartialEq,Eq,PartialOrd,Ord,Hash)]
enum PathSources {
	These(Vec<String>),
	Glob,
}

#[derive(Debug,Copy,Clone,PartialEq,Eq,PartialOrd,Ord,Hash)]
struct DryFlags {
	maybe_not_dry: bool, // --no-dry-run
	very_much_dry: bool, // --dry-run, which takes precedence
}

impl DryFlags {
	fn is_dry(self) -> bool { self.very_much_dry || !self.maybe_not_dry }

	fn write_advice<W:Write>(self, mut file: W) {
		let mut say = |s| writeln!(&mut file, "{}", s).unwrap();

		match (self.maybe_not_dry, self.very_much_dry) {
			(false, false) => {
				say("NOTICE: This was a DRY RUN!!!!!");
				say("        If you like the results, use the -D flag to DO IT!");
			},
			( true,  true) => {
				say("NOTICE: This was a DRY RUN!!!!!");
				say("        If you like the results, remove the -d flag.");
			},
			(false,  true) => {
				say("NOTICE: This was a DRY RUN!!!!! (in fact; this is the default! Forget -d!)");
				say("        If you like the results, replace -d with -D to DO IT!");
			},
			_ => { /* not a dry run */ },
		}
	}
}

fn doit(paths: PathSources, dry_run: DryFlags, source: SourcePattern, target: TargetPattern) {
	use ::std::borrow::Cow;
	use ::std::io::prelude::*;

	let source_paths = match paths {
		PathSources::Glob => {
			// temp to own the PathBufs...
			let tmp = ::glob::glob(&source.glob()).unwrap()
				.filter_map(|x| x.map(Some).unwrap_or_else(|e|
					// (show glob iteration errors but don't worry otherwise)
					{ eprintln!("{}", e); None }))
				.collect::<Vec<_>>();

			// ...so that we can use the (borrowing) to_str to filter non-unicode names.
			// (because we can't match these with regexes!)
			tmp.iter()
				.flat_map(|b| b.to_str().or_else(||
					{ eprintln!("Ignoring non-unicode path: {}", b.display()); None }))
				.map(|s| s.to_string())
				.collect()
		},
		PathSources::These(paths) => paths,
	};

	let regex = source.regex();
	let rep = target.rep();

	let mut tw = ::tabwriter::TabWriter::new(::std::io::stdout());
	for path in &source_paths {
		match regex.replace(path, rep.as_str()) {
			// FIXME: should exit with nonzero code if no matches
			Cow::Borrowed(_) => {} // Not a match
			Cow::Owned(s) => {
				writeln!(tw, "mv '{}'\t'{}'", path, s).unwrap();
			}
		}
	}
	tw.flush().unwrap();

	if dry_run.is_dry() {
		dry_run.write_advice(::std::io::stderr());
	} else {
		unimplemented!();
	}
}
