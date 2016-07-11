#![feature(plugin)]
#![plugin(double)]

#![allow(unused_mut)]
#![allow(unused_parens)]

#[macro_use] mod double;

// ____________________________________________________________________________________________
// Main 

fn main() {
  double!(5)
}

