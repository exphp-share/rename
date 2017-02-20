
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
use ::regex::Regex;

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
// Idea: --copy or bin name "recopy" to copy instead of move
//    * This affects the set of desirable safety checks;
//      an input can now have multiple outputs;

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
		.arg(Arg::from_usage("-x, --command")
			.default_value("mv")
			.help("Set the mv command used to prefix output. e.g. --command='cp -a'"))
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
	let command = matches.value_of("command").unwrap();
	let paths = matches.values_of("path").map(|c| c.map(|s| s.to_string()).collect::<Vec<_>>());

	let path_sources = match glob {
		true  => match paths {
			None => PathSources::Glob,
			Some(_paths) => unimplemented!(), // Globs over restricted paths
		},
		false => PathSources::These(paths.expect("No paths provided!")),
	};

	let (source,target) = pattern::parse(source,target);
	let source = unwrap_display!(source, "In source pattern: {}");
	let target = unwrap_display!(target, "In target pattern: {}");

	doit(path_sources, dry_run, source, target, command)
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

fn doit(paths: PathSources, dry_run: DryFlags, source: SourcePattern, target: TargetPattern, command: &str) {
	use ::std::path::PathBuf;
	use ::std::collections::HashSet;

	//-----------
	// Determine work to be done
	let regex = source.regex();
	let source_paths = match paths {
		PathSources::Glob => {
			// temp to own the PathBufs...
			let tmp = ::glob::glob(&source.glob()).unwrap()
				.filter_map(|x| ok_or_warn!(x))
				.collect::<Vec<_>>();

			// ...so that we can use the (borrowing) to_str to filter non-unicode names.
			// (because we can't match these with regexes!)
			tmp.iter()
				.flat_map(|b| warn_none!(b.to_str(), "Ignoring non-unicode path: {}", b.display()))
				.map(|s| s.to_string())
				.collect()
		},
		PathSources::These(paths) => paths,
	};

	let entries = {
		let rep = target.rep();
		source_paths.iter().map(|x| x.as_str())
			.filter_map(|src|

				replace_if_match(&regex, src, rep.as_str())

				.and_then(|targ| {
					// FIXME: It just occurred to me that readlink -f is not actually what
					//        we want here; if the source path itself is a link, we should
					//        move the link, not the referent. :/
					let canon = |s: &str| ok_or_warn!(readlink_f(s), "{}: {}", s);

					let cs = try_some!(canon(src));
					let ct = try_some!(canon(&targ));
					Some(((cs, ct), (src, targ)))
				})

			// File moved onto itself;
			// Remove it to avoid false negatives in the source-target overlap check.
			// FIXME I might still like to see it in the output listing...
			).filter(|&((ref cs, ref ct), (_, _))| cs != ct)

			.collect::<Vec<((PathBuf, PathBuf), (&str, String))>>()
	};

	// Do some checking now but defer error messages until the end
	let error_no_matches = entries.is_empty();

	let all_canon_sources = entries.iter().map(|t| &(t.0).0).collect::<HashSet<_>>();
	let all_canon_targets = entries.iter().map(|t| &(t.0).1).collect::<HashSet<_>>();

	let error_duplicate_source = all_canon_sources.len() < entries.len();
	let error_duplicate_target = all_canon_targets.len() < entries.len();

	let error_source_and_target = (&all_canon_sources & &all_canon_targets).into_iter().next();

	//-----------
	// Output
	let mut tw = ::tabwriter::TabWriter::new(::std::io::stdout());
	for &(_, (src, ref dest)) in &entries {
		// Use the non-canonicalized paths.
		// This is useful for e.g. flags like '--command=cp -a'.
		writeln!(tw, "{} '{}'\t'{}'", command, src, dest).unwrap();
	}
	tw.flush().unwrap();

	if error_no_matches {
		eprintln!("ERROR: No matches, or all input paths match their outputs!");
		eprintln!("       There is nothing to do.");
		::std::process::exit(1);
	}

	if error_duplicate_source {
		eprintln!("WARNING: A source path was matched more than once!");
		eprintln!("         In these cases, a single destination may be chosen arbitrarily.");
		// continue onwards...
	}

	if error_duplicate_target {
		// FIXME: Which file?
		//        And maybe this should refuse to run without some -f flag,
		//        because otherwise if you leave out a subst it could be SURPRISE, DATA LOSS!
		eprintln!("WARNING: A destination appears more than once!");
		eprintln!("         In such cases, some files may be lost!");
		// continue onwards...
	}

	if let Some(path) = error_source_and_target {
		eprintln!("ERROR: '{}'", path.display());
		eprintln!("       is both a source and a destination!");
		eprintln!("       This is extremely dangerous! I'm outta here!!");
		::std::process::exit(1);
	}

	if dry_run.is_dry() {
		dry_run.write_advice(::std::io::stderr());
	} else {
		eprintln!("ERROR: --no-dry-run is not yet implemented,");
		eprintln!("       but you can run the stdout output in a shell.");
		unimplemented!();
	}
}

/// Is 'Some' only when the regex matches.
fn replace_if_match(regex: &Regex, s: &str, rep: &str) -> Option<String> {
	use ::std::borrow::Cow;

	// regex.replace is documented to use Cow::Borrowed strictly in the case of no match.
	// (well, okay, I added the "strictly" part--but a man can hope, right?)
	match regex.replace(s, rep) {
		Cow::Borrowed(_) => None,
		Cow::Owned(s) => Some(s),
	}
}

/// An approximation of "readlink -f".
///
/// Fully canonicalize a path (absolute or relative) into an absolute path,
/// with no symlinks, '.', '..', or extra slashes,
/// but **allow the final component to not exist.**
fn readlink_f<P: AsRef<::std::path::Path> + Clone>(path: P) -> ::std::io::Result<::std::path::PathBuf> {
	// Leap before you look...
	::std::fs::canonicalize(path.clone())
	.or_else(|e| {
		let path: &::std::path::Path = path.as_ref();

		// we're only interested in resolving errors involving a nonexistent path...
		if path.exists() { return Err(e); }

		// ...which has a direct parent...
		let parent = try_some!(path.parent() => return Err(e));

		// (case where 'path' was relative with one component)
		let mut parent = parent.to_owned();
		if parent.to_string_lossy() == "" {
			parent = ::std::env::current_dir()?;
		}

		// ...that can be canonicalized.
		let mut base = ::std::fs::canonicalize(parent)?;
		base.push(path.file_name().unwrap());
		Ok(base)
	})
}

#[test]
fn test_replace_if_match() {
	assert_eq!(replace_if_match(&Regex::new("a").unwrap(), "a", "a"), Some("a".to_string()));
	assert_eq!(replace_if_match(&Regex::new("a").unwrap(), "b", "a"), None);
}
