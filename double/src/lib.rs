#![crate_type="dylib"]
#![feature(plugin)]
#![feature(plugin_registrar)]
#![plugin(qquote)]
#![feature(rustc_private)]
#![allow(unused_mut)]
#![allow(unused_parens)]

extern crate rustc_plugin;
extern crate syntax;
extern crate qquote;
// extern crate syntax_pos;

use qquote::convert::{build_paren_delim, ident_eq};
use qquote::quotable::Quotable;
use qquote::{build_emitter};
use syntax::ast::{self, Ident};
use syntax::tokenstream::{self, TokenTree, Delimited, TokenStream};
use syntax::ext::base::*;
use syntax::ext::base;
use syntax::parse::parser::Parser;
use syntax::parse::new_parser_from_ts;
use syntax::parse::token::{self, Token, DelimToken, BinOpToken, Lit};
use syntax::parse::token::{keywords, gensym_ident, str_to_ident};
use syntax::ptr::P;
use syntax::print::pprust;
use std::rc::Rc;

use syntax::codemap::{Span, DUMMY_SP};

use rustc_plugin::Registry;
// use syntax_pos::{mk_sp, Span, DUMMY_SP, ExpnId};

static DEBUG : bool = true;

// ____________________________________________________________________________________________
// Main macro definition

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("double", double);
    reg.register_macro("double2", double2);
    reg.register_macro("cond", cond);
}

fn double<'cx>(cx: &'cx mut ExtCtxt, sp: Span, tts: &[TokenTree]) -> Box<base::MacResult + 'cx> {
    let mut tts1 = build_paren_delim(tts.clone().to_owned());
    let mut tts2 = build_paren_delim(tts.clone().to_owned());

    let output: Vec<TokenTree> = qquote!({ unquote(tts1) + unquote(tts2) });

    { if DEBUG { println!("\nQQ out: {}\n", pprust::tts_to_string(&output[..])); } }

    build_emitter(cx, sp, output)
}

fn double2<'cx>(cx: &'cx mut ExtCtxt, sp: Span, tts: &[TokenTree]) -> Box<base::MacResult + 'cx> {
    build_emitter(cx, sp, qquote!({unquote(tts) * 2}))
}

fn cond<'cx>(cx: &'cx mut ExtCtxt, sp: Span, tts: &[TokenTree]) -> Box<base::MacResult + 'cx> {
    build_emitter(cx, sp, cond_rec(tts.clone().to_owned()))
}

fn cond_rec(input: Vec<TokenTree>) -> Vec<TokenTree> {
  if input.is_empty() { return qquote!(); }

  let next = &input[0..1];
  let rest = &input[1..];

  let clause : Vec<TokenTree> = match *next.get(0).unwrap() {
    TokenTree::Delimited(_, ref dlm) => dlm.tts.clone().to_owned(),
    _ => panic!("Invalid input")
  };

  if clause.len() != 3 { panic!("Not a match: {:?}", clause) } // clause is [test] , [rhs]

  let test: TokenTree = clause.get(0).unwrap().clone().to_owned();
  let rhs: TokenTree  = clause.get(2).unwrap().clone().to_owned();

  if ident_eq(&test, str_to_ident("else")) || rest.is_empty() { 
    qquote!({unquote(rhs)})
  } else {
    qquote!({if unquote(test) { unquote(rhs) } else { cond!(unquote(rest)) } })
  }
}
