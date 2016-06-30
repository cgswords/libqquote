#![feature(plugin_registrar, quote, rustc_private)]

extern crate rustc_plugin;
extern crate syntax;
// extern crate syntax_pos;

use syntax::ast;
use syntax::ast::{Item, MetaItem, TokenTree, LitKind};
use syntax::codemap::{Spanned};
use syntax::ext::base::*;
use syntax::ext::base;
use syntax::ext::quote::rt::{ExtParseUtils, ToTokens};
use syntax::ext::build::AstBuilder;
use syntax::parse;
use syntax::parse::token;
use syntax::ptr::P;
// use syntax::tokenstream::TokenTree;

use syntax::codemap::{mk_sp, Span, DUMMY_SP, ExpnId};

use rustc_plugin::Registry;
use std::rc::Rc;
// use syntax_pos::{mk_sp, Span, DUMMY_SP, ExpnId};

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
  reg.register_syntax_extension(token::intern("qquote"),
                                NormalTT(Box::new(qquote), None, false));
}

fn qquote<'cx>(cx : &'cx mut ExtCtxt, sp : Span, tts : &[TokenTree]) 
  -> Box<base::MacResult+'cx> {
  
  if tts.len() != 1 { return base::DummyResult::expr(sp); }
  let tt = &tts[0];

  match *tt {
    TokenTree::Token(_, token::Literal(lit, suf)) => qquote_lit(cx, sp, lit, suf),
    _ => base::DummyResult::expr(sp)
  }
}

fn qquote_lit<'cx>(cx : &'cx mut ExtCtxt, sp : Span, lit : token::Lit, suf : Option<ast::Name>) 
  -> Box<base::MacResult+'cx> {

  let sess = cx.parse_sess;

  struct Result { lit : ast::LitKind, span : Span };

  impl Result {
    fn lit(&self) -> ast::Lit {
      Spanned { node : self.lit.clone() , span : self.span }
    }
  }

  impl base::MacResult for Result {
    fn make_expr(self: Box<Self>) -> Option<P<ast::Expr>> {
      Some(P(ast::Expr { id : ast::DUMMY_NODE_ID,
                         node : ast::ExprKind::Lit(P(self.lit())),
                         span : self.span,
                         attrs : None,
                       })) 

    }
  }

  let res = match lit {
    token::Byte(i) => LitKind::Byte(parse::byte_lit(&i.as_str()).0),
    token::Char(i) => LitKind::Char(parse::char_lit(&i.as_str()).0),
    // there are some valid suffixes for integer and float literals, so we handle them.
    token::Integer(s) =>
      parse::integer_lit(&s.as_str(),
                         suf.as_ref().map(|s| s.as_str()),
                         &sess.span_diagnostic,
                         sp),
    token::Float(s) => 
      parse::float_lit(&s.as_str(),
                       suf.as_ref().map(|s| s.as_str()),
                       &sess.span_diagnostic,
                       sp),
    token::Str_(s) => 
      LitKind::Str(token::intern_and_get_ident(&parse::str_lit(&s.as_str())),
                   ast::StrStyle::Cooked),
    token::StrRaw(s, n) => 
      LitKind::Str(token::intern_and_get_ident(&parse::raw_str_lit(&s.as_str())),
                   ast::StrStyle::Raw(n)),
    token::ByteStr(i) =>
      LitKind::ByteStr(parse::byte_str_lit(&i.as_str())),
    token::ByteStrRaw(i, _) =>
      LitKind::ByteStr(Rc::new(i.to_string().into_bytes()))
  };

  Box::new(Result { lit : res, span : sp })
}


