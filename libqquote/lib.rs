#![feature(plugin_registrar, quote, rustc_private)]

extern crate rustc_plugin;
extern crate syntax;
// extern crate syntax_pos;

use syntax::ast::{self, Ident};
use syntax::tokenstream::{self, TokenTree};
use syntax::ext::base::*;
use syntax::ext::base;
use syntax::parse::parser::Parser;
use syntax::parse::token::{self, Token, keywords, gensym_ident, DelimToken, str_to_ident};
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
    reg.register_macro("qquote", qquote);
}

fn qquote<'cx>(cx: &'cx mut ExtCtxt, sp: Span, tts: &[TokenTree]) -> Box<base::MacResult + 'cx> {

    let output = qquoter(cx, tts);
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


// ____________________________________________________________________________________________
// Datatype Definitions

struct QDelimited {
    pub delim: token::DelimToken,
    pub open_span: Span,
    pub tts: Vec<QTT>,
    pub close_span: Span,
}

enum QTT {
    TT(TokenTree),
    QDL(QDelimited),
    QIdent(TokenTree),
}

type Bindings = Vec<(Ident, Vec<TokenTree>)>;

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

    vec![TokenTree::Delimited(DUMMY_SP,
                              tokenstream::Delimited {
                                  delim: token::DelimToken::Brace,
                                  open_span: DUMMY_SP,
                                  tts: output,
                                  close_span: DUMMY_SP,
                              })]
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

// ____________________________________________________________________________________________
// Quote Builder

fn convert_complex_tts<'cx>(cx: &'cx mut ExtCtxt, tts: Vec<QTT>) -> (Bindings, Vec<TokenTree>) {
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
            QTT::TT(t) => {
                // I may need to do a grotesque about of unraveling here.
                // For example, the TT `TokenTree::Token(span, Token::Eq)`
                // may need to actually emit all of that...
                push_last(&mut pushes, t);
            }
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

fn build_push_vec(tts: &[TokenTree]) -> Vec<TokenTree> {
    // FIXME this is wrong
    tts.clone().to_owned()
}

fn str_to_tok_ident(s: &str) -> Token {
    Token::Ident(str_to_ident(s))
}

fn kw_to_tok_ident(kw: keywords::Keyword) -> Token {
    Token::Ident(str_to_ident(&kw.name().as_str()[..]))
}


fn unravel(binds: Bindings) -> Vec<TokenTree> {
    let mut output = Vec::new();

    for b in binds {
        let name = b.0;
        let rhs = b.1;
        let mut new_let = build_let(name, rhs);
        output.append(&mut new_let);
    }

    output
}

fn as_tts(ts: Vec<Token>) -> Vec<TokenTree> {
    ts.into_iter().map(as_tt).collect()
}

fn as_tt(t: Token) -> TokenTree {
    TokenTree::Token(DUMMY_SP, t)
}

fn build_empty_args() -> TokenTree {
    TokenTree::Delimited(DUMMY_SP,
                         tokenstream::Delimited {
                             delim: token::DelimToken::Paren,
                             open_span: DUMMY_SP,
                             tts: vec![],
                             close_span: DUMMY_SP,
                         })
}

fn build_struct_field_assign(field: Ident, mut rhs: Vec<TokenTree>) -> Vec<TokenTree> {
    let id = as_tt(Token::Ident(field));
    let mut output = Vec::new();
    output.push(id);
    output.push(as_tt(Token::Colon));
    output.append(&mut rhs);
    output.push(as_tt(Token::Comma));
    output
}

fn build_let(id: Ident, mut tts: Vec<TokenTree>) -> Vec<TokenTree> {
    let mut output = as_tts(vec![kw_to_tok_ident(keywords::Let), Token::Ident(id), Token::Eq]);
    output.append(&mut tts);
    output.push(as_tt(Token::Semi));
    output
}

fn build_method_call(id: Ident, mthd: Ident, args: Vec<TokenTree>) -> Vec<TokenTree> {
    let mut output = as_tts(vec![Token::Ident(id), Token::Dot, Token::Ident(mthd)]);

    let args = tokenstream::Delimited {
        delim: token::DelimToken::Paren,
        open_span: DUMMY_SP,
        tts: args,
        close_span: DUMMY_SP,
    };

    output.push(TokenTree::Delimited(DUMMY_SP, args));

    output
}

fn push_last(tts: &mut Vec<Vec<TokenTree>>, tt: TokenTree) {
    if tts.is_empty() {
        tts.push(vec![tt]);
    } else {
        tts.last_mut().unwrap().push(tt);
    }
}

fn append_last(tts: &mut Vec<Vec<TokenTree>>, mut ts: Vec<TokenTree>) {
    if tts.is_empty() {
        tts.push(ts);
    } else {
        tts.last_mut().unwrap().append(&mut ts);
    }
}

fn is_unquote(id: Ident) -> bool {
    let qq = str_to_ident("unquote");
    id.name == qq.name  // We disregard context; unquote is _reserved_
}

fn is_quote(id: Ident) -> bool {
    let qq = str_to_ident("qquote");
    id.name == qq.name  // We disregard context; qquote is _reserved_
}

