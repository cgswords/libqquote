#![feature(plugin)]
#![plugin(double)]

#![allow(unused_mut)]
#![allow(unused_parens)]

// ____________________________________________________________________________________________
// Main 

fn fact1(n : i64) -> i64 {
  if ( n == 0) { 1 } else { ( n * fact1 ( n - 1 ) ) }
}

fn fact(n : i64) -> i64 { 
  cond!(
    ((n == 0), 1)
    (else, (n * fact(n-1)))
  )
}

fn fib(n : i64) -> i64 { 
  cond!(
    ((n == 0), 1)
    ((n == 1), 1)
    (else, (fib(n-1) + fib(n-2)))
  )
}
fn main() {
  println!("{:?}", fact1(5));
  println!("{:?}", fact(5));
  println!("{:?}", fib(5));
}

