use crate::transform::{MirPass, MirSource};
use rustc_middle::mir::*;
use rustc_middle::ty::TyCtxt;

pub struct MatchBranchSimplification;

/// If a source block is found that switches between two blocks that are exactly
/// the same modulo const bool assignments (e.g., one assigns true another false
/// to the same place), merge a target block statements into the source block,
/// using Eq / Ne comparison with switch value where const bools value differ.
///
/// For example:
///
/// ```rust
/// bb0: {
///     switchInt(move _3) -> [42_isize: bb1, otherwise: bb2];
/// }
///
/// bb1: {
///     _2 = const true;
///     goto -> bb3;
/// }
///
/// bb2: {
///     _2 = const false;
///     goto -> bb3;
/// }
/// ```
///
/// into:
///
/// ```rust
/// bb0: {
///    _2 = Eq(move _3, const 42_isize);
///    goto -> bb3;
/// }
/// ```

impl<'tcx> MirPass<'tcx> for MatchBranchSimplification {
    fn run_pass(&self, tcx: TyCtxt<'tcx>, src: MirSource<'tcx>, body: &mut Body<'tcx>) {
        let param_env = tcx.param_env(src.def_id());
        let bbs = body.basic_blocks_mut();
        'outer: for bb_idx in bbs.indices() {
            let (discr, val, switch_ty, first, second) = match bbs[bb_idx].terminator().kind {
                TerminatorKind::SwitchInt {
                    discr: Operand::Copy(ref place) | Operand::Move(ref place),
                    switch_ty,
                    ref targets,
                    ref values,
                    ..
                } if targets.len() == 2 && values.len() == 1 && targets[0] != targets[1] => {
                    (place, values[0], switch_ty, targets[0], targets[1])
                }
                // Only optimize switch int statements
                _ => continue,
            };

            // Check that destinations are identical, and if not, then don't optimize this block
            if &bbs[first].terminator().kind != &bbs[second].terminator().kind {
                continue;
            }

            // Check that blocks are assignments of consts to the same place or same statement,
            // and match up 1-1, if not don't optimize this block.
            let first_stmts = &bbs[first].statements;
            let scnd_stmts = &bbs[second].statements;
            if first_stmts.len() != scnd_stmts.len() {
                continue;
            }
            for (f, s) in first_stmts.iter().zip(scnd_stmts.iter()) {
                match (&f.kind, &s.kind) {
                    // If two statements are exactly the same, we can optimize.
                    (f_s, s_s) if f_s == s_s => {}

                    // If two statements are const bool assignments to the same place, we can optimize.
                    (
                        StatementKind::Assign(box (lhs_f, Rvalue::Use(Operand::Constant(f_c)))),
                        StatementKind::Assign(box (lhs_s, Rvalue::Use(Operand::Constant(s_c)))),
                    ) if lhs_f == lhs_s
                        && f_c.literal.ty.is_bool()
                        && s_c.literal.ty.is_bool()
                        && f_c.literal.try_eval_bool(tcx, param_env).is_some()
                        && s_c.literal.try_eval_bool(tcx, param_env).is_some() => {}

                    // Otherwise we cannot optimize. Try another block.
                    _ => continue 'outer,
                }
            }
            // Take ownership of items now that we know we can optimize.
            let discr = discr.clone();

            // We already checked that first and second are different blocks,
            // and bb_idx has a different terminator from both of them.
            let (from, first, second) = bbs.pick3_mut(bb_idx, first, second);

            let new_stmts = first.statements.iter().zip(second.statements.iter()).map(|(f, s)| {
                match (&f.kind, &s.kind) {
                    (f_s, s_s) if f_s == s_s => (*f).clone(),

                    (
                        StatementKind::Assign(box (lhs, Rvalue::Use(Operand::Constant(f_c)))),
                        StatementKind::Assign(box (_, Rvalue::Use(Operand::Constant(s_c)))),
                    ) => {
                        // From earlier loop we know that we are dealing with bool constants only:
                        let f_b = f_c.literal.try_eval_bool(tcx, param_env).unwrap();
                        let s_b = s_c.literal.try_eval_bool(tcx, param_env).unwrap();
                        if f_b == s_b {
                            // Same value in both blocks. Use statement as is.
                            (*f).clone()
                        } else {
                            // Different value between blocks. Make value conditional on switch condition.
                            let size = tcx.layout_of(param_env.and(switch_ty)).unwrap().size;
                            let const_cmp = Operand::const_from_scalar(
                                tcx,
                                switch_ty,
                                crate::interpret::Scalar::from_uint(val, size),
                                rustc_span::DUMMY_SP,
                            );
                            let op = if f_b { BinOp::Eq } else { BinOp::Ne };
                            let rhs = Rvalue::BinaryOp(op, Operand::Copy(discr.clone()), const_cmp);
                            Statement {
                                source_info: f.source_info,
                                kind: StatementKind::Assign(box (*lhs, rhs)),
                            }
                        }
                    }

                    _ => unreachable!(),
                }
            });
            from.statements.extend(new_stmts);
            from.terminator_mut().kind = first.terminator().kind.clone();
        }
    }
}
