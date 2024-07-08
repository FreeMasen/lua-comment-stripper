use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use lex_lua::{Span, SpannedLexer as Lexer, Token};
use walkdir::{DirEntry, WalkDir};

pub fn walk_dir(input: PathBuf, output: PathBuf, diff_dir: Option<PathBuf>, diff_verbose: bool) {
    for entry in WalkDir::new(&input) {
        let Some(entry) = should_strip(entry) else {
            continue;
        };

        let Ok(orig) = std::fs::read(entry.path()) else {
            continue;
        };
        let stripped = do_one(&orig);
        let Ok(entry_path_end) = entry.path().strip_prefix(&input) else {
            continue;
        };
        let dest_path = output.join(entry_path_end);
        if let Some(p) = dest_path.parent() {
            std::fs::create_dir_all(p).ok();
        }
        if std::fs::write(&dest_path, &stripped).is_err() {
            continue;
        }
        if let Some(diff) = &diff_dir {
            let diff_path = diff.join(entry_path_end).with_extension("diff");
            let changes = if diff_verbose {
                generate_line_diff(&orig, &stripped, &entry.path(), &dest_path)
            } else {
                generate_diff_from_tokens(&orig, &stripped, &entry.path(), &dest_path)
            };
            if let Some(changes) = changes {
                if let Some(p) = diff_path.parent() {
                    std::fs::create_dir_all(p).ok();
                }
                std::fs::write(&diff_path, changes).ok();
            } else {
                std::fs::remove_file(&diff_path).ok();
            }
        }
    }
}

fn should_strip(entry: Result<DirEntry, walkdir::Error>) -> Option<DirEntry> {
    let entry = entry.ok()?;
    (!entry.file_type().is_dir()).then_some(())?;
    (entry
        .path()
        .extension()
        .map(|e| e == "lua")
        .unwrap_or(false))
    .then_some(())?;
    Some(entry)
}

/// Strip the comments from a single lua blob returning a transformed copy of the blob
fn do_one(lua: &[u8]) -> Vec<u8> {
    let mut parser = analisar::aware::Parser::new(lua);
    let mut ret = Vec::new();
    {
        let mut writer = escrever::Writer::new(&lua, &mut ret);
        while let Some(stmt) = parser.next() {
            let stmt = match stmt {
                Ok(stmt) => stmt,
                Err(e) => {
                    let msg = match e {
                        analisar::Error::UnexpectedToken(offset, msg) => {
                            let line = point_to_offset(lua, offset);
                            format!("Unexpected token: {msg}\n{line}",)
                        }
                        _ => format!("Error parsing: {e}")
                    };
                    eprintln!("{msg}");
                    std::process::exit(1);
                }
            };
            writer.write_stmt(&stmt.statement).ok();
        }
    }
    ret
}

fn point_to_offset(lua: &[u8], offset: usize) -> String {
    let lua = String::from_utf8_lossy(lua);
    let start = lua[..offset].rfind(|ch: char| ch == '\n').map(|idx| idx + 1).unwrap_or(0);
    let position = offset - start;
    let line = &lua[start..];
    let end = line.find(|ch| ch == '\n').unwrap_or(line.len());
    let line = &line[..end];
    format!("{line}\n{:>width$}", '^', width = position)
}

/// Generate a verbose diff by comparing lines
fn generate_line_diff(
    orig: &[u8],
    stripped: &[u8],
    o_path: &Path,
    s_path: &Path,
) -> Option<String> {
    let o = String::from_utf8_lossy(orig);
    let s = String::from_utf8_lossy(stripped);
    let mut ret = String::new();
    for v in diff::lines(&o, &s) {
        use diff::Result::*;
        match v {
            Left(s) => ret.push_str(&format!("- {s}\n")),
            Right(s) => ret.push_str(&format!("+ {s}\n")),
            Both(_, _) => {}
        }
    }
    if ret.is_empty() {
        return None;
    }
    let prefix = format!("--- {}\n+++ {}", o_path.display(), s_path.display());
    Some(format!("{}\n{}", prefix, ret))
}

/// Generate a line diff for the two byte slices, this will compare the slices directly
/// and consider any whitespace only mismatches as a match and any mismatches where the
/// contents are comments
fn generate_diff_from_tokens(
    orig: &[u8],
    stripped: &[u8],
    o_path: &Path,
    s_path: &Path,
) -> Option<String> {
    let diffs = generate_diff2(orig, stripped);
    if diffs.is_empty() {
        return None;
    }
    let splitter = |v: &u8| *v == b'\n' || *v == b'\r' || *v == 0xff;
    let mut ret = String::new();
    let diff_set = BTreeSet::from_iter(diffs.iter().map(|s| s.start));
    let orig_span = 0;
    for (line_no, (o, s)) in orig
        .split_inclusive(splitter)
        .zip(stripped.split_inclusive(splitter))
        .enumerate()
    {
        let last_start = orig_span;

        if !diff_set
            .range(last_start..last_start + o.len())
            .any(|_| true)
        {
            continue;
        }
        let original = String::from_utf8_lossy(o);
        let stripped = String::from_utf8_lossy(s);
        ret.push_str(&format!("line number {line_no}\n"));
        ret.push_str(&format!("--- {}\n", o_path.display()));
        ret.push_str(&format!("+++ {}\n", s_path.display()));
        ret.push_str(&format!("+ {stripped}"));
        ret.push_str(&format!("- {original}"));
    }
    if ret.is_empty() {
        return None;
    }
    Some(ret)
}

/// Compare two bytes slices for their tokenized representation to match. excluding
/// all comments
fn generate_diff2(o: &[u8], s: &[u8]) -> Vec<Span> {
    let (os, ot) = get_split_token(o);
    let (_ss, st) = get_split_token(s);
    if ot == st {
        return Vec::new();
    }
    let mut ret = Vec::new();

    for i in 0..ot.len().max(st.len()) {
        let left = ot.get(i);
        let right = st.get(i);
        if left == right {
            continue;
        }
        ret.push(os[i]);
    }
    ret
}

fn get_split_token(l: &[u8]) -> (Vec<Span>, Vec<Token>) {
    let mut spans = Vec::new();
    let mut tokens = Vec::new();
    for t in Lexer::new(l) {
        if matches!(t.token, Token::Comment(_)) {
            continue;
        }
        if matches!(t.token, Token::Unknown(_)) {
            panic!("unknown token {t:#?}")
        }
        spans.push(t.span);
        tokens.push(t.token);
    }
    (spans, tokens)
}
