#![feature(plugin_registrar, quote, rustc_private)]

extern crate syntax;

use ::{QDelimited, QTT, Bindings};

use syntax::ast::{self, Ident};
use syntax::tokenstream::{self, TokenTree};
use syntax::ext::base::*;
use syntax::ext::base;
use syntax::parse::parser::Parser;
use syntax::parse::token::{self, Token, keywords, gensym_ident, DelimToken, str_to_ident};
use syntax::ptr::P;
use syntax::print::pprust;

use syntax::codemap::{Span, DUMMY_SP};

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

pub fn convert_complex_tts<'cx>(cx: &'cx mut ExtCtxt, tts: Vec<QTT>) -> (Bindings, Vec<TokenTree>) {
    let mut pushes: Vec<Vec<TokenTree>> = Vec::new();
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
            }
            // FIXME handle sequence repetition tokens
            QTT::QDL(qdl) => {
                let new_id = gensym_ident("qdl_tmp");
                let mut cct_rec = convert_complex_tts(cx, qdl.tts);
                bindings.append(&mut cct_rec.0);
                bindings.push((new_id, cct_rec.1));

                let mut new_dl = Vec::new();

                let sep = {
                    let mut output = vec![str_to_tok_ident("token"),
                                          Token::ModSep,
                                          str_to_tok_ident("DelimToken"),
                                          Token::ModSep];
                    match qdl.delim {
                        DelimToken::Paren => {
                            output.push(str_to_tok_ident("Paren"));
                        }
                        DelimToken::Brace => {
                            output.push(str_to_tok_ident("Brace"));
                        }
                        DelimToken::Bracket => {
                            output.push(str_to_tok_ident("Bracket"));
                        }
                    };
                    as_tts(output)
                };

                let mut delim_field = build_struct_field_assign(str_to_ident("delim"), sep);
                let mut open_sp_field =
                    build_struct_field_assign(str_to_ident("open_span"),
                                              as_tts(vec![str_to_tok_ident("DUMMY_SP")]));
                let mut tts_field = build_struct_field_assign(str_to_ident("tts"),
                                                              as_tts(vec![Token::Ident(new_id)]));
                let mut close_sp_field =
                    build_struct_field_assign(str_to_ident("close_span"),
                                              as_tts(vec![str_to_tok_ident("DUMMY_SP")]));

                new_dl.append(&mut delim_field);
                new_dl.append(&mut open_sp_field);
                new_dl.append(&mut tts_field);
                new_dl.append(&mut close_sp_field);

                let mut dl = vec![];
                dl.push(as_tt(str_to_tok_ident("Delimited")));
                dl.push(TokenTree::Delimited(DUMMY_SP,
                                             tokenstream::Delimited {
                                                 delim: DelimToken::Brace,
                                                 open_span: DUMMY_SP,
                                                 tts: new_dl,
                                                 close_span: DUMMY_SP,
                                             }));
                append_last(&mut pushes, dl);
            }
            QTT::QIdent(t) => {
                pushes.push(vec![t]);
                pushes.push(vec![]);
            }
            _ => {
              panic!("Unhandled case!")
            }
        }

    }

    let output_id = str_to_ident("output");
    if pushes.len() == 1 {
        let res = vec![TokenTree::Delimited(DUMMY_SP,
                                            tokenstream::Delimited {
                                                delim: token::DelimToken::Brace,
                                                open_span: DUMMY_SP,
                                                tts: build_push_vec(&pushes.get(0).unwrap()[..]),
                                                close_span: DUMMY_SP,
                                            })];
        (bindings, res)
    } else {
        let push_id = str_to_ident("push");
        let mut output = as_tts(vec![kw_to_tok_ident(keywords::Let),
                                     kw_to_tok_ident(keywords::Mut),
                                     Token::Ident(output_id),
                                     Token::Eq,
                                     str_to_tok_ident("Vec"),
                                     Token::ModSep,
                                     str_to_tok_ident("new")]);
        output.push(build_empty_args());
        output.push(as_tt(Token::Semi));

        for tts in pushes.into_iter().filter(|x| x.len() > 0) {
            let mut push_vec = build_method_call(output_id, push_id, build_push_vec(&tts[..]));
            output.append(&mut push_vec);
            output.push(as_tt(Token::Semi));
        }

        output.push(as_tt(Token::Ident(output_id)));

        let res = vec![TokenTree::Delimited(DUMMY_SP,
                                            tokenstream::Delimited {
                                                delim: token::DelimToken::Brace,
                                                open_span: DUMMY_SP,
                                                tts: output,
                                                close_span: DUMMY_SP,
                                            })];
        (bindings, res)
    }
}

// ____________________________________________________________________________________________
// Utilities

// NB I need to do a grotesque amount of unraveling here.
// For example, the TT `TokenTree::Token(span, Token::Eq)` needs to *look* like that in the 
// output stream.
pub fn emit_token(t: Token) -> Vec<TokenTree> {
  // FIXME do something nicer with the spans
  let modsep = TokenTree::Token(DUMMY_SP, Token::ModSep);
  let mut output = Vec::new();
  output.push(str_to_tt_ident("TokenTree"));
  output.push(modsep);
  output.push(str_to_tt_ident("Token"));

  let mut del = Vec::new();
  del.push(str_to_tt_ident("DUMMY_SP"));
  del.push(TokenTree::Token(DUMMY_SP, Token::Comma));

  let mut tbuild = build_token_tt(t);
  del.append(&mut tbuild);

  output.push(build_paren_delim(del));

  output
}

pub fn build_binop_tok(bot: token::BinOpToken) -> Vec<TokenTree> {
  let mut output = as_tts(vec![str_to_tok_ident("BinOpToken"), Token::ModSep]);

  match bot {
    token::BinOpToken::Plus    => { output.push(str_to_tt_ident("Plus")); }
    token::BinOpToken::Minus   => { output.push(str_to_tt_ident("Minus")); }
    token::BinOpToken::Star    => { output.push(str_to_tt_ident("Star")); }
    token::BinOpToken::Slash   => { output.push(str_to_tt_ident("Slash")); }
    token::BinOpToken::Percent => { output.push(str_to_tt_ident("Percent")); }
    token::BinOpToken::Caret   => { output.push(str_to_tt_ident("Caret")); }
    token::BinOpToken::And     => { output.push(str_to_tt_ident("And")); }
    token::BinOpToken::Or      => { output.push(str_to_tt_ident("Or")); }
    token::BinOpToken::Shl     => { output.push(str_to_tt_ident("Shl")); }
    token::BinOpToken::Shr     => { output.push(str_to_tt_ident("Shr")); }
  }
  output
}

pub fn build_delim_tok(dt: token::DelimToken) -> Vec<TokenTree> {
  as_tts(vec![str_to_tok_ident("DelimToken"), Token::ModSep, 
              match dt {
                token::DelimToken::Paren   => str_to_tok_ident("Paren"),
                token::DelimToken::Bracket => str_to_tok_ident("Bracket"),
                token::DelimToken::Brace   => str_to_tok_ident("Brace"),
              }
             ])
}

pub fn build_token_tt(t: Token) -> Vec<TokenTree> {
  let mut output = as_tts(vec![str_to_tok_ident("Token"), Token::ModSep]);

  match t
  { Token::Eq     => { output.push(str_to_tt_ident("Eq")); }
    Token::Lt     => { output.push(str_to_tt_ident("Lt")); }
    Token::Le     => { output.push(str_to_tt_ident("Le")); }
    Token::EqEq   => { output.push(str_to_tt_ident("EqEq")); }
    Token::Ne     => { output.push(str_to_tt_ident("Ne")); }
    Token::Ge     => { output.push(str_to_tt_ident("Ge")); }
    Token::Gt     => { output.push(str_to_tt_ident("Gt")); }
    Token::AndAnd => { output.push(str_to_tt_ident("AndAnd")); }
    Token::OrOr   => { output.push(str_to_tt_ident("OrOr")); }
    Token::Not    => { output.push(str_to_tt_ident("Not")); }
    Token::Tilde  => { output.push(str_to_tt_ident("Tilde")); }
    Token::BinOp(tok) => { 
      let mut build = build_fn_call(str_to_ident("BinOp"), build_binop_tok(tok));
      output.append(&mut build);
    }
    Token::BinOpEq(tok) => { 
      let mut build = build_fn_call(str_to_ident("BinOpEq"), build_binop_tok(tok));
      output.append(&mut build);
    }
    Token::At        => { output.push(str_to_tt_ident("At")); }
    Token::Dot       => { output.push(str_to_tt_ident("Dot")); }
    Token::DotDot    => { output.push(str_to_tt_ident("DotDot")); }
    Token::DotDotDot => { output.push(str_to_tt_ident("DotDotDot")); }
    Token::Comma     => { output.push(str_to_tt_ident("Comma")); }
    Token::Semi      => { output.push(str_to_tt_ident("Semi")); }
    Token::Colon     => { output.push(str_to_tt_ident("Colon")); }
    Token::ModSep    => { output.push(str_to_tt_ident("ModSep")); }
    Token::RArrow    => { output.push(str_to_tt_ident("RArrow")); }
    Token::LArrow    => { output.push(str_to_tt_ident("LArrow")); }
    Token::FatArrow  => { output.push(str_to_tt_ident("FatArrow")); }
    Token::Pound     => { output.push(str_to_tt_ident("Pound")); }
    Token::Dollar    => { output.push(str_to_tt_ident("Dollar")); }
    Token::Question  => { output.push(str_to_tt_ident("Question")); }

    Token::OpenDelim(dt) => { 
      let mut build = build_fn_call(str_to_ident("OpenDelim"), build_delim_tok(dt));
      output.append(&mut build);
    }
    Token::CloseDelim(dt) => { 
      let mut build = build_fn_call(str_to_ident("CloseDelim"), build_delim_tok(dt));
      output.append(&mut build);
    }
    // FIXME finish this block
    // /* Literals */
    // Literal(Lit, Option<ast::Name>),

    // /* Name components */
    // Ident(ast::Ident),
    // Token::Underscore => { output.push(str_to_ident("Underscore")); }
    // Lifetime(ast::Ident),
    _ => {
      panic!("Unhandled case!")
    }
  }
  output
}

pub fn build_push_vec(tts: &[TokenTree]) -> Vec<TokenTree> {
    // FIXME this is wrong
    tts.clone().to_owned()
}

pub fn build_paren_delim(tts: Vec<TokenTree>) -> TokenTree {
  TokenTree::Delimited(
    DUMMY_SP,
    tokenstream::Delimited {
      delim: token::DelimToken::Paren,
      open_span: DUMMY_SP,
      tts: tts,
      close_span: DUMMY_SP,
    })
}

pub fn str_to_tt_ident(s: &str) -> TokenTree {
  TokenTree::Token(DUMMY_SP, str_to_tok_ident(s))
}

pub fn str_to_tok_ident(s: &str) -> Token {
    Token::Ident(str_to_ident(s))
}

pub fn kw_to_tok_ident(kw: keywords::Keyword) -> Token {
    Token::Ident(str_to_ident(&kw.name().as_str()[..]))
}


pub fn unravel(binds: Bindings) -> Vec<TokenTree> {
    let mut output = Vec::new();

    for b in binds {
        let name = b.0;
        let rhs = b.1;
        let mut new_let = build_let(name, rhs);
        output.append(&mut new_let);
    }

    output
}

pub fn as_tts(ts: Vec<Token>) -> Vec<TokenTree> {
    ts.into_iter().map(as_tt).collect()
}

pub fn as_tt(t: Token) -> TokenTree {
    TokenTree::Token(DUMMY_SP, t)
}

pub fn build_empty_args() -> TokenTree {
  build_paren_delim(vec![])
}

pub fn build_struct_field_assign(field: Ident, mut rhs: Vec<TokenTree>) -> Vec<TokenTree> {
    let id = as_tt(Token::Ident(field));
    let mut output = Vec::new();
    output.push(id);
    output.push(as_tt(Token::Colon));
    output.append(&mut rhs);
    output.push(as_tt(Token::Comma));
    output
}

pub fn build_let(id: Ident, mut tts: Vec<TokenTree>) -> Vec<TokenTree> {
    let mut output = as_tts(vec![kw_to_tok_ident(keywords::Let), Token::Ident(id), Token::Eq]);
    output.append(&mut tts);
    output.push(as_tt(Token::Semi));
    output
}

pub fn build_method_call(id: Ident, mthd: Ident, args: Vec<TokenTree>) -> Vec<TokenTree> {
    let mut output = as_tts(vec![Token::Ident(id), Token::Dot]);
    let mut call = build_fn_call(mthd, args);
    output.append(&mut call);
    output
}

pub fn build_fn_call(name: Ident, args: Vec<TokenTree>) -> Vec<TokenTree> {
    let mut output = as_tts(vec![Token::Ident(name)]);
    output.push(build_paren_delim(args));
    output
}

pub fn push_last(tts: &mut Vec<Vec<TokenTree>>, tt: TokenTree) {
    if tts.is_empty() {
        tts.push(vec![tt]);
    } else {
        tts.last_mut().unwrap().push(tt);
    }
}

pub fn append_last(tts: &mut Vec<Vec<TokenTree>>, mut ts: Vec<TokenTree>) {
    if tts.is_empty() {
        tts.push(ts);
    } else {
        tts.last_mut().unwrap().append(&mut ts);
    }
}

pub fn is_unquote(id: Ident) -> bool {
    let qq = str_to_ident("unquote");
    id.name == qq.name  // We disregard context; unquote is _reserved_
}

pub fn is_quote(id: Ident) -> bool {
    let qq = str_to_ident("qquote");
    id.name == qq.name  // We disregard context; qquote is _reserved_
}

