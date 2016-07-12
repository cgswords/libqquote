#![feature(plugin_registrar, rustc_private)]

extern crate syntax;
extern crate syntax_pos;

use syntax::parse::parser::Parser;
use syntax::parse::{ParseSess,filemap_to_tts};
use syntax::tokenstream::TokenTree;

/// Map a string to tts, using a made-up filename:
pub fn lex(source_str: &str) -> Vec<TokenTree> {
    let ps = ParseSess::new();
    filemap_to_tts(&ps, ps.codemap().new_filemap("bogofile".to_string(), None, source_str.to_owned()))
}
