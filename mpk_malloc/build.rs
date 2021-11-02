extern crate cc;

fn build_cpp() {
    cc::Build::new()
    .file("src/toy.cpp")
    .compiler("clang++")
    .opt_level(3)
    .cpp(true)
    .warnings_into_errors(true)
    .compile("toy.so");
}

fn main() {
    build_cpp();
    println!("cargo:rustc-link-lib=mpt");
    println!("cargo:rustc-link-lib=mpk_heaplib")
}
