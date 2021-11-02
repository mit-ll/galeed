// See issue #12534.

#![feature(bindings_after_at)]
#![feature(move_ref_pattern)]

fn main() {}

struct A(Box<u8>);

fn f(a @ A(u): A) -> Box<u8> {
    //~^ ERROR use of moved value
    drop(a);
    u
}
