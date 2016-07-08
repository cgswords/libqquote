#![feature(plugin)]
#![plugin(qquote)]
#![feature(rustc_private)]

extern crate qquote;
extern crate syntax;

use syntax::ast::{self, TokenTree, Ident};
use syntax::ext::base::*;
use syntax::ext::base;
use syntax::parse::parser::Parser;
use syntax::parse::token::{self, Token, keywords, gensym_ident, DelimToken, str_to_ident};
use syntax::ptr::P;
// use syntax::tokenstream::TokenTree;

use syntax::codemap::{Span, DUMMY_SP};

#[test]
fn test_lit_1() -> () {
  assert_eq!(5 + qquote!(5), 10)
}


#[test]
fn test_lit_2() -> () {
  assert_eq!(5.1 + qquote!(5.1), 10.2)
}

#[test]
fn test_lit_3() -> () {
  assert_eq!(qquote!("foo"), "foo")
}

#[test]
#[should_panic]
fn test_lit_4() -> () {
  assert_eq!(qquote!("foob"), "foo")
}

#[test]
fn test_ident_1() -> () {
  let foo = 5;
  assert_eq!(qquote!(foo), 5)
}

#[test]
fn test_ident_2() -> () {
  let foo = 5;
  let res = qquote!(~foo);
  if res {
    assert_eq!(5, 5)
  } else {
    assert_eq!(5, 8)
  }
}

// #[test]
// fn test_exp_1() -> () {
//   let foo = 5;
//   let bar = 7;
//   let res : i32 = qquote!(~foo + ~bar);
//   assert_eq!(res, 12)
// }
