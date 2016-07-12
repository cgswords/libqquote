#![feature(plugin_registrar, rustc_private)]

extern crate syntax;
extern crate syntax_pos;

use convert::*;
use ::{QDelimited, QTT, Bindings};

use syntax::tokenstream::{self, TokenTree, TokenStream};
use syntax::ext::base::*;
use syntax::ext::base;
use syntax::parse::parser::Parser;
use syntax::parse::token::{self, Token, keywords, gensym_ident, DelimToken, str_to_ident};
use syntax::ptr::P;
use syntax::print::pprust;

pub trait Quotable {
  fn to_appendable(self) -> Vec<TokenTree>;
}

impl Quotable for Token {
  fn to_appendable(self) -> Vec<TokenTree> {
    vec![as_tt(self)]
  }
}

impl Quotable for TokenTree {
  fn to_appendable(self) -> Vec<TokenTree> {
    vec![self]
  }
}

impl Quotable for Vec<TokenTree> {
  fn to_appendable(self) -> Vec<TokenTree> {
    self
  }
}
impl Quotable for TokenStream {
  fn to_appendable(self) -> Vec<TokenTree> {
    self.tts
  }
}


