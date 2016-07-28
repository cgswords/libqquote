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

use qquote::build::{build_paren_delim, build_brace_delim, ident_eq, str_to_tok_ident};
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
    let mut tts1 = build_paren_delim(TokenStream::from_tts(tts.clone().to_owned()));
    let mut tts2 = build_paren_delim(TokenStream::from_tts(tts.clone().to_owned()));

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
    _ => panic!("Invalid input"),
  };

  if clause.len() < 2 { panic!("Invalid macro usage in cond: {:?}", clause) } // clause is ([test]) [rhs]

  let test: TokenTree = clause.get(0).unwrap().clone().to_owned();
  let rhs: Vec<TokenTree> = clause[1..].to_owned();

  if ident_eq(&test, str_to_ident("else")) || rest.is_empty() {
    qquote!({unquote(rhs)})
  } else {
    qquote!({if unquote(test) { unquote(rhs) } else { cond!(unquote(rest)) } })
  }
}

// fn delim_tts(tt: TokenTree) -> Vec<TokenTree> {
//   match tt {
//     TokenTree::Delimited(_, ref dlm) => dlm.tts.clone().to_owned(),
//     _ => panic("Invalid input; expected delimited token but found: {:?}", tt),
//   }
// }
// 
// fn loopc<'cx>(cx: &'cx mut ExtCtxt, sp: Span, tts: &[TokenTree]) -> Box<base::MacResult + 'cx> {
//   if tts.len() < 2 { 
//     panic!("Invalid macro usage in loopc: {:?}, expected `loopc!(((<id> : <type> = <val>)*) : type -> <body>)`", tts)
//   }
// 
//   fn parse_args(tts: &[TokenTree]) -> (Vec<(TokenTree,&[TokenTree])>, Vec<&[TokenTree]> {
//     let mut names_types = Vec::new();
//     let mut vals = Vec::new();
//     let mut rest = tts;
//     loop {
//       if tts.is_empty() {
//         return (names, vals);
//       } else {
//         let next_def = delim_tts(rest[0]);
//         let rest = rest[1..];
//         let parsed = next.iter()
//                          .split(|tt| 
//                              match tt {
//                                  TokenTree::Token(_, Token::Colon) => true,
//                                  TokenTree::Token(_, Token::Eq) => true,
//                              })
//                          .collect::<Vec<&[TokenTree]>>();
//         if parsed.len() != 3 || parsed[0].len() != 1 { 
//           panic!("Invalid input format, expected `(<id : <type> = <val>)`, found: {:?}", parsed); 
//         }
//         name_types.push((parsed[0].get(0).unwrap(), parsed[1]));
//         vals.push(parsed[2]);
//         }
//       }
//     }
// 
//     let args = parse_args(delim_tts(tts[0]));
//     let rest = delim_tts[2..].iter()
//                              .split(|tt|
//                                  match tt {
//                                     TokenTree::Token(_, Token::)
//                                  } 
// 
//     if tts.is_empty() { 
//       Vec::new() 
//     } else {
//       
//     }
//   }
// 
// }
