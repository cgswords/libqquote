#![feature(plugin_registrar, rustc_private)]

extern crate syntax;
extern crate syntax_pos;

use ::{QDelimited, QTT, Bindings, DEBUG};
use quotable::*;
use parse::*;
use build::*;

use syntax::ast::{self, Ident};
use syntax::tokenstream::{self, TokenTree, TokenStream};
use syntax::ext::base::*;
use syntax::ext::base;
use syntax::parse::parser::Parser;
use syntax::parse::token::{self, Token, keywords, gensym_ident, DelimToken, str_to_ident};
use syntax::ptr::P;
use syntax::print::pprust;

use syntax::codemap::{Span, DUMMY_SP};

use std::rc::Rc;

// ____________________________________________________________________________________________
// Datatype Definitions
//  [ as defined in `lib.rs` ]
// 
// struct QDelimited {
//     pub delim: token::DelimToken,
//     pub open_span: Span,
//     pub tts: Vec<QTT>,
//     pub close_span: Span,
// }
// 
// enum QTT {
//     TT(TokenTree),
//     QDL(QDelimited),
//     QIdent(TokenTree),
// }
// 
// type Bindings = Vec<(Ident, Vec<TokenTree>)>;

// ____________________________________________________________________________________________
// Quote Builder

pub fn convert_complex_tts<'cx>(cx: &'cx mut ExtCtxt, tts: Vec<QTT>) -> (Bindings, TokenStream) {
    let mut pushes: Vec<TokenStream> = Vec::new();
    let mut bindings: Bindings = Vec::new();

    let mut iter = tts.into_iter();

    loop {
        let next = iter.next();
        if next.is_none() {
            break;
        }
        let next = next.unwrap();
        match next {
            QTT::TT(TokenTree::Token(_, t)) => {
                let token_out = emit_token(t);
                append_last(&mut pushes, token_out);
                append_last(&mut pushes, lex(","));
            }
            // FIXME handle sequence repetition tokens
            QTT::QDL(qdl) => {
                { if DEBUG { println!("  QDL: {:?} ", qdl.tts); } }
                let new_id = gensym_ident("qdl_tmp");
                let mut cct_rec = convert_complex_tts(cx, qdl.tts);
                bindings.append(&mut cct_rec.0);
                bindings.push((new_id, cct_rec.1));

                let sep = match qdl.delim {
                    DelimToken::Paren => lex("token::DelimToken::Paren"),
                    DelimToken::Brace => lex("token::DelimToken::Brace"),
                    DelimToken::Bracket => lex("token::DelimToken::Bracket"),
                };

                let mut delim_field = build_struct_field_assign(str_to_ident("delim"), sep);
                let mut open_sp_field =
                    build_struct_field_assign(str_to_ident("open_span"),
                                              as_ts(vec![str_to_tok_ident("DUMMY_SP")]));
                let mut tts_field = build_struct_field_assign(str_to_ident("tts"),
                                                              as_ts(vec![Token::Ident(new_id)]));
                let mut close_sp_field =
                    build_struct_field_assign(str_to_ident("close_span"),
                                              as_ts(vec![str_to_tok_ident("DUMMY_SP")]));

                let new_dl = concat(delim_field,
                             concat(open_sp_field,
                             concat(tts_field,
                                    close_sp_field)));

                let rc_arg = concat(as_ts(vec![str_to_tok_ident("Delimited")]), build_brace_delim(new_dl));

                let args = concat(lex("DUMMY_SP,"),
                                  build_mod_call(vec![str_to_ident("Rc"),str_to_ident("new")], rc_arg));

                append_last(&mut pushes, build_mod_call(vec![str_to_ident("TokenTree"),str_to_ident("Delimited")], args));
                append_last(&mut pushes, lex(","));
            }
            QTT::QIdent(t) => {
                pushes.push(TokenStream::from_tts(vec![t]));
                pushes.push(TokenStream::mk_empty());
            }
            _ => {
              panic!("Unhandled case!")
            }
        }

    }

    let output_id = str_to_ident("output");
    if pushes.len() == 1 {
        let mut res = pushes.get(0).unwrap().clone();
        if res.len() > 1 {
          res = build_push_vec(res);
        }
        let res = concat(res, lex(".to_appendable()"));
        (bindings, res)
    } else {
        let push_id = str_to_ident("push");
        let append_id = str_to_ident("append");
        let output = lex("let mut output : Vec<TokenTree> = Vec::new();");
        let mut output = concat(output, as_ts(vec![Token::Semi]));

        for mut ts in pushes.into_iter().filter(|x| x.len() > 0) {
            let mut args = lex("&mut ");
            args = concat(args, build_push_vec(ts));
            args = concat(args, lex(".to_appendable()"));
            let push_vec = build_method_call(output_id, append_id, args);
            output = concat(output, push_vec);
            output = concat(output, as_ts(vec![Token::Semi]));
        }

        output = concat(output, lex("output.to_appendable()"));

        let res = build_brace_delim(output);
        (bindings, res)
    }
}

// ____________________________________________________________________________________________
// Utilities

pub fn unravel(binds: Bindings) -> TokenStream {
    let mut output = TokenStream::mk_empty();

    for b in binds {
        output = concat(output, build_let(b.0, b.1));
    }

    output
}

pub fn is_unquote(id: Ident) -> bool {
    let qq = str_to_ident("unquote");
    id.name == qq.name  // We disregard context; unquote is _reserved_
}

pub fn is_quote(id: Ident) -> bool {
    let qq = str_to_ident("qquote");
    id.name == qq.name  // We disregard context; qquote is _reserved_
}


