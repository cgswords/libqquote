extern crate syntax;

use syntax::parse::{ParseSess,filemap_to_tts};
use syntax::tokenstream::TokenStream;

/// Map a string to tts, using a made-up filename:
pub fn lex(source_str: &str) -> TokenStream {
    let ps = ParseSess::new();
    TokenStream::from_tts(filemap_to_tts(&ps, ps.codemap().new_filemap("bogofile".to_string(), None, source_str.to_owned())))
}
