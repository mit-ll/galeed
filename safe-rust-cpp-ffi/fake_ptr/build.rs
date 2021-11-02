extern crate cc;

const OPTLEVEL: u32 = 3;

fn build_cpp() {
    cc::Build::new()
    .file("temp_cpp/uses_fakeptr_original.ll")
    .compiler("clang++")
    .opt_level(OPTLEVEL)
    .cpp(true)
    .warnings_into_errors(true)
    // .flag("-flto=thin")
    .compile("libuses_fakeptr_original.so");
}

fn build_cpp_fakeptr() {
    cc::Build::new()
    .file("temp_cpp/uses_fakeptr_safe.ll")
    .compiler("clang++")
    .opt_level(OPTLEVEL)
    .cpp(true)
    .warnings_into_errors(true)
    // .flag("-flto=thin")
    .compile("libuses_fakeptr_safe.so");
}

fn build_scheduler() {
    cc::Build::new()
    .file("src/scheduling.cpp")
    .compiler("clang++")
    .opt_level(OPTLEVEL)
    .cpp(true)
    .warnings_into_errors(true)
    // .flag("-flto=thin")
    .compile("schedule.so");
}

fn main() {
    build_cpp();
    build_cpp_fakeptr();
    build_scheduler();
}
