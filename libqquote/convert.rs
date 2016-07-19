#![feature(plugin_registrar, rustc_private)]

extern crate syntax;
extern crate syntax_pos;

use ::{QDelimited, QTT, Bindings,DEBUG};
use quotable::*;
use parse::*;

use syntax::ast::{self, Ident};
use syntax::tokenstream::{self, TokenTree};
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
                append_last(&mut pushes, lex(","));
            }
            // FIXME handle sequence repetition tokens
            QTT::QDL(qdl) => {
                { if DEBUG { println!("  QDL: {:?} ", qdl.tts); } }
                let new_id = gensym_ident("qdl_tmp");
                let mut cct_rec = convert_complex_tts(cx, qdl.tts);
                bindings.append(&mut cct_rec.0);
                bindings.push((new_id, cct_rec.1));

                let mut new_dl = Vec::new();

                let sep = {
                    let mut output = lex("token::DelimToken::");
                    match qdl.delim {
                        DelimToken::Paren => {
                            output.push(str_to_tt_ident("Paren"));
                        }
                        DelimToken::Brace => {
                            output.push(str_to_tt_ident("Brace"));
                        }
                        DelimToken::Bracket => {
                            output.push(str_to_tt_ident("Bracket"));
                        }
                    };
                    output
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

                let mut args = lex("DUMMY_SP,");
                let mut rc_arg = vec![str_to_tt_ident("Delimited")];
                rc_arg.push(build_brace_delim(new_dl));

                args.append(&mut build_mod_call(vec![str_to_ident("Rc"),str_to_ident("new")], rc_arg));

                append_last(&mut pushes, build_mod_call(vec![str_to_ident("TokenTree"),str_to_ident("Delimited")], args));
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
        let mut res = pushes.get(0).unwrap().clone();
        if res.len() > 1 {
          res = build_push_vec(res);
        }
        res.append(&mut lex(".to_appendable()"));
        (bindings, res)
    } else {
        let push_id = str_to_ident("push");
        let append_id = str_to_ident("append");
        let mut output = lex("let mut output : Vec<TokenTree> = Vec::new();");
        output.push(as_tt(Token::Semi));

        for mut tts in pushes.into_iter().filter(|x| x.len() > 0) {
            let mut args = lex("&mut ");
            args.append(&mut build_push_vec(tts));
            args.append(&mut lex(".to_appendable()"));
            let mut push_vec = build_method_call(output_id, append_id, args);
            output.append(&mut push_vec);
            output.push(as_tt(Token::Semi));
        }

        output.append(&mut lex("output.to_appendable()"));

        let res = vec![build_brace_delim(output)];
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
  let mut output = Vec::new();
  output.append(&mut lex("TokenTree::Token"));

  let mut del = Vec::new();
  del.push(str_to_tt_ident("DUMMY_SP"));
  del.push(TokenTree::Token(DUMMY_SP, Token::Comma));

  let mut tbuild = build_token_tt(t);
  del.append(&mut tbuild);

  output.push(build_paren_delim(del));

  output
}

pub fn emit_lit(l: token::Lit, n: Option<ast::Name>) -> Vec<TokenTree> {
  let suf = match n {
              Some(n) => format!("Some(ast::Name({}))", n.0),
              None    => "None".to_string(),
            };

  let lit = match l {
              token::Lit::Byte(n)    => format!("Lit::Byte(token::intern(\"{}\"))", n.to_string()),
              token::Lit::Char(n)    => format!("Lit::Char(token::intern(\"{}\"))", n.to_string()),
              token::Lit::Integer(n) => format!("Lit::Integer(token::intern(\"{}\"))", n.to_string()),
              token::Lit::Float(n)   => format!("Lit::Float(token::intern(\"{}\"))", n.to_string()),
              token::Lit::Str_(n)    => format!("Lit::Str_(token::intern(\"{}\"))", n.to_string()),
              token::Lit::ByteStr(n) => format!("Lit::ByteStr(token::intern(\"{}\"))", n.to_string()),
              _ => panic!("Unsupported literal"), 
            };

  let res = format!("Token::Literal({},{})",lit, suf);
  { println!("{}", res); }
  lex(&res)
}

pub fn build_binop_tok(bot: token::BinOpToken) -> Vec<TokenTree> {
  match bot {
    token::BinOpToken::Plus    => lex("Token::BinOp(BinOpToken::Plus)"),
    token::BinOpToken::Minus   => lex("Token::BinOp(BinOpToken::Minus)"),
    token::BinOpToken::Star    => lex("Token::BinOp(BinOpToken::Star)"),
    token::BinOpToken::Slash   => lex("Token::BinOp(BinOpToken::Slash)"),
    token::BinOpToken::Percent => lex("Token::BinOp(BinOpToken::Percent)"),
    token::BinOpToken::Caret   => lex("Token::BinOp(BinOpToken::Caret)"),
    token::BinOpToken::And     => lex("Token::BinOp(BinOpToken::And)"),
    token::BinOpToken::Or      => lex("Token::BinOp(BinOpToken::Or)"),
    token::BinOpToken::Shl     => lex("Token::BinOp(BinOpToken::Shl)"),
    token::BinOpToken::Shr     => lex("Token::BinOp(BinOpToken::Shr)"),
  }
}

pub fn build_binopeq_tok(bot: token::BinOpToken) -> Vec<TokenTree> {
  match bot {
    token::BinOpToken::Plus    => lex("Token::BinOpEq(BinOpToken::Plus)"),
    token::BinOpToken::Minus   => lex("Token::BinOpEq(BinOpToken::Minus)"),
    token::BinOpToken::Star    => lex("Token::BinOpEq(BinOpToken::Star)"),
    token::BinOpToken::Slash   => lex("Token::BinOpEq(BinOpToken::Slash)"),
    token::BinOpToken::Percent => lex("Token::BinOpEq(BinOpToken::Percent)"),
    token::BinOpToken::Caret   => lex("Token::BinOpEq(BinOpToken::Caret)"),
    token::BinOpToken::And     => lex("Token::BinOpEq(BinOpToken::And)"),
    token::BinOpToken::Or      => lex("Token::BinOpEq(BinOpToken::Or)"),
    token::BinOpToken::Shl     => lex("Token::BinOpEq(BinOpToken::Shl)"),
    token::BinOpToken::Shr     => lex("Token::BinOpEq(BinOpToken::Shr)"),
  }
}

pub fn build_delim_tok(dt: token::DelimToken) -> Vec<TokenTree> {
  match dt {
    token::DelimToken::Paren   => lex("DelimToken::Paren"),
    token::DelimToken::Bracket => lex("DelimToken::Bracket"),
    token::DelimToken::Brace   => lex("DelimToken::Brace"),
  }
}

pub fn build_token_tt(t: Token) -> Vec<TokenTree> {
  match t
  { Token::Eq           => lex("Token::Eq"),
    Token::Lt           => lex("Token::Lt"),
    Token::Le           => lex("Token::Le"),
    Token::EqEq         => lex("Token::EqEq"),
    Token::Ne           => lex("Token::Ne"),
    Token::Ge           => lex("Token::Ge"),
    Token::Gt           => lex("Token::Gt"),
    Token::AndAnd       => lex("Token::AndAnd"),
    Token::OrOr         => lex("Token::OrOr"),
    Token::Not          => lex("Token::Not"),
    Token::Tilde        => lex("Token::Tilde"),
    Token::BinOp(tok)   => build_binop_tok(tok),
    Token::BinOpEq(tok) => build_binopeq_tok(tok),
    Token::At           => lex("Token::At"),
    Token::Dot          => lex("Token::Dot"),
    Token::DotDot       => lex("Token::DotDot"),
    Token::DotDotDot    => lex("Token::DotDotDot"),
    Token::Comma        => lex("Token::Comma"),
    Token::Semi         => lex("Token::Semi"),
    Token::Colon        => lex("Token::Colon"),
    Token::ModSep       => lex("Token::ModSep"),
    Token::RArrow       => lex("Token::RArrow"),
    Token::LArrow       => lex("Token::LArrow"),
    Token::FatArrow     => lex("Token::FatArrow"),
    Token::Pound        => lex("Token::Pound"),
    Token::Dollar       => lex("Token::Dollar"),
    Token::Question     => lex("Token::Question"),
    Token::OpenDelim(dt) => match dt {
      token::DelimToken::Paren   => lex("Token::OpenDelim(DelimToken::Paren)"), 
      token::DelimToken::Bracket => lex("Token::OpenDelim(DelimToken::Bracket)"), 
      token::DelimToken::Brace   => lex("Token::OpenDelim(DelimToken::Brace)"), 
    },
    Token::CloseDelim(dt) => match dt {
      token::DelimToken::Paren   => lex("Token::CloseDelim(DelimToken::Paren)"), 
      token::DelimToken::Bracket => lex("Token::CloseDelim(DelimToken::Bracket)"), 
      token::DelimToken::Brace   => lex("Token::CloseDelim(DelimToken::Brace)"), 
    },
    Token::Underscore => lex("_"),
    Token::Literal(lit, sfx) => emit_lit(lit, sfx),
    // fix ident expansion information... somehow
    Token::Ident(ident) => lex(&format!("Token::Ident(str_to_ident(\"{}\"))",ident.name)),
    Token::Lifetime(ident) => lex(&format!("Token::Ident(str_to_ident(\"{}\"))",ident.name)),
    _ => panic!("Unhandled case!"),
  }
}

pub fn build_push_vec(tts: Vec<TokenTree>) -> Vec<TokenTree> {
    build_mac_call(str_to_ident("vec"), tts)
    //tts.clone().to_owned()
}

pub fn build_paren_delim(tts: Vec<TokenTree>) -> TokenTree {
  TokenTree::Delimited(
    DUMMY_SP,
    Rc::new(tokenstream::Delimited {
      delim: token::DelimToken::Paren,
      open_span: DUMMY_SP,
      tts: tts,
      close_span: DUMMY_SP,
    }))
}

pub fn build_brace_delim(tts: Vec<TokenTree>) -> TokenTree {
  TokenTree::Delimited(
    DUMMY_SP,
    Rc::new(tokenstream::Delimited {
      delim: token::DelimToken::Brace,
      open_span: DUMMY_SP,
      tts: tts,
      close_span: DUMMY_SP,
    }))
}

pub fn build_bracket_delim(tts: Vec<TokenTree>) -> TokenTree {
  TokenTree::Delimited(
    DUMMY_SP,
    Rc::new(tokenstream::Delimited {
      delim: token::DelimToken::Bracket,
      open_span: DUMMY_SP,
      tts: tts,
      close_span: DUMMY_SP,
    }))
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

pub fn build_mod_call(ids: Vec<Ident>, args: Vec<TokenTree>) -> Vec<TokenTree> {
    let mut output = intersperse(ids.into_iter().map(|id| as_tt(Token::Ident(id))).collect(),
                                 TokenTree::Token(DUMMY_SP, Token::ModSep));
    output.append(&mut vec![build_paren_delim(args)]);
    output
}

pub fn build_fn_call(name: Ident, args: Vec<TokenTree>) -> Vec<TokenTree> {
    let mut output = as_tts(vec![Token::Ident(name)]);
    output.push(build_paren_delim(args));
    output
}

pub fn build_mac_call(name: Ident, args: Vec<TokenTree>) -> Vec<TokenTree> {
    let mut output = as_tts(vec![Token::Ident(name),Token::Not]);
    output.push(build_paren_delim(args));
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

pub fn ident_eq(tident : &TokenTree, id: Ident) -> bool {
  let tid = match *tident {
    TokenTree::Token(_, Token::Ident(ref id)) => id,
    _ => { return false; }
  };

  tid.name == id.name // We disregard context for this; it's for 'reserved' keywords
  // A good implementation should do something... way smarter.
}

// ____________________________________________________________________________________________
// Vector Utilities

pub fn push_last<T>(tts: &mut Vec<Vec<T>>, tt: T) {
    if tts.is_empty() {
        tts.push(vec![tt]);
    } else {
        tts.last_mut().unwrap().push(tt);
    }
}

pub fn last_empty<T>(tts: &Vec<Vec<T>>) -> bool {
  tts.is_empty() || tts.last().unwrap().is_empty()
}

pub fn append_last<T>(tts: &mut Vec<Vec<T>>, mut ts: Vec<T>) {
    if tts.is_empty() {
        tts.push(ts);
    } else {
        tts.last_mut().unwrap().append(&mut ts);
    }
}

pub fn intersperse<T>(vs : Vec<T>, t : T) -> Vec<T> where T : Clone { 
  if vs.len() < 2 { return vs; }
  let mut output = vec![vs.get(0).unwrap().to_owned()];

  for v in vs.into_iter().skip(1) {
    output.push(t.clone());
    output.push(v);
  }
  output
}
