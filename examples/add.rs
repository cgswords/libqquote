#![feature(plugin)]
#![feature(plugin_registrar)]
#![plugin(qquote)]

#![allow(unused_mut)]
#![allow(unused_parens)]

extern crate rustc_plugin;
extern crate syntax;
// extern crate syntax_pos;

use syntax::ast::{self, Ident};
use syntax::tokenstream::{self, TokenTree};
use syntax::ext::base::*;
use syntax::ext::base;
use syntax::parse::parser::Parser;
use syntax::parse::token::{self, Token, keywords, gensym_ident, DelimToken, str_to_ident, BinOpToken};
use syntax::ptr::P;
use syntax::print::pprust;

use syntax::codemap::{Span, DUMMY_SP};

use rustc_plugin::Registry;
// use syntax_pos::{mk_sp, Span, DUMMY_SP, ExpnId};

static DEBUG : bool = true;

// ____________________________________________________________________________________________
// Main macro definition

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("add", add);
}

fn add<'cx>(cx: &'cx mut ExtCtxt, sp: Span, tts: &[TokenTree]) -> Box<base::MacResult + 'cx> {

    let mut tts : Vec<TokenTree> = tts.clone().to_owned();
    let output : &[TokenTree] = qquote!(unquote(tts) + unquote(tts));
    { if DEBUG { println!("\nQQ out: {}\n", pprust::tts_to_string(&output[..])); } }
    let parser = cx.new_parser_from_tts(&output);

    struct Result<'a> {
        prsr: Parser<'a>,
        span: Span,
    }; //FIXME is this the right lifetime

    impl<'a> Result<'a> {
        fn block(&mut self) -> P<ast::Block> {
            let res = self.prsr.parse_block().unwrap();
            { if DEBUG { println!("\nOutput ast: {:?}\n", res); } }
            res
        }
    }

    impl<'a> base::MacResult for Result<'a> {
        fn make_expr(self: Box<Self>) -> Option<P<ast::Expr>> {
            let mut me = *self;
            Some(P(ast::Expr {
                id: ast::DUMMY_NODE_ID,
                node: ast::ExprKind::Block(me.block()),
                span: me.span,
                attrs: ast::ThinVec::new(),
            }))

        }
    }

    Box::new(Result {
        prsr: parser,
        span: sp,
    })
}
