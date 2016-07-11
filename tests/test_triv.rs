#![feature(plugin)]
#![plugin(qquote)]

#![allow(unused_mut)]
#![allow(unused_parens)]

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
  let res = qquote!(unquote(foo));
  assert_eq!(res, 5)
}

#[test]
fn test_add() -> () {
  let foo = 5;
  let res = qquote!(unquote(foo));
  assert_eq!(res, 5)
}

