extern crate syntax;
extern crate syntax_pos;

use ::{QTT, Bindings, DEBUG};
use parse::*;
use build::*;

use syntax::ast::Ident;
use syntax::tokenstream::{TokenTree, TokenStream};
use syntax::ext::base::*;
use syntax::parse::token::{Token, gensym_ident, DelimToken, str_to_ident};

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

fn build_concats(tss: Vec<(TokenStream, bool)>) -> TokenStream {
    let mut pushes : Vec<(TokenStream, bool)> = tss.into_iter().filter(|&(ref ts, _)| !ts.is_empty()).collect();
    let mut output = match pushes.pop() {
      Some((ts, true)) => build_vec_ts(ts),
      Some((ts, false)) => ts,
      None => { return TokenStream::mk_empty(); }
    };

    while let Some((ts, is_tts)) = pushes.pop() {
      output = 
          build_fn_call(
              str_to_ident("concat"), 
              concat(
                  concat(if is_tts { build_vec_ts(ts) } else { ts }, 
                         as_ts(vec![Token::Comma])), 
                  output));
    }
    output
}

pub fn convert_complex_tts<'cx>(cx: &'cx mut ExtCtxt, tts: Vec<QTT>) -> (Bindings, TokenStream) {
    let mut pushes: Vec<(TokenStream, bool)> = Vec::new();
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

                let delim_field = build_struct_field_assign(str_to_ident("delim"), sep);
                let open_sp_field =
                     build_struct_field_assign(str_to_ident("open_span"),
                                               as_ts(vec![str_to_tok_ident("DUMMY_SP")]));
                let tts_field = build_struct_field_assign(str_to_ident("tts"),
                                                          build_method_call(
                                                            new_id,
                                                            str_to_ident("to_tts"),
                                                            TokenStream::mk_empty()));
                let close_sp_field =
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
                pushes.push((TokenStream::from_tts(vec![t]),false));
                pushes.push((TokenStream::mk_empty(), false));
            }
            _ => {
              panic!("Unhandled case!")
            }
        }

    }

    (bindings, build_concats(pushes))
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

pub fn append_last(tts: &mut Vec<(TokenStream, bool)>, to_app: TokenStream) {
    let push_elem = {
      let last = tts.pop();
      match last {
          Some((ts, _)) => (concat(ts, to_app), true),
          None => (to_app, true),
      }
    };
    tts.push(push_elem);
}


