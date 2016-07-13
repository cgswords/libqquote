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

use qquote::convert::build_paren_delim;
use qquote::quotable::Quotable;
use syntax::ast::{self, Ident};
use syntax::tokenstream::{self, TokenTree, Delimited};
use syntax::ext::base::*;
use syntax::ext::base;
use syntax::parse::parser::Parser;
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
}

fn double<'cx>(cx: &'cx mut ExtCtxt, sp: Span, tts: &[TokenTree]) -> Box<base::MacResult + 'cx> {
    let mut tts1 = build_paren_delim(tts.clone().to_owned());
    let mut tts2 = build_paren_delim(tts.clone().to_owned());

    // let output: Vec<TokenTree> = qquote!({ unquote(tts1) + unquote(tts2) });

    let output : Vec<TokenTree> = { 
     let tmp = ( tts ) ; let qdl_tmp = {
       let mut output = Vec :: new (  ) ; ; output . append (
       & mut vec ! ( tmp ) . to_appendable (  ) ) ; output . append (
       & mut vec ! (
       TokenTree :: Token ( DUMMY_SP , Token :: BinOp ( BinOpToken :: Star ) ) ,
       TokenTree :: Token (
       DUMMY_SP , Token :: Literal ( Lit :: Integer ( ast :: Name ( 148 ) ) , None )
       ) , ) . to_appendable (  ) ) ; output . to_appendable (  ) } ; 
       
       let res = vec ! ( TokenTree :: Delimited ( DUMMY_SP , Rc :: new ( Delimited {
         delim : token :: DelimToken :: Brace , open_span : DUMMY_SP , tts : qdl_tmp ,
         close_span : DUMMY_SP , } ) ));
       
       res . to_appendable (  )
    };

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

fn double2<'cx>(cx: &'cx mut ExtCtxt, sp: Span, tts: &[TokenTree]) -> Box<base::MacResult + 'cx> {
    let tts = tts.clone().to_owned();

    let output: Vec<TokenTree> = qquote!({ unquote(tts) * 2 });

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
