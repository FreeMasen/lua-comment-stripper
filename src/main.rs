use std::path::PathBuf;

use clap::Parser;
use lua_comment_stripper::walk_dir;

#[derive(Debug, Parser)]
struct Args {
    /// The input directory
    input: PathBuf,
    /// The output directory
    output: PathBuf,
    /// The directory to output diff files
    #[arg(long)]
    diff_dir: Option<PathBuf>,
    /// If provided will output the full file diffs including whitespace and comments
    #[arg(long, short)]
    diff_verbose: bool,
    /// Clean the output directory before writing
    #[arg(short, long)]
    clean: bool,
}

fn main() {
    let args = Args::parse();
    if args.clean {
        std::fs::remove_dir_all(&args.output).ok();
        if let Some(diff) = &args.diff_dir {
            std::fs::remove_dir_all(&diff).ok();
        }
    }
    walk_dir(args.input, args.output, args.diff_dir, args.diff_verbose)
}
