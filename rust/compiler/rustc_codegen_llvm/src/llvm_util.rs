use crate::back::write::create_informational_target_machine;
use crate::llvm;
use libc::c_int;
use rustc_data_structures::fx::FxHashSet;
use rustc_feature::UnstableFeatures;
use rustc_middle::bug;
use rustc_session::config::PrintRequest;
use rustc_session::Session;
use rustc_span::symbol::sym;
use rustc_span::symbol::Symbol;
use rustc_target::spec::{MergeFunctions, PanicStrategy};
use std::ffi::CString;

use std::slice;
use std::str;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;

static POISONED: AtomicBool = AtomicBool::new(false);
static INIT: Once = Once::new();

pub(crate) fn init(sess: &Session) {
    unsafe {
        // Before we touch LLVM, make sure that multithreading is enabled.
        INIT.call_once(|| {
            if llvm::LLVMStartMultithreaded() != 1 {
                // use an extra bool to make sure that all future usage of LLVM
                // cannot proceed despite the Once not running more than once.
                POISONED.store(true, Ordering::SeqCst);
            }

            configure_llvm(sess);
        });

        if POISONED.load(Ordering::SeqCst) {
            bug!("couldn't enable multi-threaded LLVM");
        }
    }
}

fn require_inited() {
    INIT.call_once(|| bug!("llvm is not initialized"));
    if POISONED.load(Ordering::SeqCst) {
        bug!("couldn't enable multi-threaded LLVM");
    }
}

unsafe fn configure_llvm(sess: &Session) {
    let n_args = sess.opts.cg.llvm_args.len() + sess.target.target.options.llvm_args.len();
    let mut llvm_c_strs = Vec::with_capacity(n_args + 1);
    let mut llvm_args = Vec::with_capacity(n_args + 1);

    llvm::LLVMRustInstallFatalErrorHandler();

    fn llvm_arg_to_arg_name(full_arg: &str) -> &str {
        full_arg.trim().split(|c: char| c == '=' || c.is_whitespace()).next().unwrap_or("")
    }

    let cg_opts = sess.opts.cg.llvm_args.iter();
    let tg_opts = sess.target.target.options.llvm_args.iter();
    let sess_args = cg_opts.chain(tg_opts);

    let user_specified_args: FxHashSet<_> =
        sess_args.clone().map(|s| llvm_arg_to_arg_name(s)).filter(|s| !s.is_empty()).collect();

    {
        // This adds the given argument to LLVM. Unless `force` is true
        // user specified arguments are *not* overridden.
        let mut add = |arg: &str, force: bool| {
            if force || !user_specified_args.contains(llvm_arg_to_arg_name(arg)) {
                let s = CString::new(arg).unwrap();
                llvm_args.push(s.as_ptr());
                llvm_c_strs.push(s);
            }
        };
        // Set the llvm "program name" to make usage and invalid argument messages more clear.
        add("rustc -Cllvm-args=\"...\" with", true);
        if sess.time_llvm_passes() {
            add("-time-passes", false);
        }
        if sess.print_llvm_passes() {
            add("-debug-pass=Structure", false);
        }
        if !sess.opts.debugging_opts.no_generate_arange_section {
            add("-generate-arange-section", false);
        }
        match sess
            .opts
            .debugging_opts
            .merge_functions
            .unwrap_or(sess.target.target.options.merge_functions)
        {
            MergeFunctions::Disabled | MergeFunctions::Trampolines => {}
            MergeFunctions::Aliases => {
                add("-mergefunc-use-aliases", false);
            }
        }

        if sess.target.target.target_os == "emscripten"
            && sess.panic_strategy() == PanicStrategy::Unwind
        {
            add("-enable-emscripten-cxx-exceptions", false);
        }

        // HACK(eddyb) LLVM inserts `llvm.assume` calls to preserve align attributes
        // during inlining. Unfortunately these may block other optimizations.
        add("-preserve-alignment-assumptions-during-inlining=false", false);

        for arg in sess_args {
            add(&(*arg), true);
        }
    }

    if sess.opts.debugging_opts.llvm_time_trace && get_major_version() >= 9 {
        // time-trace is not thread safe and running it in parallel will cause seg faults.
        if !sess.opts.debugging_opts.no_parallel_llvm {
            bug!("`-Z llvm-time-trace` requires `-Z no-parallel-llvm")
        }

        llvm::LLVMTimeTraceProfilerInitialize();
    }

    llvm::LLVMInitializePasses();

    ::rustc_llvm::initialize_available_targets();

    llvm::LLVMRustSetLLVMOptions(llvm_args.len() as c_int, llvm_args.as_ptr());
}

pub fn time_trace_profiler_finish(file_name: &str) {
    unsafe {
        if get_major_version() >= 9 {
            let file_name = CString::new(file_name).unwrap();
            llvm::LLVMTimeTraceProfilerFinish(file_name.as_ptr());
        }
    }
}

// WARNING: the features after applying `to_llvm_feature` must be known
// to LLVM or the feature detection code will walk past the end of the feature
// array, leading to crashes.

const ARM_ALLOWED_FEATURES: &[(&str, Option<Symbol>)] = &[
    ("aclass", Some(sym::arm_target_feature)),
    ("mclass", Some(sym::arm_target_feature)),
    ("rclass", Some(sym::arm_target_feature)),
    ("dsp", Some(sym::arm_target_feature)),
    ("neon", Some(sym::arm_target_feature)),
    ("crc", Some(sym::arm_target_feature)),
    ("crypto", Some(sym::arm_target_feature)),
    ("v5te", Some(sym::arm_target_feature)),
    ("v6", Some(sym::arm_target_feature)),
    ("v6k", Some(sym::arm_target_feature)),
    ("v6t2", Some(sym::arm_target_feature)),
    ("v7", Some(sym::arm_target_feature)),
    ("v8", Some(sym::arm_target_feature)),
    ("vfp2", Some(sym::arm_target_feature)),
    ("vfp3", Some(sym::arm_target_feature)),
    ("vfp4", Some(sym::arm_target_feature)),
    // This is needed for inline assembly, but shouldn't be stabilized as-is
    // since it should be enabled per-function using #[instruction_set], not
    // #[target_feature].
    ("thumb-mode", Some(sym::arm_target_feature)),
];

const AARCH64_ALLOWED_FEATURES: &[(&str, Option<Symbol>)] = &[
    ("fp", Some(sym::aarch64_target_feature)),
    ("neon", Some(sym::aarch64_target_feature)),
    ("sve", Some(sym::aarch64_target_feature)),
    ("crc", Some(sym::aarch64_target_feature)),
    ("crypto", Some(sym::aarch64_target_feature)),
    ("ras", Some(sym::aarch64_target_feature)),
    ("lse", Some(sym::aarch64_target_feature)),
    ("rdm", Some(sym::aarch64_target_feature)),
    ("fp16", Some(sym::aarch64_target_feature)),
    ("rcpc", Some(sym::aarch64_target_feature)),
    ("dotprod", Some(sym::aarch64_target_feature)),
    ("tme", Some(sym::aarch64_target_feature)),
    ("v8.1a", Some(sym::aarch64_target_feature)),
    ("v8.2a", Some(sym::aarch64_target_feature)),
    ("v8.3a", Some(sym::aarch64_target_feature)),
];

const X86_ALLOWED_FEATURES: &[(&str, Option<Symbol>)] = &[
    ("adx", Some(sym::adx_target_feature)),
    ("aes", None),
    ("avx", None),
    ("avx2", None),
    ("avx512bw", Some(sym::avx512_target_feature)),
    ("avx512cd", Some(sym::avx512_target_feature)),
    ("avx512dq", Some(sym::avx512_target_feature)),
    ("avx512er", Some(sym::avx512_target_feature)),
    ("avx512f", Some(sym::avx512_target_feature)),
    ("avx512ifma", Some(sym::avx512_target_feature)),
    ("avx512pf", Some(sym::avx512_target_feature)),
    ("avx512vbmi", Some(sym::avx512_target_feature)),
    ("avx512vl", Some(sym::avx512_target_feature)),
    ("avx512vpopcntdq", Some(sym::avx512_target_feature)),
    ("bmi1", None),
    ("bmi2", None),
    ("cmpxchg16b", Some(sym::cmpxchg16b_target_feature)),
    ("f16c", Some(sym::f16c_target_feature)),
    ("fma", None),
    ("fxsr", None),
    ("lzcnt", None),
    ("mmx", Some(sym::mmx_target_feature)),
    ("movbe", Some(sym::movbe_target_feature)),
    ("pclmulqdq", None),
    ("popcnt", None),
    ("rdrand", None),
    ("rdseed", None),
    ("rtm", Some(sym::rtm_target_feature)),
    ("sha", None),
    ("sse", None),
    ("sse2", None),
    ("sse3", None),
    ("sse4.1", None),
    ("sse4.2", None),
    ("sse4a", Some(sym::sse4a_target_feature)),
    ("ssse3", None),
    ("tbm", Some(sym::tbm_target_feature)),
    ("xsave", None),
    ("xsavec", None),
    ("xsaveopt", None),
    ("xsaves", None),
];

const HEXAGON_ALLOWED_FEATURES: &[(&str, Option<Symbol>)] = &[
    ("hvx", Some(sym::hexagon_target_feature)),
    ("hvx-length128b", Some(sym::hexagon_target_feature)),
];

const POWERPC_ALLOWED_FEATURES: &[(&str, Option<Symbol>)] = &[
    ("altivec", Some(sym::powerpc_target_feature)),
    ("power8-altivec", Some(sym::powerpc_target_feature)),
    ("power9-altivec", Some(sym::powerpc_target_feature)),
    ("power8-vector", Some(sym::powerpc_target_feature)),
    ("power9-vector", Some(sym::powerpc_target_feature)),
    ("vsx", Some(sym::powerpc_target_feature)),
];

const MIPS_ALLOWED_FEATURES: &[(&str, Option<Symbol>)] =
    &[("fp64", Some(sym::mips_target_feature)), ("msa", Some(sym::mips_target_feature))];

const RISCV_ALLOWED_FEATURES: &[(&str, Option<Symbol>)] = &[
    ("m", Some(sym::riscv_target_feature)),
    ("a", Some(sym::riscv_target_feature)),
    ("c", Some(sym::riscv_target_feature)),
    ("f", Some(sym::riscv_target_feature)),
    ("d", Some(sym::riscv_target_feature)),
    ("e", Some(sym::riscv_target_feature)),
];

const WASM_ALLOWED_FEATURES: &[(&str, Option<Symbol>)] = &[
    ("simd128", Some(sym::wasm_target_feature)),
    ("atomics", Some(sym::wasm_target_feature)),
    ("nontrapping-fptoint", Some(sym::wasm_target_feature)),
];

/// When rustdoc is running, provide a list of all known features so that all their respective
/// primitives may be documented.
///
/// IMPORTANT: If you're adding another feature list above, make sure to add it to this iterator!
pub fn all_known_features() -> impl Iterator<Item = (&'static str, Option<Symbol>)> {
    std::iter::empty()
        .chain(ARM_ALLOWED_FEATURES.iter())
        .chain(AARCH64_ALLOWED_FEATURES.iter())
        .chain(X86_ALLOWED_FEATURES.iter())
        .chain(HEXAGON_ALLOWED_FEATURES.iter())
        .chain(POWERPC_ALLOWED_FEATURES.iter())
        .chain(MIPS_ALLOWED_FEATURES.iter())
        .chain(RISCV_ALLOWED_FEATURES.iter())
        .chain(WASM_ALLOWED_FEATURES.iter())
        .cloned()
}

pub fn to_llvm_feature<'a>(sess: &Session, s: &'a str) -> &'a str {
    let arch = if sess.target.target.arch == "x86_64" { "x86" } else { &*sess.target.target.arch };
    match (arch, s) {
        ("x86", "pclmulqdq") => "pclmul",
        ("x86", "rdrand") => "rdrnd",
        ("x86", "bmi1") => "bmi",
        ("x86", "cmpxchg16b") => "cx16",
        ("aarch64", "fp") => "fp-armv8",
        ("aarch64", "fp16") => "fullfp16",
        (_, s) => s,
    }
}

pub fn target_features(sess: &Session) -> Vec<Symbol> {
    let target_machine = create_informational_target_machine(sess);
    supported_target_features(sess)
        .iter()
        .filter_map(|&(feature, gate)| {
            if UnstableFeatures::from_environment().is_nightly_build() || gate.is_none() {
                Some(feature)
            } else {
                None
            }
        })
        .filter(|feature| {
            let llvm_feature = to_llvm_feature(sess, feature);
            let cstr = CString::new(llvm_feature).unwrap();
            unsafe { llvm::LLVMRustHasFeature(target_machine, cstr.as_ptr()) }
        })
        .map(|feature| Symbol::intern(feature))
        .collect()
}

pub fn supported_target_features(sess: &Session) -> &'static [(&'static str, Option<Symbol>)] {
    match &*sess.target.target.arch {
        "arm" => ARM_ALLOWED_FEATURES,
        "aarch64" => AARCH64_ALLOWED_FEATURES,
        "x86" | "x86_64" => X86_ALLOWED_FEATURES,
        "hexagon" => HEXAGON_ALLOWED_FEATURES,
        "mips" | "mips64" => MIPS_ALLOWED_FEATURES,
        "powerpc" | "powerpc64" => POWERPC_ALLOWED_FEATURES,
        "riscv32" | "riscv64" => RISCV_ALLOWED_FEATURES,
        "wasm32" => WASM_ALLOWED_FEATURES,
        _ => &[],
    }
}

pub fn print_version() {
    // Can be called without initializing LLVM
    unsafe {
        println!("LLVM version: {}.{}", llvm::LLVMRustVersionMajor(), llvm::LLVMRustVersionMinor());
    }
}

pub fn get_major_version() -> u32 {
    unsafe { llvm::LLVMRustVersionMajor() }
}

pub fn print_passes() {
    // Can be called without initializing LLVM
    unsafe {
        llvm::LLVMRustPrintPasses();
    }
}

pub(crate) fn print(req: PrintRequest, sess: &Session) {
    require_inited();
    let tm = create_informational_target_machine(sess);
    unsafe {
        match req {
            PrintRequest::TargetCPUs => llvm::LLVMRustPrintTargetCPUs(tm),
            PrintRequest::TargetFeatures => llvm::LLVMRustPrintTargetFeatures(tm),
            _ => bug!("rustc_codegen_llvm can't handle print request: {:?}", req),
        }
    }
}

pub fn target_cpu(sess: &Session) -> &str {
    let name = match sess.opts.cg.target_cpu {
        Some(ref s) => &**s,
        None => &*sess.target.target.options.cpu,
    };
    if name != "native" {
        return name;
    }

    unsafe {
        let mut len = 0;
        let ptr = llvm::LLVMRustGetHostCPUName(&mut len);
        str::from_utf8(slice::from_raw_parts(ptr as *const u8, len)).unwrap()
    }
}
