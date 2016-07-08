#![feature(plugin)]
#![feature(rustc_private)]
#![feature(trace_macros)]
#![plugin(qquote)]

extern crate qquote;

trace_macros!(true);

fn test_lit_1() -> () {
  assert_eq!(5 + qquote!(5), 10)
}


fn test_lit_2() -> () {
  assert_eq!(5.1 + qquote!(5.1), 10.2)
}

fn test_lit_3() -> () {
  assert_eq!(qquote!("foo"), "foo")
}

fn test_lit_4() -> () {
  assert_eq!(qquote!("foob"), "foo")
}

fn test_ident_1() -> () {
  let foo = 5;
  assert_eq!(qquote!(foo), 5)
}

fn test_ident_2() -> () {
  let foo = 5;
  assert_eq(qquote!(unquote(foo)), 5)
}

fn main() {
  test_lit_1();
  test_lit_2();
  test_lit_3();
  test_lit_4();
  test_ident_1();
  test_ident_2();
}
