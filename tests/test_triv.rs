#![feature(plugin)]
#![plugin(qquote)]

#[test]
fn test_trivial_1() -> () {
  assert_eq!(5 + qquote!(5), 10)
}


#[test]
fn test_trivial_2() -> () {
  assert_eq!(5.1 + qquote!(5.1), 10.2)
}

#[test]
fn test_trivial_3() -> () {
  assert_eq!(qquote!("foo"), "foo")
}

#[test]
#[should_panic]
fn test_trivial_4() -> () {
  assert_eq!(qquote!("foob"), "foo")
}
