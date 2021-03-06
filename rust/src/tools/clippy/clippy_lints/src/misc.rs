use if_chain::if_chain;
use rustc_ast::ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::intravisit::FnKind;
use rustc_hir::{
    self as hir, def, BinOpKind, BindingAnnotation, Body, Expr, ExprKind, FnDecl, HirId, Mutability, PatKind, Stmt,
    StmtKind, TyKind, UnOp,
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::hygiene::DesugaringKind;
use rustc_span::source_map::{ExpnKind, Span};

use crate::consts::{constant, Constant};
use crate::utils::sugg::Sugg;
use crate::utils::{
    get_item_name, get_parent_expr, higher, implements_trait, in_constant, is_integer_const, iter_input_pats,
    last_path_segment, match_qpath, match_trait_method, paths, snippet, snippet_opt, span_lint, span_lint_and_sugg,
    span_lint_and_then, span_lint_hir_and_then, walk_ptrs_ty, SpanlessEq,
};

declare_clippy_lint! {
    /// **What it does:** Checks for function arguments and let bindings denoted as
    /// `ref`.
    ///
    /// **Why is this bad?** The `ref` declaration makes the function take an owned
    /// value, but turns the argument into a reference (which means that the value
    /// is destroyed when exiting the function). This adds not much value: either
    /// take a reference type, or take an owned value and create references in the
    /// body.
    ///
    /// For let bindings, `let x = &foo;` is preferred over `let ref x = foo`. The
    /// type of `x` is more obvious with the former.
    ///
    /// **Known problems:** If the argument is dereferenced within the function,
    /// removing the `ref` will lead to errors. This can be fixed by removing the
    /// dereferences, e.g., changing `*x` to `x` within the function.
    ///
    /// **Example:**
    /// ```rust,ignore
    /// // Bad
    /// fn foo(ref x: u8) -> bool {
    ///     true
    /// }
    ///
    /// // Good
    /// fn foo(x: &u8) -> bool {
    ///     true
    /// }
    /// ```
    pub TOPLEVEL_REF_ARG,
    style,
    "an entire binding declared as `ref`, in a function argument or a `let` statement"
}

declare_clippy_lint! {
    /// **What it does:** Checks for comparisons to NaN.
    ///
    /// **Why is this bad?** NaN does not compare meaningfully to anything ??? not
    /// even itself ??? so those comparisons are simply wrong.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    /// ```rust
    /// # let x = 1.0;
    ///
    /// // Bad
    /// if x == f32::NAN { }
    ///
    /// // Good
    /// if x.is_nan() { }
    /// ```
    pub CMP_NAN,
    correctness,
    "comparisons to `NAN`, which will always return false, probably not intended"
}

declare_clippy_lint! {
    /// **What it does:** Checks for (in-)equality comparisons on floating-point
    /// values (apart from zero), except in functions called `*eq*` (which probably
    /// implement equality for a type involving floats).
    ///
    /// **Why is this bad?** Floating point calculations are usually imprecise, so
    /// asking if two values are *exactly* equal is asking for trouble. For a good
    /// guide on what to do, see [the floating point
    /// guide](http://www.floating-point-gui.de/errors/comparison).
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    /// ```rust
    /// let x = 1.2331f64;
    /// let y = 1.2332f64;
    ///
    /// // Bad
    /// if y == 1.23f64 { }
    /// if y != x {} // where both are floats
    ///
    /// // Good
    /// let error = f64::EPSILON; // Use an epsilon for comparison
    /// // Or, if Rust <= 1.42, use `std::f64::EPSILON` constant instead.
    /// // let error = std::f64::EPSILON;
    /// if (y - 1.23f64).abs() < error { }
    /// if (y - x).abs() > error { }
    /// ```
    pub FLOAT_CMP,
    correctness,
    "using `==` or `!=` on float values instead of comparing difference with an epsilon"
}

declare_clippy_lint! {
    /// **What it does:** Checks for conversions to owned values just for the sake
    /// of a comparison.
    ///
    /// **Why is this bad?** The comparison can operate on a reference, so creating
    /// an owned value effectively throws it away directly afterwards, which is
    /// needlessly consuming code and heap space.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    /// ```rust
    /// # let x = "foo";
    /// # let y = String::from("foo");
    /// if x.to_owned() == y {}
    /// ```
    /// Could be written as
    /// ```rust
    /// # let x = "foo";
    /// # let y = String::from("foo");
    /// if x == y {}
    /// ```
    pub CMP_OWNED,
    perf,
    "creating owned instances for comparing with others, e.g., `x == \"foo\".to_string()`"
}

declare_clippy_lint! {
    /// **What it does:** Checks for getting the remainder of a division by one.
    ///
    /// **Why is this bad?** The result can only ever be zero. No one will write
    /// such code deliberately, unless trying to win an Underhanded Rust
    /// Contest. Even for that contest, it's probably a bad idea. Use something more
    /// underhanded.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    /// ```rust
    /// # let x = 1;
    /// let a = x % 1;
    /// ```
    pub MODULO_ONE,
    correctness,
    "taking a number modulo 1, which always returns 0"
}

declare_clippy_lint! {
    /// **What it does:** Checks for the use of bindings with a single leading
    /// underscore.
    ///
    /// **Why is this bad?** A single leading underscore is usually used to indicate
    /// that a binding will not be used. Using such a binding breaks this
    /// expectation.
    ///
    /// **Known problems:** The lint does not work properly with desugaring and
    /// macro, it has been allowed in the mean time.
    ///
    /// **Example:**
    /// ```rust
    /// let _x = 0;
    /// let y = _x + 1; // Here we are using `_x`, even though it has a leading
    ///                 // underscore. We should rename `_x` to `x`
    /// ```
    pub USED_UNDERSCORE_BINDING,
    pedantic,
    "using a binding which is prefixed with an underscore"
}

declare_clippy_lint! {
    /// **What it does:** Checks for the use of short circuit boolean conditions as
    /// a
    /// statement.
    ///
    /// **Why is this bad?** Using a short circuit boolean condition as a statement
    /// may hide the fact that the second part is executed or not depending on the
    /// outcome of the first part.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    /// ```rust,ignore
    /// f() && g(); // We should write `if f() { g(); }`.
    /// ```
    pub SHORT_CIRCUIT_STATEMENT,
    complexity,
    "using a short circuit boolean condition as a statement"
}

declare_clippy_lint! {
    /// **What it does:** Catch casts from `0` to some pointer type
    ///
    /// **Why is this bad?** This generally means `null` and is better expressed as
    /// {`std`, `core`}`::ptr::`{`null`, `null_mut`}.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// // Bad
    /// let a = 0 as *const u32;
    ///
    /// // Good
    /// let a = std::ptr::null::<u32>();
    /// ```
    pub ZERO_PTR,
    style,
    "using `0 as *{const, mut} T`"
}

declare_clippy_lint! {
    /// **What it does:** Checks for (in-)equality comparisons on floating-point
    /// value and constant, except in functions called `*eq*` (which probably
    /// implement equality for a type involving floats).
    ///
    /// **Why is this bad?** Floating point calculations are usually imprecise, so
    /// asking if two values are *exactly* equal is asking for trouble. For a good
    /// guide on what to do, see [the floating point
    /// guide](http://www.floating-point-gui.de/errors/comparison).
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    /// ```rust
    /// let x: f64 = 1.0;
    /// const ONE: f64 = 1.00;
    ///
    /// // Bad
    /// if x == ONE { } // where both are floats
    ///
    /// // Good
    /// let error = f64::EPSILON; // Use an epsilon for comparison
    /// // Or, if Rust <= 1.42, use `std::f64::EPSILON` constant instead.
    /// // let error = std::f64::EPSILON;
    /// if (x - ONE).abs() < error { }
    /// ```
    pub FLOAT_CMP_CONST,
    restriction,
    "using `==` or `!=` on float constants instead of comparing difference with an epsilon"
}

declare_lint_pass!(MiscLints => [
    TOPLEVEL_REF_ARG,
    CMP_NAN,
    FLOAT_CMP,
    CMP_OWNED,
    MODULO_ONE,
    USED_UNDERSCORE_BINDING,
    SHORT_CIRCUIT_STATEMENT,
    ZERO_PTR,
    FLOAT_CMP_CONST
]);

impl<'tcx> LateLintPass<'tcx> for MiscLints {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        k: FnKind<'tcx>,
        decl: &'tcx FnDecl<'_>,
        body: &'tcx Body<'_>,
        _: Span,
        _: HirId,
    ) {
        if let FnKind::Closure(_) = k {
            // Does not apply to closures
            return;
        }
        for arg in iter_input_pats(decl, body) {
            if let PatKind::Binding(BindingAnnotation::Ref | BindingAnnotation::RefMut, ..) = arg.pat.kind {
                span_lint(
                    cx,
                    TOPLEVEL_REF_ARG,
                    arg.pat.span,
                    "`ref` directly on a function argument is ignored. \
                    Consider using a reference type instead.",
                );
            }
        }
    }

    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &'tcx Stmt<'_>) {
        if_chain! {
            if let StmtKind::Local(ref local) = stmt.kind;
            if let PatKind::Binding(an, .., name, None) = local.pat.kind;
            if let Some(ref init) = local.init;
            if !higher::is_from_for_desugar(local);
            then {
                if an == BindingAnnotation::Ref || an == BindingAnnotation::RefMut {
                    let sugg_init = if init.span.from_expansion() {
                        Sugg::hir_with_macro_callsite(cx, init, "..")
                    } else {
                        Sugg::hir(cx, init, "..")
                    };
                    let (mutopt, initref) = if an == BindingAnnotation::RefMut {
                        ("mut ", sugg_init.mut_addr())
                    } else {
                        ("", sugg_init.addr())
                    };
                    let tyopt = if let Some(ref ty) = local.ty {
                        format!(": &{mutopt}{ty}", mutopt=mutopt, ty=snippet(cx, ty.span, "_"))
                    } else {
                        String::new()
                    };
                    span_lint_hir_and_then(
                        cx,
                        TOPLEVEL_REF_ARG,
                        init.hir_id,
                        local.pat.span,
                        "`ref` on an entire `let` pattern is discouraged, take a reference with `&` instead",
                        |diag| {
                            diag.span_suggestion(
                                stmt.span,
                                "try",
                                format!(
                                    "let {name}{tyopt} = {initref};",
                                    name=snippet(cx, name.span, "_"),
                                    tyopt=tyopt,
                                    initref=initref,
                                ),
                                Applicability::MachineApplicable,
                            );
                        }
                    );
                }
            }
        };
        if_chain! {
            if let StmtKind::Semi(ref expr) = stmt.kind;
            if let ExprKind::Binary(ref binop, ref a, ref b) = expr.kind;
            if binop.node == BinOpKind::And || binop.node == BinOpKind::Or;
            if let Some(sugg) = Sugg::hir_opt(cx, a);
            then {
                span_lint_and_then(cx,
                    SHORT_CIRCUIT_STATEMENT,
                    stmt.span,
                    "boolean short circuit operator in statement may be clearer using an explicit test",
                    |diag| {
                        let sugg = if binop.node == BinOpKind::Or { !sugg } else { sugg };
                        diag.span_suggestion(
                            stmt.span,
                            "replace it with",
                            format!(
                                "if {} {{ {}; }}",
                                sugg,
                                &snippet(cx, b.span, ".."),
                            ),
                            Applicability::MachineApplicable, // snippet
                        );
                    });
            }
        };
    }

    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        match expr.kind {
            ExprKind::Cast(ref e, ref ty) => {
                check_cast(cx, expr.span, e, ty);
                return;
            },
            ExprKind::Binary(ref cmp, ref left, ref right) => {
                let op = cmp.node;
                if op.is_comparison() {
                    check_nan(cx, left, expr);
                    check_nan(cx, right, expr);
                    check_to_owned(cx, left, right, true);
                    check_to_owned(cx, right, left, false);
                }
                if (op == BinOpKind::Eq || op == BinOpKind::Ne) && (is_float(cx, left) || is_float(cx, right)) {
                    if is_allowed(cx, left) || is_allowed(cx, right) {
                        return;
                    }

                    // Allow comparing the results of signum()
                    if is_signum(cx, left) && is_signum(cx, right) {
                        return;
                    }

                    if let Some(name) = get_item_name(cx, expr) {
                        let name = name.as_str();
                        if name == "eq"
                            || name == "ne"
                            || name == "is_nan"
                            || name.starts_with("eq_")
                            || name.ends_with("_eq")
                        {
                            return;
                        }
                    }
                    let is_comparing_arrays = is_array(cx, left) || is_array(cx, right);
                    let (lint, msg) = get_lint_and_message(
                        is_named_constant(cx, left) || is_named_constant(cx, right),
                        is_comparing_arrays,
                    );
                    span_lint_and_then(cx, lint, expr.span, msg, |diag| {
                        let lhs = Sugg::hir(cx, left, "..");
                        let rhs = Sugg::hir(cx, right, "..");

                        if !is_comparing_arrays {
                            diag.span_suggestion(
                                expr.span,
                                "consider comparing them within some error",
                                format!(
                                    "({}).abs() {} error",
                                    lhs - rhs,
                                    if op == BinOpKind::Eq { '<' } else { '>' }
                                ),
                                Applicability::HasPlaceholders, // snippet
                            );
                        }
                        diag.note("`f32::EPSILON` and `f64::EPSILON` are available for the `error`");
                    });
                } else if op == BinOpKind::Rem && is_integer_const(cx, right, 1) {
                    span_lint(cx, MODULO_ONE, expr.span, "any number modulo 1 will be 0");
                }
            },
            _ => {},
        }
        if in_attributes_expansion(expr) || expr.span.is_desugaring(DesugaringKind::Await) {
            // Don't lint things expanded by #[derive(...)], etc or `await` desugaring
            return;
        }
        let binding = match expr.kind {
            ExprKind::Path(ref qpath) if !matches!(qpath, hir::QPath::LangItem(..)) => {
                let binding = last_path_segment(qpath).ident.as_str();
                if binding.starts_with('_') &&
                    !binding.starts_with("__") &&
                    binding != "_result" && // FIXME: #944
                    is_used(cx, expr) &&
                    // don't lint if the declaration is in a macro
                    non_macro_local(cx, cx.qpath_res(qpath, expr.hir_id))
                {
                    Some(binding)
                } else {
                    None
                }
            },
            ExprKind::Field(_, ident) => {
                let name = ident.as_str();
                if name.starts_with('_') && !name.starts_with("__") {
                    Some(name)
                } else {
                    None
                }
            },
            _ => None,
        };
        if let Some(binding) = binding {
            span_lint(
                cx,
                USED_UNDERSCORE_BINDING,
                expr.span,
                &format!(
                    "used binding `{}` which is prefixed with an underscore. A leading \
                     underscore signals that a binding will not be used.",
                    binding
                ),
            );
        }
    }
}

fn get_lint_and_message(
    is_comparing_constants: bool,
    is_comparing_arrays: bool,
) -> (&'static rustc_lint::Lint, &'static str) {
    if is_comparing_constants {
        (
            FLOAT_CMP_CONST,
            if is_comparing_arrays {
                "strict comparison of `f32` or `f64` constant arrays"
            } else {
                "strict comparison of `f32` or `f64` constant"
            },
        )
    } else {
        (
            FLOAT_CMP,
            if is_comparing_arrays {
                "strict comparison of `f32` or `f64` arrays"
            } else {
                "strict comparison of `f32` or `f64`"
            },
        )
    }
}

fn check_nan(cx: &LateContext<'_>, expr: &Expr<'_>, cmp_expr: &Expr<'_>) {
    if_chain! {
        if !in_constant(cx, cmp_expr.hir_id);
        if let Some((value, _)) = constant(cx, cx.typeck_results(), expr);
        then {
            let needs_lint = match value {
                Constant::F32(num) => num.is_nan(),
                Constant::F64(num) => num.is_nan(),
                _ => false,
            };

            if needs_lint {
                span_lint(
                    cx,
                    CMP_NAN,
                    cmp_expr.span,
                    "doomed comparison with `NAN`, use `{f32,f64}::is_nan()` instead",
                );
            }
        }
    }
}

fn is_named_constant<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) -> bool {
    if let Some((_, res)) = constant(cx, cx.typeck_results(), expr) {
        res
    } else {
        false
    }
}

fn is_allowed<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) -> bool {
    match constant(cx, cx.typeck_results(), expr) {
        Some((Constant::F32(f), _)) => f == 0.0 || f.is_infinite(),
        Some((Constant::F64(f), _)) => f == 0.0 || f.is_infinite(),
        Some((Constant::Vec(vec), _)) => vec.iter().all(|f| match f {
            Constant::F32(f) => *f == 0.0 || (*f).is_infinite(),
            Constant::F64(f) => *f == 0.0 || (*f).is_infinite(),
            _ => false,
        }),
        _ => false,
    }
}

// Return true if `expr` is the result of `signum()` invoked on a float value.
fn is_signum(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // The negation of a signum is still a signum
    if let ExprKind::Unary(UnOp::UnNeg, ref child_expr) = expr.kind {
        return is_signum(cx, &child_expr);
    }

    if_chain! {
        if let ExprKind::MethodCall(ref method_name, _, ref expressions, _) = expr.kind;
        if sym!(signum) == method_name.ident.name;
        // Check that the receiver of the signum() is a float (expressions[0] is the receiver of
        // the method call)
        then {
            return is_float(cx, &expressions[0]);
        }
    }
    false
}

fn is_float(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let value = &walk_ptrs_ty(cx.typeck_results().expr_ty(expr)).kind();

    if let ty::Array(arr_ty, _) = value {
        return matches!(arr_ty.kind(), ty::Float(_));
    };

    matches!(value, ty::Float(_))
}

fn is_array(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    matches!(&walk_ptrs_ty(cx.typeck_results().expr_ty(expr)).kind(), ty::Array(_, _))
}

fn check_to_owned(cx: &LateContext<'_>, expr: &Expr<'_>, other: &Expr<'_>, left: bool) {
    #[derive(Default)]
    struct EqImpl {
        ty_eq_other: bool,
        other_eq_ty: bool,
    }

    impl EqImpl {
        fn is_implemented(&self) -> bool {
            self.ty_eq_other || self.other_eq_ty
        }
    }

    fn symmetric_partial_eq<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>, other: Ty<'tcx>) -> Option<EqImpl> {
        cx.tcx.lang_items().eq_trait().map(|def_id| EqImpl {
            ty_eq_other: implements_trait(cx, ty, def_id, &[other.into()]),
            other_eq_ty: implements_trait(cx, other, def_id, &[ty.into()]),
        })
    }

    let (arg_ty, snip) = match expr.kind {
        ExprKind::MethodCall(.., ref args, _) if args.len() == 1 => {
            if match_trait_method(cx, expr, &paths::TO_STRING) || match_trait_method(cx, expr, &paths::TO_OWNED) {
                (cx.typeck_results().expr_ty(&args[0]), snippet(cx, args[0].span, ".."))
            } else {
                return;
            }
        },
        ExprKind::Call(ref path, ref v) if v.len() == 1 => {
            if let ExprKind::Path(ref path) = path.kind {
                if match_qpath(path, &["String", "from_str"]) || match_qpath(path, &["String", "from"]) {
                    (cx.typeck_results().expr_ty(&v[0]), snippet(cx, v[0].span, ".."))
                } else {
                    return;
                }
            } else {
                return;
            }
        },
        _ => return,
    };

    let other_ty = cx.typeck_results().expr_ty(other);

    let without_deref = symmetric_partial_eq(cx, arg_ty, other_ty).unwrap_or_default();
    let with_deref = arg_ty
        .builtin_deref(true)
        .and_then(|tam| symmetric_partial_eq(cx, tam.ty, other_ty))
        .unwrap_or_default();

    if !with_deref.is_implemented() && !without_deref.is_implemented() {
        return;
    }

    let other_gets_derefed = matches!(other.kind, ExprKind::Unary(UnOp::UnDeref, _));

    let lint_span = if other_gets_derefed {
        expr.span.to(other.span)
    } else {
        expr.span
    };

    span_lint_and_then(
        cx,
        CMP_OWNED,
        lint_span,
        "this creates an owned instance just for comparison",
        |diag| {
            // This also catches `PartialEq` implementations that call `to_owned`.
            if other_gets_derefed {
                diag.span_label(lint_span, "try implementing the comparison without allocating");
                return;
            }

            let expr_snip;
            let eq_impl;
            if with_deref.is_implemented() {
                expr_snip = format!("*{}", snip);
                eq_impl = with_deref;
            } else {
                expr_snip = snip.to_string();
                eq_impl = without_deref;
            };

            let span;
            let hint;
            if (eq_impl.ty_eq_other && left) || (eq_impl.other_eq_ty && !left) {
                span = expr.span;
                hint = expr_snip;
            } else {
                span = expr.span.to(other.span);
                if eq_impl.ty_eq_other {
                    hint = format!("{} == {}", expr_snip, snippet(cx, other.span, ".."));
                } else {
                    hint = format!("{} == {}", snippet(cx, other.span, ".."), expr_snip);
                }
            }

            diag.span_suggestion(
                span,
                "try",
                hint,
                Applicability::MachineApplicable, // snippet
            );
        },
    );
}

/// Heuristic to see if an expression is used. Should be compatible with
/// `unused_variables`'s idea
/// of what it means for an expression to be "used".
fn is_used(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    get_parent_expr(cx, expr).map_or(true, |parent| match parent.kind {
        ExprKind::Assign(_, ref rhs, _) | ExprKind::AssignOp(_, _, ref rhs) => SpanlessEq::new(cx).eq_expr(rhs, expr),
        _ => is_used(cx, parent),
    })
}

/// Tests whether an expression is in a macro expansion (e.g., something
/// generated by `#[derive(...)]` or the like).
fn in_attributes_expansion(expr: &Expr<'_>) -> bool {
    use rustc_span::hygiene::MacroKind;
    if expr.span.from_expansion() {
        let data = expr.span.ctxt().outer_expn_data();
        matches!(data.kind, ExpnKind::Macro(MacroKind::Attr, _))
    } else {
        false
    }
}

/// Tests whether `res` is a variable defined outside a macro.
fn non_macro_local(cx: &LateContext<'_>, res: def::Res) -> bool {
    if let def::Res::Local(id) = res {
        !cx.tcx.hir().span(id).from_expansion()
    } else {
        false
    }
}

fn check_cast(cx: &LateContext<'_>, span: Span, e: &Expr<'_>, ty: &hir::Ty<'_>) {
    if_chain! {
        if let TyKind::Ptr(ref mut_ty) = ty.kind;
        if let ExprKind::Lit(ref lit) = e.kind;
        if let LitKind::Int(0, _) = lit.node;
        if !in_constant(cx, e.hir_id);
        then {
            let (msg, sugg_fn) = match mut_ty.mutbl {
                Mutability::Mut => ("`0 as *mut _` detected", "std::ptr::null_mut"),
                Mutability::Not => ("`0 as *const _` detected", "std::ptr::null"),
            };

            let (sugg, appl) = if let TyKind::Infer = mut_ty.ty.kind {
                (format!("{}()", sugg_fn), Applicability::MachineApplicable)
            } else if let Some(mut_ty_snip) = snippet_opt(cx, mut_ty.ty.span) {
                (format!("{}::<{}>()", sugg_fn, mut_ty_snip), Applicability::MachineApplicable)
            } else {
                // `MaybeIncorrect` as type inference may not work with the suggested code
                (format!("{}()", sugg_fn), Applicability::MaybeIncorrect)
            };
            span_lint_and_sugg(cx, ZERO_PTR, span, msg, "try", sugg, appl);
        }
    }
}
