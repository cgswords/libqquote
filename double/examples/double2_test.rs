#![feature(plugin)]
#![plugin(double)]

#![allow(unused_mut)]
#![allow(unused_parens)]

// ____________________________________________________________________________________________
// Main 

fn main() {
  let foo = double2!(5);
  println!("{:?}", foo);
}

