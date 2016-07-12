#![feature(plugin_registrar, quote, rustc_private)]

extern crate rustc_plugin;
extern crate syntax;
// extern crate syntax_pos;

pub mod convert;
use convert::*;

pub mod quotable;
pub mod parse;

use syntax::ast::{self, Ident};
use syntax::tokenstream::{self, TokenTree, Delimited};
use syntax::ext::base::*;
use syntax::ext::base;
use syntax::parse::parser::Parser;
use syntax::parse::token::{self, Token, keywords, gensym_ident, DelimToken, str_to_ident};
use syntax::ptr::P;
use syntax::print::pprust;

use syntax::codemap::{Span, DUMMY_SP};

use rustc_plugin::Registry;
// use syntax_pos::{mk_sp, Span, DUMMY_SP, ExpnId};

use std::rc::Rc;

static DEBUG : bool = true;

// ____________________________________________________________________________________________
// Main macro definition

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("qquote", qquote);
}

fn qquote<'cx>(cx: &'cx mut ExtCtxt, sp: Span, tts: &[TokenTree]) -> Box<base::MacResult + 'cx> {

    let output = qquoter(cx, tts);
    { if DEBUG { println!("\nQQ out: {}\n", pprust::tts_to_string(&output[..])); } }
    let parser = cx.new_parser_from_tts(&output);

    struct Result<'a> {
        prsr: Parser<'a>,
        span: Span,
    };

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


// ____________________________________________________________________________________________
// Datatype Definitions

pub struct QDelimited {
    pub delim: token::DelimToken,
    pub open_span: Span,
    pub tts: Vec<QTT>,
    pub close_span: Span,
}

pub enum QTT {
    TT(TokenTree),
    QDL(QDelimited),
    QIdent(TokenTree),
}

pub type Bindings = Vec<(Ident, Vec<TokenTree>)>;

// ____________________________________________________________________________________________
// Quasiquoter Algorithm

fn qquoter<'cx>(cx: &'cx mut ExtCtxt, tts: &[TokenTree]) -> Vec<TokenTree> {
    let qq_res = qquote_iter(cx, 0, tts.clone().to_owned());
    let mut bindings = qq_res.0;
    let body = qq_res.1;
    let mut cct_res = convert_complex_tts(cx, body);

    bindings.append(&mut cct_res.0);

    let output = if bindings.is_empty() {
                   cct_res.1
                 } else {
                   let mut bindings = unravel(bindings);
                   let mut output = Vec::new();
                   output.append(&mut bindings);
                   output.append(&mut cct_res.1);
                   output
                 };

    vec![build_brace_delim(output)]
}

fn qquote_iter<'cx>(cx: &'cx mut ExtCtxt, depth: i64, tts: Vec<TokenTree>) -> (Bindings, Vec<QTT>) {
    let mut depth = depth;
    let mut bindings: Bindings = Vec::new();
    let mut output: Vec<QTT> = Vec::new();

    let mut iter = tts.into_iter();

    loop {
        let next = iter.next();
        if next.is_none() {
            break;
        }
        let next = next.unwrap();
        match next {
            TokenTree::Token(_, Token::Ident(id)) if is_unquote(id) => {
                if depth == 0 {
                    let exp = iter.next();
                    if exp.is_none() {
                        break;
                    } // produce an error or something first
                    let exp = vec![exp.unwrap().to_owned()];

                    let new_id = gensym_ident("tmp");
                    bindings.push((new_id, exp));
                    { println!("Bindings: {:?}", bindings); }
                    output.push(QTT::QIdent(as_tt(Token::Ident(new_id.clone()))));
                } else {
                    depth = depth - 1;
                    output.push(QTT::TT(next.clone()));
                }
            }
            TokenTree::Token(_, Token::Ident(id)) if is_quote(id) => {
                depth = depth + 1;
            }
            TokenTree::Delimited(_, ref dl) => {
                let br = qquote_iter(cx, depth, dl.tts.clone().to_owned());
                let mut bind_ = br.0;
                let res_ = br.1;
                bindings.append(&mut bind_);

                let new_dl = QDelimited {
                    delim: dl.delim,
                    open_span: dl.open_span,
                    tts: res_,
                    close_span: dl.close_span,
                };

                output.push(QTT::QDL(new_dl));
            }
            t => {
                output.push(QTT::TT(t));
            }
        }
    }

    (bindings, output)
}

