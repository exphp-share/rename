
extern crate clap;
extern crate regex;
extern crate combine;
extern crate tabwriter;

#[macro_use]
mod macros;
mod pattern;
use pattern::Pattern;

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

#[derive(Debug,Clone,PartialEq,Eq,PartialOrd,Ord,Hash)]
enum PathSources<'a> {
	These(Vec<&'a str>),
	Glob(Option<Vec<&'a str>>), // Note: Glob(None) is unrestricted, Glob(Some(_)) is restricted.
}

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
	let dry_run = matches.is_present("dry-run");
	let no_dry_run = matches.is_present("no-dry-run");
	let source = matches.value_of("source").unwrap();
	let target = matches.value_of("target").unwrap();
	let paths = matches.values_of("path").map(|c| c.collect::<Vec<_>>());

	let path_sources = match glob {
		true  => PathSources::Glob(paths),
		false => PathSources::These(paths.expect("No paths provided!")),
	};

	let source = pattern::parse(source).unwrap_or_else(|e| panic!("In source pattern: {}", e));
	let target = pattern::parse(target).unwrap_or_else(|e| panic!("In target pattern: {}", e));

	doit(path_sources, dry_run, no_dry_run, source, target)
}

fn doit(paths: PathSources, dry_run: bool, no_dry_run: bool, source: Pattern, target: Pattern) {
	use ::std::borrow::Cow;
	use ::std::io::prelude::*;
	let mut tw = ::tabwriter::TabWriter::new(::std::io::stdout());

	match paths {
		PathSources::Glob(_) => unimplemented!(),
		PathSources::These(paths) => {
			let regex = regex::Regex::new(&source.source_regex()).unwrap();
			let rep = target.target_regex();
			for &path in &paths {
				match regex.replace(path, rep.as_str()) {
					Cow::Borrowed(_) => {} // No match
					Cow::Owned(s) => {
						writeln!(tw, "mv '{}'\t'{}'", path, s).unwrap();
					}
				}
			}
		},
	}
	tw.flush().unwrap();

	match (dry_run, no_dry_run) {
		(false, false) => {
			eprintln!("NOTICE: This was a DRY RUN!!!!!");
			eprintln!("        If you like the results, use the -D flag to DO IT!");
		},
		( true,  true) => {
			eprintln!("NOTICE: This was a DRY RUN!!!!!");
			eprintln!("        If you like the results, remove the -d flag.");
		},
		( true, false) => {
			eprintln!("NOTICE: This was a DRY RUN!!!!! (in fact; this is the default! Forget -d!)");
			eprintln!("        If you like the results, replace -d with -D to DO IT!");
		},
		(false,  true) => unimplemented!(),
	}
}
