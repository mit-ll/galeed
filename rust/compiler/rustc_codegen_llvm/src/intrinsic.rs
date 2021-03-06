use crate::abi::{Abi, FnAbi, LlvmType, PassMode};
use crate::builder::Builder;
use crate::context::CodegenCx;
use crate::llvm;
use crate::type_::Type;
use crate::type_of::LayoutLlvmExt;
use crate::va_arg::emit_va_arg;
use crate::value::Value;

use rustc_ast as ast;
use rustc_codegen_ssa::base::{compare_simd_types, wants_msvc_seh};
use rustc_codegen_ssa::common::span_invalid_monomorphization_error;
use rustc_codegen_ssa::common::{IntPredicate, TypeKind};
use rustc_codegen_ssa::glue;
use rustc_codegen_ssa::mir::operand::{OperandRef, OperandValue};
use rustc_codegen_ssa::mir::place::PlaceRef;
use rustc_codegen_ssa::traits::*;
use rustc_codegen_ssa::MemFlags;
use rustc_hir as hir;
use rustc_middle::ty::layout::{FnAbiExt, HasTyCtxt};
use rustc_middle::ty::{self, Ty};
use rustc_middle::{bug, span_bug};
use rustc_span::{sym, symbol::kw, Span, Symbol};
use rustc_target::abi::{self, HasDataLayout, LayoutOf, Primitive};
use rustc_target::spec::PanicStrategy;

use std::cmp::Ordering;
use std::iter;

fn get_simple_intrinsic(cx: &CodegenCx<'ll, '_>, name: Symbol) -> Option<&'ll Value> {
    let llvm_name = match name {
        sym::sqrtf32 => "llvm.sqrt.f32",
        sym::sqrtf64 => "llvm.sqrt.f64",
        sym::powif32 => "llvm.powi.f32",
        sym::powif64 => "llvm.powi.f64",
        sym::sinf32 => "llvm.sin.f32",
        sym::sinf64 => "llvm.sin.f64",
        sym::cosf32 => "llvm.cos.f32",
        sym::cosf64 => "llvm.cos.f64",
        sym::powf32 => "llvm.pow.f32",
        sym::powf64 => "llvm.pow.f64",
        sym::expf32 => "llvm.exp.f32",
        sym::expf64 => "llvm.exp.f64",
        sym::exp2f32 => "llvm.exp2.f32",
        sym::exp2f64 => "llvm.exp2.f64",
        sym::logf32 => "llvm.log.f32",
        sym::logf64 => "llvm.log.f64",
        sym::log10f32 => "llvm.log10.f32",
        sym::log10f64 => "llvm.log10.f64",
        sym::log2f32 => "llvm.log2.f32",
        sym::log2f64 => "llvm.log2.f64",
        sym::fmaf32 => "llvm.fma.f32",
        sym::fmaf64 => "llvm.fma.f64",
        sym::fabsf32 => "llvm.fabs.f32",
        sym::fabsf64 => "llvm.fabs.f64",
        sym::minnumf32 => "llvm.minnum.f32",
        sym::minnumf64 => "llvm.minnum.f64",
        sym::maxnumf32 => "llvm.maxnum.f32",
        sym::maxnumf64 => "llvm.maxnum.f64",
        sym::copysignf32 => "llvm.copysign.f32",
        sym::copysignf64 => "llvm.copysign.f64",
        sym::floorf32 => "llvm.floor.f32",
        sym::floorf64 => "llvm.floor.f64",
        sym::ceilf32 => "llvm.ceil.f32",
        sym::ceilf64 => "llvm.ceil.f64",
        sym::truncf32 => "llvm.trunc.f32",
        sym::truncf64 => "llvm.trunc.f64",
        sym::rintf32 => "llvm.rint.f32",
        sym::rintf64 => "llvm.rint.f64",
        sym::nearbyintf32 => "llvm.nearbyint.f32",
        sym::nearbyintf64 => "llvm.nearbyint.f64",
        sym::roundf32 => "llvm.round.f32",
        sym::roundf64 => "llvm.round.f64",
        sym::assume => "llvm.assume",
        sym::abort => "llvm.trap",
        _ => return None,
    };
    Some(cx.get_intrinsic(&llvm_name))
}

impl IntrinsicCallMethods<'tcx> for Builder<'a, 'll, 'tcx> {
    fn codegen_intrinsic_call(
        &mut self,
        instance: ty::Instance<'tcx>,
        fn_abi: &FnAbi<'tcx, Ty<'tcx>>,
        args: &[OperandRef<'tcx, &'ll Value>],
        llresult: &'ll Value,
        span: Span,
    ) {
        let tcx = self.tcx;
        let callee_ty = instance.ty(tcx, ty::ParamEnv::reveal_all());

        let (def_id, substs) = match *callee_ty.kind() {
            ty::FnDef(def_id, substs) => (def_id, substs),
            _ => bug!("expected fn item type, found {}", callee_ty),
        };

        let sig = callee_ty.fn_sig(tcx);
        let sig = tcx.normalize_erasing_late_bound_regions(ty::ParamEnv::reveal_all(), &sig);
        let arg_tys = sig.inputs();
        let ret_ty = sig.output();
        let name = tcx.item_name(def_id);
        let name_str = &*name.as_str();

        let llret_ty = self.layout_of(ret_ty).llvm_type(self);
        let result = PlaceRef::new_sized(llresult, fn_abi.ret.layout);

        let simple = get_simple_intrinsic(self, name);
        let llval = match name {
            _ if simple.is_some() => self.call(
                simple.unwrap(),
                &args.iter().map(|arg| arg.immediate()).collect::<Vec<_>>(),
                None,
            ),
            sym::unreachable => {
                return;
            }
            sym::likely => {
                let expect = self.get_intrinsic(&("llvm.expect.i1"));
                self.call(expect, &[args[0].immediate(), self.const_bool(true)], None)
            }
            sym::unlikely => {
                let expect = self.get_intrinsic(&("llvm.expect.i1"));
                self.call(expect, &[args[0].immediate(), self.const_bool(false)], None)
            }
            kw::Try => {
                try_intrinsic(
                    self,
                    args[0].immediate(),
                    args[1].immediate(),
                    args[2].immediate(),
                    llresult,
                );
                return;
            }
            sym::breakpoint => {
                let llfn = self.get_intrinsic(&("llvm.debugtrap"));
                self.call(llfn, &[], None)
            }
            sym::va_start => self.va_start(args[0].immediate()),
            sym::va_end => self.va_end(args[0].immediate()),
            sym::va_copy => {
                let intrinsic = self.cx().get_intrinsic(&("llvm.va_copy"));
                self.call(intrinsic, &[args[0].immediate(), args[1].immediate()], None)
            }
            sym::va_arg => {
                match fn_abi.ret.layout.abi {
                    abi::Abi::Scalar(ref scalar) => {
                        match scalar.value {
                            Primitive::Int(..) => {
                                if self.cx().size_of(ret_ty).bytes() < 4 {
                                    // `va_arg` should not be called on a integer type
                                    // less than 4 bytes in length. If it is, promote
                                    // the integer to a `i32` and truncate the result
                                    // back to the smaller type.
                                    let promoted_result = emit_va_arg(self, args[0], tcx.types.i32);
                                    self.trunc(promoted_result, llret_ty)
                                } else {
                                    emit_va_arg(self, args[0], ret_ty)
                                }
                            }
                            Primitive::F64 | Primitive::Pointer => {
                                emit_va_arg(self, args[0], ret_ty)
                            }
                            // `va_arg` should never be used with the return type f32.
                            Primitive::F32 => bug!("the va_arg intrinsic does not work with `f32`"),
                        }
                    }
                    _ => bug!("the va_arg intrinsic does not work with non-scalar types"),
                }
            }
            sym::size_of_val => {
                let tp_ty = substs.type_at(0);
                if let OperandValue::Pair(_, meta) = args[0].val {
                    let (llsize, _) = glue::size_and_align_of_dst(self, tp_ty, Some(meta));
                    llsize
                } else {
                    self.const_usize(self.size_of(tp_ty).bytes())
                }
            }
            sym::min_align_of_val => {
                let tp_ty = substs.type_at(0);
                if let OperandValue::Pair(_, meta) = args[0].val {
                    let (_, llalign) = glue::size_and_align_of_dst(self, tp_ty, Some(meta));
                    llalign
                } else {
                    self.const_usize(self.align_of(tp_ty).bytes())
                }
            }
            sym::size_of
            | sym::pref_align_of
            | sym::min_align_of
            | sym::needs_drop
            | sym::type_id
            | sym::type_name
            | sym::variant_count => {
                let value = self
                    .tcx
                    .const_eval_instance(ty::ParamEnv::reveal_all(), instance, None)
                    .unwrap();
                OperandRef::from_const(self, value, ret_ty).immediate_or_packed_pair(self)
            }
            // Effectively no-op
            sym::forget => {
                return;
            }
            sym::offset => {
                let ptr = args[0].immediate();
                let offset = args[1].immediate();
                self.inbounds_gep(ptr, &[offset])
            }
            sym::arith_offset => {
                let ptr = args[0].immediate();
                let offset = args[1].immediate();
                self.gep(ptr, &[offset])
            }

            sym::copy_nonoverlapping => {
                copy_intrinsic(
                    self,
                    false,
                    false,
                    substs.type_at(0),
                    args[1].immediate(),
                    args[0].immediate(),
                    args[2].immediate(),
                );
                return;
            }
            sym::copy => {
                copy_intrinsic(
                    self,
                    true,
                    false,
                    substs.type_at(0),
                    args[1].immediate(),
                    args[0].immediate(),
                    args[2].immediate(),
                );
                return;
            }
            sym::write_bytes => {
                memset_intrinsic(
                    self,
                    false,
                    substs.type_at(0),
                    args[0].immediate(),
                    args[1].immediate(),
                    args[2].immediate(),
                );
                return;
            }

            sym::volatile_copy_nonoverlapping_memory => {
                copy_intrinsic(
                    self,
                    false,
                    true,
                    substs.type_at(0),
                    args[0].immediate(),
                    args[1].immediate(),
                    args[2].immediate(),
                );
                return;
            }
            sym::volatile_copy_memory => {
                copy_intrinsic(
                    self,
                    true,
                    true,
                    substs.type_at(0),
                    args[0].immediate(),
                    args[1].immediate(),
                    args[2].immediate(),
                );
                return;
            }
            sym::volatile_set_memory => {
                memset_intrinsic(
                    self,
                    true,
                    substs.type_at(0),
                    args[0].immediate(),
                    args[1].immediate(),
                    args[2].immediate(),
                );
                return;
            }
            sym::volatile_load | sym::unaligned_volatile_load => {
                let tp_ty = substs.type_at(0);
                let mut ptr = args[0].immediate();
                if let PassMode::Cast(ty) = fn_abi.ret.mode {
                    ptr = self.pointercast(ptr, self.type_ptr_to(ty.llvm_type(self)));
                }
                let load = self.volatile_load(ptr);
                let align = if name == sym::unaligned_volatile_load {
                    1
                } else {
                    self.align_of(tp_ty).bytes() as u32
                };
                unsafe {
                    llvm::LLVMSetAlignment(load, align);
                }
                self.to_immediate(load, self.layout_of(tp_ty))
            }
            sym::volatile_store => {
                let dst = args[0].deref(self.cx());
                args[1].val.volatile_store(self, dst);
                return;
            }
            sym::unaligned_volatile_store => {
                let dst = args[0].deref(self.cx());
                args[1].val.unaligned_volatile_store(self, dst);
                return;
            }
            sym::prefetch_read_data
            | sym::prefetch_write_data
            | sym::prefetch_read_instruction
            | sym::prefetch_write_instruction => {
                let expect = self.get_intrinsic(&("llvm.prefetch"));
                let (rw, cache_type) = match name {
                    sym::prefetch_read_data => (0, 1),
                    sym::prefetch_write_data => (1, 1),
                    sym::prefetch_read_instruction => (0, 0),
                    sym::prefetch_write_instruction => (1, 0),
                    _ => bug!(),
                };
                self.call(
                    expect,
                    &[
                        args[0].immediate(),
                        self.const_i32(rw),
                        args[1].immediate(),
                        self.const_i32(cache_type),
                    ],
                    None,
                )
            }
            sym::ctlz
            | sym::ctlz_nonzero
            | sym::cttz
            | sym::cttz_nonzero
            | sym::ctpop
            | sym::bswap
            | sym::bitreverse
            | sym::add_with_overflow
            | sym::sub_with_overflow
            | sym::mul_with_overflow
            | sym::wrapping_add
            | sym::wrapping_sub
            | sym::wrapping_mul
            | sym::unchecked_div
            | sym::unchecked_rem
            | sym::unchecked_shl
            | sym::unchecked_shr
            | sym::unchecked_add
            | sym::unchecked_sub
            | sym::unchecked_mul
            | sym::exact_div
            | sym::rotate_left
            | sym::rotate_right
            | sym::saturating_add
            | sym::saturating_sub => {
                let ty = arg_tys[0];
                match int_type_width_signed(ty, self) {
                    Some((width, signed)) => match name {
                        sym::ctlz | sym::cttz => {
                            let y = self.const_bool(false);
                            let llfn = self.get_intrinsic(&format!("llvm.{}.i{}", name, width));
                            self.call(llfn, &[args[0].immediate(), y], None)
                        }
                        sym::ctlz_nonzero | sym::cttz_nonzero => {
                            let y = self.const_bool(true);
                            let llvm_name = &format!("llvm.{}.i{}", &name_str[..4], width);
                            let llfn = self.get_intrinsic(llvm_name);
                            self.call(llfn, &[args[0].immediate(), y], None)
                        }
                        sym::ctpop => self.call(
                            self.get_intrinsic(&format!("llvm.ctpop.i{}", width)),
                            &[args[0].immediate()],
                            None,
                        ),
                        sym::bswap => {
                            if width == 8 {
                                args[0].immediate() // byte swap a u8/i8 is just a no-op
                            } else {
                                self.call(
                                    self.get_intrinsic(&format!("llvm.bswap.i{}", width)),
                                    &[args[0].immediate()],
                                    None,
                                )
                            }
                        }
                        sym::bitreverse => self.call(
                            self.get_intrinsic(&format!("llvm.bitreverse.i{}", width)),
                            &[args[0].immediate()],
                            None,
                        ),
                        sym::add_with_overflow
                        | sym::sub_with_overflow
                        | sym::mul_with_overflow => {
                            let intrinsic = format!(
                                "llvm.{}{}.with.overflow.i{}",
                                if signed { 's' } else { 'u' },
                                &name_str[..3],
                                width
                            );
                            let llfn = self.get_intrinsic(&intrinsic);

                            // Convert `i1` to a `bool`, and write it to the out parameter
                            let pair =
                                self.call(llfn, &[args[0].immediate(), args[1].immediate()], None);
                            let val = self.extract_value(pair, 0);
                            let overflow = self.extract_value(pair, 1);
                            let overflow = self.zext(overflow, self.type_bool());

                            let dest = result.project_field(self, 0);
                            self.store(val, dest.llval, dest.align);
                            let dest = result.project_field(self, 1);
                            self.store(overflow, dest.llval, dest.align);

                            return;
                        }
                        sym::wrapping_add => self.add(args[0].immediate(), args[1].immediate()),
                        sym::wrapping_sub => self.sub(args[0].immediate(), args[1].immediate()),
                        sym::wrapping_mul => self.mul(args[0].immediate(), args[1].immediate()),
                        sym::exact_div => {
                            if signed {
                                self.exactsdiv(args[0].immediate(), args[1].immediate())
                            } else {
                                self.exactudiv(args[0].immediate(), args[1].immediate())
                            }
                        }
                        sym::unchecked_div => {
                            if signed {
                                self.sdiv(args[0].immediate(), args[1].immediate())
                            } else {
                                self.udiv(args[0].immediate(), args[1].immediate())
                            }
                        }
                        sym::unchecked_rem => {
                            if signed {
                                self.srem(args[0].immediate(), args[1].immediate())
                            } else {
                                self.urem(args[0].immediate(), args[1].immediate())
                            }
                        }
                        sym::unchecked_shl => self.shl(args[0].immediate(), args[1].immediate()),
                        sym::unchecked_shr => {
                            if signed {
                                self.ashr(args[0].immediate(), args[1].immediate())
                            } else {
                                self.lshr(args[0].immediate(), args[1].immediate())
                            }
                        }
                        sym::unchecked_add => {
                            if signed {
                                self.unchecked_sadd(args[0].immediate(), args[1].immediate())
                            } else {
                                self.unchecked_uadd(args[0].immediate(), args[1].immediate())
                            }
                        }
                        sym::unchecked_sub => {
                            if signed {
                                self.unchecked_ssub(args[0].immediate(), args[1].immediate())
                            } else {
                                self.unchecked_usub(args[0].immediate(), args[1].immediate())
                            }
                        }
                        sym::unchecked_mul => {
                            if signed {
                                self.unchecked_smul(args[0].immediate(), args[1].immediate())
                            } else {
                                self.unchecked_umul(args[0].immediate(), args[1].immediate())
                            }
                        }
                        sym::rotate_left | sym::rotate_right => {
                            let is_left = name == sym::rotate_left;
                            let val = args[0].immediate();
                            let raw_shift = args[1].immediate();
                            // rotate = funnel shift with first two args the same
                            let llvm_name =
                                &format!("llvm.fsh{}.i{}", if is_left { 'l' } else { 'r' }, width);
                            let llfn = self.get_intrinsic(llvm_name);
                            self.call(llfn, &[val, val, raw_shift], None)
                        }
                        sym::saturating_add | sym::saturating_sub => {
                            let is_add = name == sym::saturating_add;
                            let lhs = args[0].immediate();
                            let rhs = args[1].immediate();
                            let llvm_name = &format!(
                                "llvm.{}{}.sat.i{}",
                                if signed { 's' } else { 'u' },
                                if is_add { "add" } else { "sub" },
                                width
                            );
                            let llfn = self.get_intrinsic(llvm_name);
                            self.call(llfn, &[lhs, rhs], None)
                        }
                        _ => bug!(),
                    },
                    None => {
                        span_invalid_monomorphization_error(
                            tcx.sess,
                            span,
                            &format!(
                                "invalid monomorphization of `{}` intrinsic: \
                                      expected basic integer type, found `{}`",
                                name, ty
                            ),
                        );
                        return;
                    }
                }
            }
            sym::fadd_fast | sym::fsub_fast | sym::fmul_fast | sym::fdiv_fast | sym::frem_fast => {
                match float_type_width(arg_tys[0]) {
                    Some(_width) => match name {
                        sym::fadd_fast => self.fadd_fast(args[0].immediate(), args[1].immediate()),
                        sym::fsub_fast => self.fsub_fast(args[0].immediate(), args[1].immediate()),
                        sym::fmul_fast => self.fmul_fast(args[0].immediate(), args[1].immediate()),
                        sym::fdiv_fast => self.fdiv_fast(args[0].immediate(), args[1].immediate()),
                        sym::frem_fast => self.frem_fast(args[0].immediate(), args[1].immediate()),
                        _ => bug!(),
                    },
                    None => {
                        span_invalid_monomorphization_error(
                            tcx.sess,
                            span,
                            &format!(
                                "invalid monomorphization of `{}` intrinsic: \
                                      expected basic float type, found `{}`",
                                name, arg_tys[0]
                            ),
                        );
                        return;
                    }
                }
            }

            sym::float_to_int_unchecked => {
                if float_type_width(arg_tys[0]).is_none() {
                    span_invalid_monomorphization_error(
                        tcx.sess,
                        span,
                        &format!(
                            "invalid monomorphization of `float_to_int_unchecked` \
                                  intrinsic: expected basic float type, \
                                  found `{}`",
                            arg_tys[0]
                        ),
                    );
                    return;
                }
                let (width, signed) = match int_type_width_signed(ret_ty, self.cx) {
                    Some(pair) => pair,
                    None => {
                        span_invalid_monomorphization_error(
                            tcx.sess,
                            span,
                            &format!(
                                "invalid monomorphization of `float_to_int_unchecked` \
                                      intrinsic:  expected basic integer type, \
                                      found `{}`",
                                ret_ty
                            ),
                        );
                        return;
                    }
                };
                if signed {
                    self.fptosi(args[0].immediate(), self.cx.type_ix(width))
                } else {
                    self.fptoui(args[0].immediate(), self.cx.type_ix(width))
                }
            }

            sym::discriminant_value => {
                if ret_ty.is_integral() {
                    args[0].deref(self.cx()).codegen_get_discr(self, ret_ty)
                } else {
                    span_bug!(span, "Invalid discriminant type for `{:?}`", arg_tys[0])
                }
            }

            _ if name_str.starts_with("simd_") => {
                match generic_simd_intrinsic(self, name, callee_ty, args, ret_ty, llret_ty, span) {
                    Ok(llval) => llval,
                    Err(()) => return,
                }
            }
            // This requires that atomic intrinsics follow a specific naming pattern:
            // "atomic_<operation>[_<ordering>]", and no ordering means SeqCst
            name if name_str.starts_with("atomic_") => {
                use rustc_codegen_ssa::common::AtomicOrdering::*;
                use rustc_codegen_ssa::common::{AtomicRmwBinOp, SynchronizationScope};

                let split: Vec<&str> = name_str.split('_').collect();

                let is_cxchg = split[1] == "cxchg" || split[1] == "cxchgweak";
                let (order, failorder) = match split.len() {
                    2 => (SequentiallyConsistent, SequentiallyConsistent),
                    3 => match split[2] {
                        "unordered" => (Unordered, Unordered),
                        "relaxed" => (Monotonic, Monotonic),
                        "acq" => (Acquire, Acquire),
                        "rel" => (Release, Monotonic),
                        "acqrel" => (AcquireRelease, Acquire),
                        "failrelaxed" if is_cxchg => (SequentiallyConsistent, Monotonic),
                        "failacq" if is_cxchg => (SequentiallyConsistent, Acquire),
                        _ => self.sess().fatal("unknown ordering in atomic intrinsic"),
                    },
                    4 => match (split[2], split[3]) {
                        ("acq", "failrelaxed") if is_cxchg => (Acquire, Monotonic),
                        ("acqrel", "failrelaxed") if is_cxchg => (AcquireRelease, Monotonic),
                        _ => self.sess().fatal("unknown ordering in atomic intrinsic"),
                    },
                    _ => self.sess().fatal("Atomic intrinsic not in correct format"),
                };

                let invalid_monomorphization = |ty| {
                    span_invalid_monomorphization_error(
                        tcx.sess,
                        span,
                        &format!(
                            "invalid monomorphization of `{}` intrinsic: \
                                  expected basic integer type, found `{}`",
                            name, ty
                        ),
                    );
                };

                match split[1] {
                    "cxchg" | "cxchgweak" => {
                        let ty = substs.type_at(0);
                        if int_type_width_signed(ty, self).is_some() {
                            let weak = split[1] == "cxchgweak";
                            let pair = self.atomic_cmpxchg(
                                args[0].immediate(),
                                args[1].immediate(),
                                args[2].immediate(),
                                order,
                                failorder,
                                weak,
                            );
                            let val = self.extract_value(pair, 0);
                            let success = self.extract_value(pair, 1);
                            let success = self.zext(success, self.type_bool());

                            let dest = result.project_field(self, 0);
                            self.store(val, dest.llval, dest.align);
                            let dest = result.project_field(self, 1);
                            self.store(success, dest.llval, dest.align);
                            return;
                        } else {
                            return invalid_monomorphization(ty);
                        }
                    }

                    "load" => {
                        let ty = substs.type_at(0);
                        if int_type_width_signed(ty, self).is_some() {
                            let size = self.size_of(ty);
                            self.atomic_load(args[0].immediate(), order, size)
                        } else {
                            return invalid_monomorphization(ty);
                        }
                    }

                    "store" => {
                        let ty = substs.type_at(0);
                        if int_type_width_signed(ty, self).is_some() {
                            let size = self.size_of(ty);
                            self.atomic_store(
                                args[1].immediate(),
                                args[0].immediate(),
                                order,
                                size,
                            );
                            return;
                        } else {
                            return invalid_monomorphization(ty);
                        }
                    }

                    "fence" => {
                        self.atomic_fence(order, SynchronizationScope::CrossThread);
                        return;
                    }

                    "singlethreadfence" => {
                        self.atomic_fence(order, SynchronizationScope::SingleThread);
                        return;
                    }

                    // These are all AtomicRMW ops
                    op => {
                        let atom_op = match op {
                            "xchg" => AtomicRmwBinOp::AtomicXchg,
                            "xadd" => AtomicRmwBinOp::AtomicAdd,
                            "xsub" => AtomicRmwBinOp::AtomicSub,
                            "and" => AtomicRmwBinOp::AtomicAnd,
                            "nand" => AtomicRmwBinOp::AtomicNand,
                            "or" => AtomicRmwBinOp::AtomicOr,
                            "xor" => AtomicRmwBinOp::AtomicXor,
                            "max" => AtomicRmwBinOp::AtomicMax,
                            "min" => AtomicRmwBinOp::AtomicMin,
                            "umax" => AtomicRmwBinOp::AtomicUMax,
                            "umin" => AtomicRmwBinOp::AtomicUMin,
                            _ => self.sess().fatal("unknown atomic operation"),
                        };

                        let ty = substs.type_at(0);
                        if int_type_width_signed(ty, self).is_some() {
                            self.atomic_rmw(
                                atom_op,
                                args[0].immediate(),
                                args[1].immediate(),
                                order,
                            )
                        } else {
                            return invalid_monomorphization(ty);
                        }
                    }
                }
            }

            sym::nontemporal_store => {
                let dst = args[0].deref(self.cx());
                args[1].val.nontemporal_store(self, dst);
                return;
            }

            sym::ptr_guaranteed_eq | sym::ptr_guaranteed_ne => {
                let a = args[0].immediate();
                let b = args[1].immediate();
                if name == sym::ptr_guaranteed_eq {
                    self.icmp(IntPredicate::IntEQ, a, b)
                } else {
                    self.icmp(IntPredicate::IntNE, a, b)
                }
            }

            sym::ptr_offset_from => {
                let ty = substs.type_at(0);
                let pointee_size = self.size_of(ty);

                // This is the same sequence that Clang emits for pointer subtraction.
                // It can be neither `nsw` nor `nuw` because the input is treated as
                // unsigned but then the output is treated as signed, so neither works.
                let a = args[0].immediate();
                let b = args[1].immediate();
                let a = self.ptrtoint(a, self.type_isize());
                let b = self.ptrtoint(b, self.type_isize());
                let d = self.sub(a, b);
                let pointee_size = self.const_usize(pointee_size.bytes());
                // this is where the signed magic happens (notice the `s` in `exactsdiv`)
                self.exactsdiv(d, pointee_size)
            }

            _ => bug!("unknown intrinsic '{}'", name),
        };

        if !fn_abi.ret.is_ignore() {
            if let PassMode::Cast(ty) = fn_abi.ret.mode {
                let ptr_llty = self.type_ptr_to(ty.llvm_type(self));
                let ptr = self.pointercast(result.llval, ptr_llty);
                self.store(llval, ptr, result.align);
            } else {
                OperandRef::from_immediate_or_packed_pair(self, llval, result.layout)
                    .val
                    .store(self, result);
            }
        }
    }

    fn abort(&mut self) {
        let fnname = self.get_intrinsic(&("llvm.trap"));
        self.call(fnname, &[], None);
    }

    fn assume(&mut self, val: Self::Value) {
        let assume_intrinsic = self.get_intrinsic("llvm.assume");
        self.call(assume_intrinsic, &[val], None);
    }

    fn expect(&mut self, cond: Self::Value, expected: bool) -> Self::Value {
        let expect = self.get_intrinsic(&"llvm.expect.i1");
        self.call(expect, &[cond, self.const_bool(expected)], None)
    }

    fn sideeffect(&mut self) {
        if self.tcx.sess.opts.debugging_opts.insert_sideeffect {
            let fnname = self.get_intrinsic(&("llvm.sideeffect"));
            self.call(fnname, &[], None);
        }
    }

    fn va_start(&mut self, va_list: &'ll Value) -> &'ll Value {
        let intrinsic = self.cx().get_intrinsic("llvm.va_start");
        self.call(intrinsic, &[va_list], None)
    }

    fn va_end(&mut self, va_list: &'ll Value) -> &'ll Value {
        let intrinsic = self.cx().get_intrinsic("llvm.va_end");
        self.call(intrinsic, &[va_list], None)
    }
}

fn copy_intrinsic(
    bx: &mut Builder<'a, 'll, 'tcx>,
    allow_overlap: bool,
    volatile: bool,
    ty: Ty<'tcx>,
    dst: &'ll Value,
    src: &'ll Value,
    count: &'ll Value,
) {
    let (size, align) = bx.size_and_align_of(ty);
    let size = bx.mul(bx.const_usize(size.bytes()), count);
    let flags = if volatile { MemFlags::VOLATILE } else { MemFlags::empty() };
    if allow_overlap {
        bx.memmove(dst, align, src, align, size, flags);
    } else {
        bx.memcpy(dst, align, src, align, size, flags);
    }
}

fn memset_intrinsic(
    bx: &mut Builder<'a, 'll, 'tcx>,
    volatile: bool,
    ty: Ty<'tcx>,
    dst: &'ll Value,
    val: &'ll Value,
    count: &'ll Value,
) {
    let (size, align) = bx.size_and_align_of(ty);
    let size = bx.mul(bx.const_usize(size.bytes()), count);
    let flags = if volatile { MemFlags::VOLATILE } else { MemFlags::empty() };
    bx.memset(dst, val, size, align, flags);
}

fn try_intrinsic(
    bx: &mut Builder<'a, 'll, 'tcx>,
    try_func: &'ll Value,
    data: &'ll Value,
    catch_func: &'ll Value,
    dest: &'ll Value,
) {
    if bx.sess().panic_strategy() == PanicStrategy::Abort {
        bx.call(try_func, &[data], None);
        // Return 0 unconditionally from the intrinsic call;
        // we can never unwind.
        let ret_align = bx.tcx().data_layout.i32_align.abi;
        bx.store(bx.const_i32(0), dest, ret_align);
    } else if wants_msvc_seh(bx.sess()) {
        codegen_msvc_try(bx, try_func, data, catch_func, dest);
    } else if bx.sess().target.target.options.is_like_emscripten {
        codegen_emcc_try(bx, try_func, data, catch_func, dest);
    } else {
        codegen_gnu_try(bx, try_func, data, catch_func, dest);
    }
}

// MSVC's definition of the `rust_try` function.
//
// This implementation uses the new exception handling instructions in LLVM
// which have support in LLVM for SEH on MSVC targets. Although these
// instructions are meant to work for all targets, as of the time of this
// writing, however, LLVM does not recommend the usage of these new instructions
// as the old ones are still more optimized.
fn codegen_msvc_try(
    bx: &mut Builder<'a, 'll, 'tcx>,
    try_func: &'ll Value,
    data: &'ll Value,
    catch_func: &'ll Value,
    dest: &'ll Value,
) {
    let llfn = get_rust_try_fn(bx, &mut |mut bx| {
        bx.set_personality_fn(bx.eh_personality());
        bx.sideeffect();

        let mut normal = bx.build_sibling_block("normal");
        let mut catchswitch = bx.build_sibling_block("catchswitch");
        let mut catchpad_rust = bx.build_sibling_block("catchpad_rust");
        let mut catchpad_foreign = bx.build_sibling_block("catchpad_foreign");
        let mut caught = bx.build_sibling_block("caught");

        let try_func = llvm::get_param(bx.llfn(), 0);
        let data = llvm::get_param(bx.llfn(), 1);
        let catch_func = llvm::get_param(bx.llfn(), 2);

        // We're generating an IR snippet that looks like:
        //
        //   declare i32 @rust_try(%try_func, %data, %catch_func) {
        //      %slot = alloca i8*
        //      invoke %try_func(%data) to label %normal unwind label %catchswitch
        //
        //   normal:
        //      ret i32 0
        //
        //   catchswitch:
        //      %cs = catchswitch within none [%catchpad_rust, %catchpad_foreign] unwind to caller
        //
        //   catchpad_rust:
        //      %tok = catchpad within %cs [%type_descriptor, 8, %slot]
        //      %ptr = load %slot
        //      call %catch_func(%data, %ptr)
        //      catchret from %tok to label %caught
        //
        //   catchpad_foreign:
        //      %tok = catchpad within %cs [null, 64, null]
        //      call %catch_func(%data, null)
        //      catchret from %tok to label %caught
        //
        //   caught:
        //      ret i32 1
        //   }
        //
        // This structure follows the basic usage of throw/try/catch in LLVM.
        // For example, compile this C++ snippet to see what LLVM generates:
        //
        //      struct rust_panic {
        //          rust_panic(const rust_panic&);
        //          ~rust_panic();
        //
        //          void* x[2];
        //      };
        //
        //      int __rust_try(
        //          void (*try_func)(void*),
        //          void *data,
        //          void (*catch_func)(void*, void*) noexcept
        //      ) {
        //          try {
        //              try_func(data);
        //              return 0;
        //          } catch(rust_panic& a) {
        //              catch_func(data, &a);
        //              return 1;
        //          } catch(...) {
        //              catch_func(data, NULL);
        //              return 1;
        //          }
        //      }
        //
        // More information can be found in libstd's seh.rs implementation.
        let ptr_align = bx.tcx().data_layout.pointer_align.abi;
        let slot = bx.alloca(bx.type_i8p(), ptr_align);
        bx.invoke(try_func, &[data], normal.llbb(), catchswitch.llbb(), None);

        normal.ret(bx.const_i32(0));

        let cs = catchswitch.catch_switch(None, None, 2);
        catchswitch.add_handler(cs, catchpad_rust.llbb());
        catchswitch.add_handler(cs, catchpad_foreign.llbb());

        // We can't use the TypeDescriptor defined in libpanic_unwind because it
        // might be in another DLL and the SEH encoding only supports specifying
        // a TypeDescriptor from the current module.
        //
        // However this isn't an issue since the MSVC runtime uses string
        // comparison on the type name to match TypeDescriptors rather than
        // pointer equality.
        //
        // So instead we generate a new TypeDescriptor in each module that uses
        // `try` and let the linker merge duplicate definitions in the same
        // module.
        //
        // When modifying, make sure that the type_name string exactly matches
        // the one used in src/libpanic_unwind/seh.rs.
        let type_info_vtable = bx.declare_global("??_7type_info@@6B@", bx.type_i8p());
        let type_name = bx.const_bytes(b"rust_panic\0");
        let type_info =
            bx.const_struct(&[type_info_vtable, bx.const_null(bx.type_i8p()), type_name], false);
        let tydesc = bx.declare_global("__rust_panic_type_info", bx.val_ty(type_info));
        unsafe {
            llvm::LLVMRustSetLinkage(tydesc, llvm::Linkage::LinkOnceODRLinkage);
            llvm::SetUniqueComdat(bx.llmod, tydesc);
            llvm::LLVMSetInitializer(tydesc, type_info);
        }

        // The flag value of 8 indicates that we are catching the exception by
        // reference instead of by value. We can't use catch by value because
        // that requires copying the exception object, which we don't support
        // since our exception object effectively contains a Box.
        //
        // Source: MicrosoftCXXABI::getAddrOfCXXCatchHandlerType in clang
        let flags = bx.const_i32(8);
        let funclet = catchpad_rust.catch_pad(cs, &[tydesc, flags, slot]);
        let ptr = catchpad_rust.load(slot, ptr_align);
        catchpad_rust.call(catch_func, &[data, ptr], Some(&funclet));
        catchpad_rust.catch_ret(&funclet, caught.llbb());

        // The flag value of 64 indicates a "catch-all".
        let flags = bx.const_i32(64);
        let null = bx.const_null(bx.type_i8p());
        let funclet = catchpad_foreign.catch_pad(cs, &[null, flags, null]);
        catchpad_foreign.call(catch_func, &[data, null], Some(&funclet));
        catchpad_foreign.catch_ret(&funclet, caught.llbb());

        caught.ret(bx.const_i32(1));
    });

    // Note that no invoke is used here because by definition this function
    // can't panic (that's what it's catching).
    let ret = bx.call(llfn, &[try_func, data, catch_func], None);
    let i32_align = bx.tcx().data_layout.i32_align.abi;
    bx.store(ret, dest, i32_align);
}

// Definition of the standard `try` function for Rust using the GNU-like model
// of exceptions (e.g., the normal semantics of LLVM's `landingpad` and `invoke`
// instructions).
//
// This codegen is a little surprising because we always call a shim
// function instead of inlining the call to `invoke` manually here. This is done
// because in LLVM we're only allowed to have one personality per function
// definition. The call to the `try` intrinsic is being inlined into the
// function calling it, and that function may already have other personality
// functions in play. By calling a shim we're guaranteed that our shim will have
// the right personality function.
fn codegen_gnu_try(
    bx: &mut Builder<'a, 'll, 'tcx>,
    try_func: &'ll Value,
    data: &'ll Value,
    catch_func: &'ll Value,
    dest: &'ll Value,
) {
    let llfn = get_rust_try_fn(bx, &mut |mut bx| {
        // Codegens the shims described above:
        //
        //   bx:
        //      invoke %try_func(%data) normal %normal unwind %catch
        //
        //   normal:
        //      ret 0
        //
        //   catch:
        //      (%ptr, _) = landingpad
        //      call %catch_func(%data, %ptr)
        //      ret 1

        bx.sideeffect();

        let mut then = bx.build_sibling_block("then");
        let mut catch = bx.build_sibling_block("catch");

        let try_func = llvm::get_param(bx.llfn(), 0);
        let data = llvm::get_param(bx.llfn(), 1);
        let catch_func = llvm::get_param(bx.llfn(), 2);
        bx.invoke(try_func, &[data], then.llbb(), catch.llbb(), None);
        then.ret(bx.const_i32(0));

        // Type indicator for the exception being thrown.
        //
        // The first value in this tuple is a pointer to the exception object
        // being thrown.  The second value is a "selector" indicating which of
        // the landing pad clauses the exception's type had been matched to.
        // rust_try ignores the selector.
        let lpad_ty = bx.type_struct(&[bx.type_i8p(), bx.type_i32()], false);
        let vals = catch.landing_pad(lpad_ty, bx.eh_personality(), 1);
        let tydesc = bx.const_null(bx.type_i8p());
        catch.add_clause(vals, tydesc);
        let ptr = catch.extract_value(vals, 0);
        catch.call(catch_func, &[data, ptr], None);
        catch.ret(bx.const_i32(1));
    });

    // Note that no invoke is used here because by definition this function
    // can't panic (that's what it's catching).
    let ret = bx.call(llfn, &[try_func, data, catch_func], None);
    let i32_align = bx.tcx().data_layout.i32_align.abi;
    bx.store(ret, dest, i32_align);
}

// Variant of codegen_gnu_try used for emscripten where Rust panics are
// implemented using C++ exceptions. Here we use exceptions of a specific type
// (`struct rust_panic`) to represent Rust panics.
fn codegen_emcc_try(
    bx: &mut Builder<'a, 'll, 'tcx>,
    try_func: &'ll Value,
    data: &'ll Value,
    catch_func: &'ll Value,
    dest: &'ll Value,
) {
    let llfn = get_rust_try_fn(bx, &mut |mut bx| {
        // Codegens the shims described above:
        //
        //   bx:
        //      invoke %try_func(%data) normal %normal unwind %catch
        //
        //   normal:
        //      ret 0
        //
        //   catch:
        //      (%ptr, %selector) = landingpad
        //      %rust_typeid = @llvm.eh.typeid.for(@_ZTI10rust_panic)
        //      %is_rust_panic = %selector == %rust_typeid
        //      %catch_data = alloca { i8*, i8 }
        //      %catch_data[0] = %ptr
        //      %catch_data[1] = %is_rust_panic
        //      call %catch_func(%data, %catch_data)
        //      ret 1

        bx.sideeffect();

        let mut then = bx.build_sibling_block("then");
        let mut catch = bx.build_sibling_block("catch");

        let try_func = llvm::get_param(bx.llfn(), 0);
        let data = llvm::get_param(bx.llfn(), 1);
        let catch_func = llvm::get_param(bx.llfn(), 2);
        bx.invoke(try_func, &[data], then.llbb(), catch.llbb(), None);
        then.ret(bx.const_i32(0));

        // Type indicator for the exception being thrown.
        //
        // The first value in this tuple is a pointer to the exception object
        // being thrown.  The second value is a "selector" indicating which of
        // the landing pad clauses the exception's type had been matched to.
        let tydesc = bx.eh_catch_typeinfo();
        let lpad_ty = bx.type_struct(&[bx.type_i8p(), bx.type_i32()], false);
        let vals = catch.landing_pad(lpad_ty, bx.eh_personality(), 2);
        catch.add_clause(vals, tydesc);
        catch.add_clause(vals, bx.const_null(bx.type_i8p()));
        let ptr = catch.extract_value(vals, 0);
        let selector = catch.extract_value(vals, 1);

        // Check if the typeid we got is the one for a Rust panic.
        let llvm_eh_typeid_for = bx.get_intrinsic("llvm.eh.typeid.for");
        let rust_typeid = catch.call(llvm_eh_typeid_for, &[tydesc], None);
        let is_rust_panic = catch.icmp(IntPredicate::IntEQ, selector, rust_typeid);
        let is_rust_panic = catch.zext(is_rust_panic, bx.type_bool());

        // We need to pass two values to catch_func (ptr and is_rust_panic), so
        // create an alloca and pass a pointer to that.
        let ptr_align = bx.tcx().data_layout.pointer_align.abi;
        let i8_align = bx.tcx().data_layout.i8_align.abi;
        let catch_data =
            catch.alloca(bx.type_struct(&[bx.type_i8p(), bx.type_bool()], false), ptr_align);
        let catch_data_0 = catch.inbounds_gep(catch_data, &[bx.const_usize(0), bx.const_usize(0)]);
        catch.store(ptr, catch_data_0, ptr_align);
        let catch_data_1 = catch.inbounds_gep(catch_data, &[bx.const_usize(0), bx.const_usize(1)]);
        catch.store(is_rust_panic, catch_data_1, i8_align);
        let catch_data = catch.bitcast(catch_data, bx.type_i8p());

        catch.call(catch_func, &[data, catch_data], None);
        catch.ret(bx.const_i32(1));
    });

    // Note that no invoke is used here because by definition this function
    // can't panic (that's what it's catching).
    let ret = bx.call(llfn, &[try_func, data, catch_func], None);
    let i32_align = bx.tcx().data_layout.i32_align.abi;
    bx.store(ret, dest, i32_align);
}

// Helper function to give a Block to a closure to codegen a shim function.
// This is currently primarily used for the `try` intrinsic functions above.
fn gen_fn<'ll, 'tcx>(
    cx: &CodegenCx<'ll, 'tcx>,
    name: &str,
    inputs: Vec<Ty<'tcx>>,
    output: Ty<'tcx>,
    codegen: &mut dyn FnMut(Builder<'_, 'll, 'tcx>),
) -> &'ll Value {
    let rust_fn_sig = ty::Binder::bind(cx.tcx.mk_fn_sig(
        inputs.into_iter(),
        output,
        false,
        hir::Unsafety::Unsafe,
        Abi::Rust,
    ));
    let fn_abi = FnAbi::of_fn_ptr(cx, rust_fn_sig, &[]);
    let llfn = cx.declare_fn(name, &fn_abi);
    cx.set_frame_pointer_elimination(llfn);
    cx.apply_target_cpu_attr(llfn);
    // FIXME(eddyb) find a nicer way to do this.
    unsafe { llvm::LLVMRustSetLinkage(llfn, llvm::Linkage::InternalLinkage) };
    let bx = Builder::new_block(cx, llfn, "entry-block");
    codegen(bx);
    llfn
}

// Helper function used to get a handle to the `__rust_try` function used to
// catch exceptions.
//
// This function is only generated once and is then cached.
fn get_rust_try_fn<'ll, 'tcx>(
    cx: &CodegenCx<'ll, 'tcx>,
    codegen: &mut dyn FnMut(Builder<'_, 'll, 'tcx>),
) -> &'ll Value {
    if let Some(llfn) = cx.rust_try_fn.get() {
        return llfn;
    }

    // Define the type up front for the signature of the rust_try function.
    let tcx = cx.tcx;
    let i8p = tcx.mk_mut_ptr(tcx.types.i8);
    let try_fn_ty = tcx.mk_fn_ptr(ty::Binder::bind(tcx.mk_fn_sig(
        iter::once(i8p),
        tcx.mk_unit(),
        false,
        hir::Unsafety::Unsafe,
        Abi::Rust,
    )));
    let catch_fn_ty = tcx.mk_fn_ptr(ty::Binder::bind(tcx.mk_fn_sig(
        [i8p, i8p].iter().cloned(),
        tcx.mk_unit(),
        false,
        hir::Unsafety::Unsafe,
        Abi::Rust,
    )));
    let output = tcx.types.i32;
    let rust_try = gen_fn(cx, "__rust_try", vec![try_fn_ty, i8p, catch_fn_ty], output, codegen);
    cx.rust_try_fn.set(Some(rust_try));
    rust_try
}

fn generic_simd_intrinsic(
    bx: &mut Builder<'a, 'll, 'tcx>,
    name: Symbol,
    callee_ty: Ty<'tcx>,
    args: &[OperandRef<'tcx, &'ll Value>],
    ret_ty: Ty<'tcx>,
    llret_ty: &'ll Type,
    span: Span,
) -> Result<&'ll Value, ()> {
    // macros for error handling:
    macro_rules! emit_error {
        ($msg: tt) => {
            emit_error!($msg, )
        };
        ($msg: tt, $($fmt: tt)*) => {
            span_invalid_monomorphization_error(
                bx.sess(), span,
                &format!(concat!("invalid monomorphization of `{}` intrinsic: ", $msg),
                         name, $($fmt)*));
        }
    }

    macro_rules! return_error {
        ($($fmt: tt)*) => {
            {
                emit_error!($($fmt)*);
                return Err(());
            }
        }
    }

    macro_rules! require {
        ($cond: expr, $($fmt: tt)*) => {
            if !$cond {
                return_error!($($fmt)*);
            }
        };
    }

    macro_rules! require_simd {
        ($ty: expr, $position: expr) => {
            require!($ty.is_simd(), "expected SIMD {} type, found non-SIMD `{}`", $position, $ty)
        };
    }

    let tcx = bx.tcx();
    let sig = tcx
        .normalize_erasing_late_bound_regions(ty::ParamEnv::reveal_all(), &callee_ty.fn_sig(tcx));
    let arg_tys = sig.inputs();
    let name_str = &*name.as_str();

    if name == sym::simd_select_bitmask {
        let in_ty = arg_tys[0];
        let m_len = match in_ty.kind() {
            // Note that this `.unwrap()` crashes for isize/usize, that's sort
            // of intentional as there's not currently a use case for that.
            ty::Int(i) => i.bit_width().unwrap(),
            ty::Uint(i) => i.bit_width().unwrap(),
            _ => return_error!("`{}` is not an integral type", in_ty),
        };
        require_simd!(arg_tys[1], "argument");
        let v_len = arg_tys[1].simd_size(tcx);
        require!(
            m_len == v_len,
            "mismatched lengths: mask length `{}` != other vector length `{}`",
            m_len,
            v_len
        );
        let i1 = bx.type_i1();
        let i1xn = bx.type_vector(i1, m_len);
        let m_i1s = bx.bitcast(args[0].immediate(), i1xn);
        return Ok(bx.select(m_i1s, args[1].immediate(), args[2].immediate()));
    }

    // every intrinsic below takes a SIMD vector as its first argument
    require_simd!(arg_tys[0], "input");
    let in_ty = arg_tys[0];
    let in_elem = arg_tys[0].simd_type(tcx);
    let in_len = arg_tys[0].simd_size(tcx);

    let comparison = match name {
        sym::simd_eq => Some(hir::BinOpKind::Eq),
        sym::simd_ne => Some(hir::BinOpKind::Ne),
        sym::simd_lt => Some(hir::BinOpKind::Lt),
        sym::simd_le => Some(hir::BinOpKind::Le),
        sym::simd_gt => Some(hir::BinOpKind::Gt),
        sym::simd_ge => Some(hir::BinOpKind::Ge),
        _ => None,
    };

    if let Some(cmp_op) = comparison {
        require_simd!(ret_ty, "return");

        let out_len = ret_ty.simd_size(tcx);
        require!(
            in_len == out_len,
            "expected return type with length {} (same as input type `{}`), \
                  found `{}` with length {}",
            in_len,
            in_ty,
            ret_ty,
            out_len
        );
        require!(
            bx.type_kind(bx.element_type(llret_ty)) == TypeKind::Integer,
            "expected return type with integer elements, found `{}` with non-integer `{}`",
            ret_ty,
            ret_ty.simd_type(tcx)
        );

        return Ok(compare_simd_types(
            bx,
            args[0].immediate(),
            args[1].immediate(),
            in_elem,
            llret_ty,
            cmp_op,
        ));
    }

    if name_str.starts_with("simd_shuffle") {
        let n: u64 = name_str["simd_shuffle".len()..].parse().unwrap_or_else(|_| {
            span_bug!(span, "bad `simd_shuffle` instruction only caught in codegen?")
        });

        require_simd!(ret_ty, "return");

        let out_len = ret_ty.simd_size(tcx);
        require!(
            out_len == n,
            "expected return type of length {}, found `{}` with length {}",
            n,
            ret_ty,
            out_len
        );
        require!(
            in_elem == ret_ty.simd_type(tcx),
            "expected return element type `{}` (element of input `{}`), \
                  found `{}` with element type `{}`",
            in_elem,
            in_ty,
            ret_ty,
            ret_ty.simd_type(tcx)
        );

        let total_len = u128::from(in_len) * 2;

        let vector = args[2].immediate();

        let indices: Option<Vec<_>> = (0..n)
            .map(|i| {
                let arg_idx = i;
                let val = bx.const_get_elt(vector, i as u64);
                match bx.const_to_opt_u128(val, true) {
                    None => {
                        emit_error!("shuffle index #{} is not a constant", arg_idx);
                        None
                    }
                    Some(idx) if idx >= total_len => {
                        emit_error!(
                            "shuffle index #{} is out of bounds (limit {})",
                            arg_idx,
                            total_len
                        );
                        None
                    }
                    Some(idx) => Some(bx.const_i32(idx as i32)),
                }
            })
            .collect();
        let indices = match indices {
            Some(i) => i,
            None => return Ok(bx.const_null(llret_ty)),
        };

        return Ok(bx.shuffle_vector(
            args[0].immediate(),
            args[1].immediate(),
            bx.const_vector(&indices),
        ));
    }

    if name == sym::simd_insert {
        require!(
            in_elem == arg_tys[2],
            "expected inserted type `{}` (element of input `{}`), found `{}`",
            in_elem,
            in_ty,
            arg_tys[2]
        );
        return Ok(bx.insert_element(
            args[0].immediate(),
            args[2].immediate(),
            args[1].immediate(),
        ));
    }
    if name == sym::simd_extract {
        require!(
            ret_ty == in_elem,
            "expected return type `{}` (element of input `{}`), found `{}`",
            in_elem,
            in_ty,
            ret_ty
        );
        return Ok(bx.extract_element(args[0].immediate(), args[1].immediate()));
    }

    if name == sym::simd_select {
        let m_elem_ty = in_elem;
        let m_len = in_len;
        require_simd!(arg_tys[1], "argument");
        let v_len = arg_tys[1].simd_size(tcx);
        require!(
            m_len == v_len,
            "mismatched lengths: mask length `{}` != other vector length `{}`",
            m_len,
            v_len
        );
        match m_elem_ty.kind() {
            ty::Int(_) => {}
            _ => return_error!("mask element type is `{}`, expected `i_`", m_elem_ty),
        }
        // truncate the mask to a vector of i1s
        let i1 = bx.type_i1();
        let i1xn = bx.type_vector(i1, m_len as u64);
        let m_i1s = bx.trunc(args[0].immediate(), i1xn);
        return Ok(bx.select(m_i1s, args[1].immediate(), args[2].immediate()));
    }

    if name == sym::simd_bitmask {
        // The `fn simd_bitmask(vector) -> unsigned integer` intrinsic takes a
        // vector mask and returns an unsigned integer containing the most
        // significant bit (MSB) of each lane.

        // If the vector has less than 8 lanes, an u8 is returned with zeroed
        // trailing bits.
        let expected_int_bits = in_len.max(8);
        match ret_ty.kind() {
            ty::Uint(i) if i.bit_width() == Some(expected_int_bits) => (),
            _ => return_error!("bitmask `{}`, expected `u{}`", ret_ty, expected_int_bits),
        }

        // Integer vector <i{in_bitwidth} x in_len>:
        let (i_xn, in_elem_bitwidth) = match in_elem.kind() {
            ty::Int(i) => {
                (args[0].immediate(), i.bit_width().unwrap_or(bx.data_layout().pointer_size.bits()))
            }
            ty::Uint(i) => {
                (args[0].immediate(), i.bit_width().unwrap_or(bx.data_layout().pointer_size.bits()))
            }
            _ => return_error!(
                "vector argument `{}`'s element type `{}`, expected integer element type",
                in_ty,
                in_elem
            ),
        };

        // Shift the MSB to the right by "in_elem_bitwidth - 1" into the first bit position.
        let shift_indices =
            vec![
                bx.cx.const_int(bx.type_ix(in_elem_bitwidth), (in_elem_bitwidth - 1) as _);
                in_len as _
            ];
        let i_xn_msb = bx.lshr(i_xn, bx.const_vector(shift_indices.as_slice()));
        // Truncate vector to an <i1 x N>
        let i1xn = bx.trunc(i_xn_msb, bx.type_vector(bx.type_i1(), in_len));
        // Bitcast <i1 x N> to iN:
        let i_ = bx.bitcast(i1xn, bx.type_ix(in_len));
        // Zero-extend iN to the bitmask type:
        return Ok(bx.zext(i_, bx.type_ix(expected_int_bits)));
    }

    fn simd_simple_float_intrinsic(
        name: &str,
        in_elem: &::rustc_middle::ty::TyS<'_>,
        in_ty: &::rustc_middle::ty::TyS<'_>,
        in_len: u64,
        bx: &mut Builder<'a, 'll, 'tcx>,
        span: Span,
        args: &[OperandRef<'tcx, &'ll Value>],
    ) -> Result<&'ll Value, ()> {
        macro_rules! emit_error {
            ($msg: tt) => {
                emit_error!($msg, )
            };
            ($msg: tt, $($fmt: tt)*) => {
                span_invalid_monomorphization_error(
                    bx.sess(), span,
                    &format!(concat!("invalid monomorphization of `{}` intrinsic: ", $msg),
                             name, $($fmt)*));
            }
        }
        macro_rules! return_error {
            ($($fmt: tt)*) => {
                {
                    emit_error!($($fmt)*);
                    return Err(());
                }
            }
        }
        let ety = match in_elem.kind() {
            ty::Float(f) if f.bit_width() == 32 => {
                if in_len < 2 || in_len > 16 {
                    return_error!(
                        "unsupported floating-point vector `{}` with length `{}` \
                         out-of-range [2, 16]",
                        in_ty,
                        in_len
                    );
                }
                "f32"
            }
            ty::Float(f) if f.bit_width() == 64 => {
                if in_len < 2 || in_len > 8 {
                    return_error!(
                        "unsupported floating-point vector `{}` with length `{}` \
                                   out-of-range [2, 8]",
                        in_ty,
                        in_len
                    );
                }
                "f64"
            }
            ty::Float(f) => {
                return_error!(
                    "unsupported element type `{}` of floating-point vector `{}`",
                    f.name_str(),
                    in_ty
                );
            }
            _ => {
                return_error!("`{}` is not a floating-point type", in_ty);
            }
        };

        let llvm_name = &format!("llvm.{0}.v{1}{2}", name, in_len, ety);
        let intrinsic = bx.get_intrinsic(&llvm_name);
        let c =
            bx.call(intrinsic, &args.iter().map(|arg| arg.immediate()).collect::<Vec<_>>(), None);
        unsafe { llvm::LLVMRustSetHasUnsafeAlgebra(c) };
        Ok(c)
    }

    match name {
        sym::simd_fsqrt => {
            return simd_simple_float_intrinsic("sqrt", in_elem, in_ty, in_len, bx, span, args);
        }
        sym::simd_fsin => {
            return simd_simple_float_intrinsic("sin", in_elem, in_ty, in_len, bx, span, args);
        }
        sym::simd_fcos => {
            return simd_simple_float_intrinsic("cos", in_elem, in_ty, in_len, bx, span, args);
        }
        sym::simd_fabs => {
            return simd_simple_float_intrinsic("fabs", in_elem, in_ty, in_len, bx, span, args);
        }
        sym::simd_floor => {
            return simd_simple_float_intrinsic("floor", in_elem, in_ty, in_len, bx, span, args);
        }
        sym::simd_ceil => {
            return simd_simple_float_intrinsic("ceil", in_elem, in_ty, in_len, bx, span, args);
        }
        sym::simd_fexp => {
            return simd_simple_float_intrinsic("exp", in_elem, in_ty, in_len, bx, span, args);
        }
        sym::simd_fexp2 => {
            return simd_simple_float_intrinsic("exp2", in_elem, in_ty, in_len, bx, span, args);
        }
        sym::simd_flog10 => {
            return simd_simple_float_intrinsic("log10", in_elem, in_ty, in_len, bx, span, args);
        }
        sym::simd_flog2 => {
            return simd_simple_float_intrinsic("log2", in_elem, in_ty, in_len, bx, span, args);
        }
        sym::simd_flog => {
            return simd_simple_float_intrinsic("log", in_elem, in_ty, in_len, bx, span, args);
        }
        sym::simd_fpowi => {
            return simd_simple_float_intrinsic("powi", in_elem, in_ty, in_len, bx, span, args);
        }
        sym::simd_fpow => {
            return simd_simple_float_intrinsic("pow", in_elem, in_ty, in_len, bx, span, args);
        }
        sym::simd_fma => {
            return simd_simple_float_intrinsic("fma", in_elem, in_ty, in_len, bx, span, args);
        }
        _ => { /* fallthrough */ }
    }

    // FIXME: use:
    //  https://github.com/llvm-mirror/llvm/blob/master/include/llvm/IR/Function.h#L182
    //  https://github.com/llvm-mirror/llvm/blob/master/include/llvm/IR/Intrinsics.h#L81
    fn llvm_vector_str(elem_ty: Ty<'_>, vec_len: u64, no_pointers: usize) -> String {
        let p0s: String = "p0".repeat(no_pointers);
        match *elem_ty.kind() {
            ty::Int(v) => format!("v{}{}i{}", vec_len, p0s, v.bit_width().unwrap()),
            ty::Uint(v) => format!("v{}{}i{}", vec_len, p0s, v.bit_width().unwrap()),
            ty::Float(v) => format!("v{}{}f{}", vec_len, p0s, v.bit_width()),
            _ => unreachable!(),
        }
    }

    fn llvm_vector_ty(
        cx: &CodegenCx<'ll, '_>,
        elem_ty: Ty<'_>,
        vec_len: u64,
        mut no_pointers: usize,
    ) -> &'ll Type {
        // FIXME: use cx.layout_of(ty).llvm_type() ?
        let mut elem_ty = match *elem_ty.kind() {
            ty::Int(v) => cx.type_int_from_ty(v),
            ty::Uint(v) => cx.type_uint_from_ty(v),
            ty::Float(v) => cx.type_float_from_ty(v),
            _ => unreachable!(),
        };
        while no_pointers > 0 {
            elem_ty = cx.type_ptr_to(elem_ty);
            no_pointers -= 1;
        }
        cx.type_vector(elem_ty, vec_len)
    }

    if name == sym::simd_gather {
        // simd_gather(values: <N x T>, pointers: <N x *_ T>,
        //             mask: <N x i{M}>) -> <N x T>
        // * N: number of elements in the input vectors
        // * T: type of the element to load
        // * M: any integer width is supported, will be truncated to i1

        // All types must be simd vector types
        require_simd!(in_ty, "first");
        require_simd!(arg_tys[1], "second");
        require_simd!(arg_tys[2], "third");
        require_simd!(ret_ty, "return");

        // Of the same length:
        require!(
            in_len == arg_tys[1].simd_size(tcx),
            "expected {} argument with length {} (same as input type `{}`), \
                  found `{}` with length {}",
            "second",
            in_len,
            in_ty,
            arg_tys[1],
            arg_tys[1].simd_size(tcx)
        );
        require!(
            in_len == arg_tys[2].simd_size(tcx),
            "expected {} argument with length {} (same as input type `{}`), \
                  found `{}` with length {}",
            "third",
            in_len,
            in_ty,
            arg_tys[2],
            arg_tys[2].simd_size(tcx)
        );

        // The return type must match the first argument type
        require!(ret_ty == in_ty, "expected return type `{}`, found `{}`", in_ty, ret_ty);

        // This counts how many pointers
        fn ptr_count(t: Ty<'_>) -> usize {
            match t.kind() {
                ty::RawPtr(p) => 1 + ptr_count(p.ty),
                _ => 0,
            }
        }

        // Non-ptr type
        fn non_ptr(t: Ty<'_>) -> Ty<'_> {
            match t.kind() {
                ty::RawPtr(p) => non_ptr(p.ty),
                _ => t,
            }
        }

        // The second argument must be a simd vector with an element type that's a pointer
        // to the element type of the first argument
        let (pointer_count, underlying_ty) = match arg_tys[1].simd_type(tcx).kind() {
            ty::RawPtr(p) if p.ty == in_elem => {
                (ptr_count(arg_tys[1].simd_type(tcx)), non_ptr(arg_tys[1].simd_type(tcx)))
            }
            _ => {
                require!(
                    false,
                    "expected element type `{}` of second argument `{}` \
                                 to be a pointer to the element type `{}` of the first \
                                 argument `{}`, found `{}` != `*_ {}`",
                    arg_tys[1].simd_type(tcx),
                    arg_tys[1],
                    in_elem,
                    in_ty,
                    arg_tys[1].simd_type(tcx),
                    in_elem
                );
                unreachable!();
            }
        };
        assert!(pointer_count > 0);
        assert_eq!(pointer_count - 1, ptr_count(arg_tys[0].simd_type(tcx)));
        assert_eq!(underlying_ty, non_ptr(arg_tys[0].simd_type(tcx)));

        // The element type of the third argument must be a signed integer type of any width:
        match arg_tys[2].simd_type(tcx).kind() {
            ty::Int(_) => (),
            _ => {
                require!(
                    false,
                    "expected element type `{}` of third argument `{}` \
                                 to be a signed integer type",
                    arg_tys[2].simd_type(tcx),
                    arg_tys[2]
                );
            }
        }

        // Alignment of T, must be a constant integer value:
        let alignment_ty = bx.type_i32();
        let alignment = bx.const_i32(bx.align_of(in_elem).bytes() as i32);

        // Truncate the mask vector to a vector of i1s:
        let (mask, mask_ty) = {
            let i1 = bx.type_i1();
            let i1xn = bx.type_vector(i1, in_len);
            (bx.trunc(args[2].immediate(), i1xn), i1xn)
        };

        // Type of the vector of pointers:
        let llvm_pointer_vec_ty = llvm_vector_ty(bx, underlying_ty, in_len, pointer_count);
        let llvm_pointer_vec_str = llvm_vector_str(underlying_ty, in_len, pointer_count);

        // Type of the vector of elements:
        let llvm_elem_vec_ty = llvm_vector_ty(bx, underlying_ty, in_len, pointer_count - 1);
        let llvm_elem_vec_str = llvm_vector_str(underlying_ty, in_len, pointer_count - 1);

        let llvm_intrinsic =
            format!("llvm.masked.gather.{}.{}", llvm_elem_vec_str, llvm_pointer_vec_str);
        let f = bx.declare_cfn(
            &llvm_intrinsic,
            bx.type_func(
                &[llvm_pointer_vec_ty, alignment_ty, mask_ty, llvm_elem_vec_ty],
                llvm_elem_vec_ty,
            ),
        );
        llvm::SetUnnamedAddress(f, llvm::UnnamedAddr::No);
        let v = bx.call(f, &[args[1].immediate(), alignment, mask, args[0].immediate()], None);
        return Ok(v);
    }

    if name == sym::simd_scatter {
        // simd_scatter(values: <N x T>, pointers: <N x *mut T>,
        //             mask: <N x i{M}>) -> ()
        // * N: number of elements in the input vectors
        // * T: type of the element to load
        // * M: any integer width is supported, will be truncated to i1

        // All types must be simd vector types
        require_simd!(in_ty, "first");
        require_simd!(arg_tys[1], "second");
        require_simd!(arg_tys[2], "third");

        // Of the same length:
        require!(
            in_len == arg_tys[1].simd_size(tcx),
            "expected {} argument with length {} (same as input type `{}`), \
                  found `{}` with length {}",
            "second",
            in_len,
            in_ty,
            arg_tys[1],
            arg_tys[1].simd_size(tcx)
        );
        require!(
            in_len == arg_tys[2].simd_size(tcx),
            "expected {} argument with length {} (same as input type `{}`), \
                  found `{}` with length {}",
            "third",
            in_len,
            in_ty,
            arg_tys[2],
            arg_tys[2].simd_size(tcx)
        );

        // This counts how many pointers
        fn ptr_count(t: Ty<'_>) -> usize {
            match t.kind() {
                ty::RawPtr(p) => 1 + ptr_count(p.ty),
                _ => 0,
            }
        }

        // Non-ptr type
        fn non_ptr(t: Ty<'_>) -> Ty<'_> {
            match t.kind() {
                ty::RawPtr(p) => non_ptr(p.ty),
                _ => t,
            }
        }

        // The second argument must be a simd vector with an element type that's a pointer
        // to the element type of the first argument
        let (pointer_count, underlying_ty) = match arg_tys[1].simd_type(tcx).kind() {
            ty::RawPtr(p) if p.ty == in_elem && p.mutbl == hir::Mutability::Mut => {
                (ptr_count(arg_tys[1].simd_type(tcx)), non_ptr(arg_tys[1].simd_type(tcx)))
            }
            _ => {
                require!(
                    false,
                    "expected element type `{}` of second argument `{}` \
                                 to be a pointer to the element type `{}` of the first \
                                 argument `{}`, found `{}` != `*mut {}`",
                    arg_tys[1].simd_type(tcx),
                    arg_tys[1],
                    in_elem,
                    in_ty,
                    arg_tys[1].simd_type(tcx),
                    in_elem
                );
                unreachable!();
            }
        };
        assert!(pointer_count > 0);
        assert_eq!(pointer_count - 1, ptr_count(arg_tys[0].simd_type(tcx)));
        assert_eq!(underlying_ty, non_ptr(arg_tys[0].simd_type(tcx)));

        // The element type of the third argument must be a signed integer type of any width:
        match arg_tys[2].simd_type(tcx).kind() {
            ty::Int(_) => (),
            _ => {
                require!(
                    false,
                    "expected element type `{}` of third argument `{}` \
                                 to be a signed integer type",
                    arg_tys[2].simd_type(tcx),
                    arg_tys[2]
                );
            }
        }

        // Alignment of T, must be a constant integer value:
        let alignment_ty = bx.type_i32();
        let alignment = bx.const_i32(bx.align_of(in_elem).bytes() as i32);

        // Truncate the mask vector to a vector of i1s:
        let (mask, mask_ty) = {
            let i1 = bx.type_i1();
            let i1xn = bx.type_vector(i1, in_len);
            (bx.trunc(args[2].immediate(), i1xn), i1xn)
        };

        let ret_t = bx.type_void();

        // Type of the vector of pointers:
        let llvm_pointer_vec_ty = llvm_vector_ty(bx, underlying_ty, in_len, pointer_count);
        let llvm_pointer_vec_str = llvm_vector_str(underlying_ty, in_len, pointer_count);

        // Type of the vector of elements:
        let llvm_elem_vec_ty = llvm_vector_ty(bx, underlying_ty, in_len, pointer_count - 1);
        let llvm_elem_vec_str = llvm_vector_str(underlying_ty, in_len, pointer_count - 1);

        let llvm_intrinsic =
            format!("llvm.masked.scatter.{}.{}", llvm_elem_vec_str, llvm_pointer_vec_str);
        let f = bx.declare_cfn(
            &llvm_intrinsic,
            bx.type_func(&[llvm_elem_vec_ty, llvm_pointer_vec_ty, alignment_ty, mask_ty], ret_t),
        );
        llvm::SetUnnamedAddress(f, llvm::UnnamedAddr::No);
        let v = bx.call(f, &[args[0].immediate(), args[1].immediate(), alignment, mask], None);
        return Ok(v);
    }

    macro_rules! arith_red {
        ($name:ident : $integer_reduce:ident, $float_reduce:ident, $ordered:expr, $op:ident,
         $identity:expr) => {
            if name == sym::$name {
                require!(
                    ret_ty == in_elem,
                    "expected return type `{}` (element of input `{}`), found `{}`",
                    in_elem,
                    in_ty,
                    ret_ty
                );
                return match in_elem.kind() {
                    ty::Int(_) | ty::Uint(_) => {
                        let r = bx.$integer_reduce(args[0].immediate());
                        if $ordered {
                            // if overflow occurs, the result is the
                            // mathematical result modulo 2^n:
                            Ok(bx.$op(args[1].immediate(), r))
                        } else {
                            Ok(bx.$integer_reduce(args[0].immediate()))
                        }
                    }
                    ty::Float(f) => {
                        let acc = if $ordered {
                            // ordered arithmetic reductions take an accumulator
                            args[1].immediate()
                        } else {
                            // unordered arithmetic reductions use the identity accumulator
                            match f.bit_width() {
                                32 => bx.const_real(bx.type_f32(), $identity),
                                64 => bx.const_real(bx.type_f64(), $identity),
                                v => return_error!(
                                    r#"
unsupported {} from `{}` with element `{}` of size `{}` to `{}`"#,
                                    sym::$name,
                                    in_ty,
                                    in_elem,
                                    v,
                                    ret_ty
                                ),
                            }
                        };
                        Ok(bx.$float_reduce(acc, args[0].immediate()))
                    }
                    _ => return_error!(
                        "unsupported {} from `{}` with element `{}` to `{}`",
                        sym::$name,
                        in_ty,
                        in_elem,
                        ret_ty
                    ),
                };
            }
        };
    }

    arith_red!(simd_reduce_add_ordered: vector_reduce_add, vector_reduce_fadd, true, add, 0.0);
    arith_red!(simd_reduce_mul_ordered: vector_reduce_mul, vector_reduce_fmul, true, mul, 1.0);
    arith_red!(
        simd_reduce_add_unordered: vector_reduce_add,
        vector_reduce_fadd_fast,
        false,
        add,
        0.0
    );
    arith_red!(
        simd_reduce_mul_unordered: vector_reduce_mul,
        vector_reduce_fmul_fast,
        false,
        mul,
        1.0
    );

    macro_rules! minmax_red {
        ($name:ident: $int_red:ident, $float_red:ident) => {
            if name == sym::$name {
                require!(
                    ret_ty == in_elem,
                    "expected return type `{}` (element of input `{}`), found `{}`",
                    in_elem,
                    in_ty,
                    ret_ty
                );
                return match in_elem.kind() {
                    ty::Int(_i) => Ok(bx.$int_red(args[0].immediate(), true)),
                    ty::Uint(_u) => Ok(bx.$int_red(args[0].immediate(), false)),
                    ty::Float(_f) => Ok(bx.$float_red(args[0].immediate())),
                    _ => return_error!(
                        "unsupported {} from `{}` with element `{}` to `{}`",
                        sym::$name,
                        in_ty,
                        in_elem,
                        ret_ty
                    ),
                };
            }
        };
    }

    minmax_red!(simd_reduce_min: vector_reduce_min, vector_reduce_fmin);
    minmax_red!(simd_reduce_max: vector_reduce_max, vector_reduce_fmax);

    minmax_red!(simd_reduce_min_nanless: vector_reduce_min, vector_reduce_fmin_fast);
    minmax_red!(simd_reduce_max_nanless: vector_reduce_max, vector_reduce_fmax_fast);

    macro_rules! bitwise_red {
        ($name:ident : $red:ident, $boolean:expr) => {
            if name == sym::$name {
                let input = if !$boolean {
                    require!(
                        ret_ty == in_elem,
                        "expected return type `{}` (element of input `{}`), found `{}`",
                        in_elem,
                        in_ty,
                        ret_ty
                    );
                    args[0].immediate()
                } else {
                    match in_elem.kind() {
                        ty::Int(_) | ty::Uint(_) => {}
                        _ => return_error!(
                            "unsupported {} from `{}` with element `{}` to `{}`",
                            sym::$name,
                            in_ty,
                            in_elem,
                            ret_ty
                        ),
                    }

                    // boolean reductions operate on vectors of i1s:
                    let i1 = bx.type_i1();
                    let i1xn = bx.type_vector(i1, in_len as u64);
                    bx.trunc(args[0].immediate(), i1xn)
                };
                return match in_elem.kind() {
                    ty::Int(_) | ty::Uint(_) => {
                        let r = bx.$red(input);
                        Ok(if !$boolean { r } else { bx.zext(r, bx.type_bool()) })
                    }
                    _ => return_error!(
                        "unsupported {} from `{}` with element `{}` to `{}`",
                        sym::$name,
                        in_ty,
                        in_elem,
                        ret_ty
                    ),
                };
            }
        };
    }

    bitwise_red!(simd_reduce_and: vector_reduce_and, false);
    bitwise_red!(simd_reduce_or: vector_reduce_or, false);
    bitwise_red!(simd_reduce_xor: vector_reduce_xor, false);
    bitwise_red!(simd_reduce_all: vector_reduce_and, true);
    bitwise_red!(simd_reduce_any: vector_reduce_or, true);

    if name == sym::simd_cast {
        require_simd!(ret_ty, "return");
        let out_len = ret_ty.simd_size(tcx);
        require!(
            in_len == out_len,
            "expected return type with length {} (same as input type `{}`), \
                  found `{}` with length {}",
            in_len,
            in_ty,
            ret_ty,
            out_len
        );
        // casting cares about nominal type, not just structural type
        let out_elem = ret_ty.simd_type(tcx);

        if in_elem == out_elem {
            return Ok(args[0].immediate());
        }

        enum Style {
            Float,
            Int(/* is signed? */ bool),
            Unsupported,
        }

        let (in_style, in_width) = match in_elem.kind() {
            // vectors of pointer-sized integers should've been
            // disallowed before here, so this unwrap is safe.
            ty::Int(i) => (Style::Int(true), i.bit_width().unwrap()),
            ty::Uint(u) => (Style::Int(false), u.bit_width().unwrap()),
            ty::Float(f) => (Style::Float, f.bit_width()),
            _ => (Style::Unsupported, 0),
        };
        let (out_style, out_width) = match out_elem.kind() {
            ty::Int(i) => (Style::Int(true), i.bit_width().unwrap()),
            ty::Uint(u) => (Style::Int(false), u.bit_width().unwrap()),
            ty::Float(f) => (Style::Float, f.bit_width()),
            _ => (Style::Unsupported, 0),
        };

        match (in_style, out_style) {
            (Style::Int(in_is_signed), Style::Int(_)) => {
                return Ok(match in_width.cmp(&out_width) {
                    Ordering::Greater => bx.trunc(args[0].immediate(), llret_ty),
                    Ordering::Equal => args[0].immediate(),
                    Ordering::Less => {
                        if in_is_signed {
                            bx.sext(args[0].immediate(), llret_ty)
                        } else {
                            bx.zext(args[0].immediate(), llret_ty)
                        }
                    }
                });
            }
            (Style::Int(in_is_signed), Style::Float) => {
                return Ok(if in_is_signed {
                    bx.sitofp(args[0].immediate(), llret_ty)
                } else {
                    bx.uitofp(args[0].immediate(), llret_ty)
                });
            }
            (Style::Float, Style::Int(out_is_signed)) => {
                return Ok(if out_is_signed {
                    bx.fptosi(args[0].immediate(), llret_ty)
                } else {
                    bx.fptoui(args[0].immediate(), llret_ty)
                });
            }
            (Style::Float, Style::Float) => {
                return Ok(match in_width.cmp(&out_width) {
                    Ordering::Greater => bx.fptrunc(args[0].immediate(), llret_ty),
                    Ordering::Equal => args[0].immediate(),
                    Ordering::Less => bx.fpext(args[0].immediate(), llret_ty),
                });
            }
            _ => { /* Unsupported. Fallthrough. */ }
        }
        require!(
            false,
            "unsupported cast from `{}` with element `{}` to `{}` with element `{}`",
            in_ty,
            in_elem,
            ret_ty,
            out_elem
        );
    }
    macro_rules! arith {
        ($($name: ident: $($($p: ident),* => $call: ident),*;)*) => {
            $(if name == sym::$name {
                match in_elem.kind() {
                    $($(ty::$p(_))|* => {
                        return Ok(bx.$call(args[0].immediate(), args[1].immediate()))
                    })*
                    _ => {},
                }
                require!(false,
                         "unsupported operation on `{}` with element `{}`",
                         in_ty,
                         in_elem)
            })*
        }
    }
    arith! {
        simd_add: Uint, Int => add, Float => fadd;
        simd_sub: Uint, Int => sub, Float => fsub;
        simd_mul: Uint, Int => mul, Float => fmul;
        simd_div: Uint => udiv, Int => sdiv, Float => fdiv;
        simd_rem: Uint => urem, Int => srem, Float => frem;
        simd_shl: Uint, Int => shl;
        simd_shr: Uint => lshr, Int => ashr;
        simd_and: Uint, Int => and;
        simd_or: Uint, Int => or;
        simd_xor: Uint, Int => xor;
        simd_fmax: Float => maxnum;
        simd_fmin: Float => minnum;

    }

    if name == sym::simd_saturating_add || name == sym::simd_saturating_sub {
        let lhs = args[0].immediate();
        let rhs = args[1].immediate();
        let is_add = name == sym::simd_saturating_add;
        let ptr_bits = bx.tcx().data_layout.pointer_size.bits() as _;
        let (signed, elem_width, elem_ty) = match *in_elem.kind() {
            ty::Int(i) => (true, i.bit_width().unwrap_or(ptr_bits), bx.cx.type_int_from_ty(i)),
            ty::Uint(i) => (false, i.bit_width().unwrap_or(ptr_bits), bx.cx.type_uint_from_ty(i)),
            _ => {
                return_error!(
                    "expected element type `{}` of vector type `{}` \
                     to be a signed or unsigned integer type",
                    arg_tys[0].simd_type(tcx),
                    arg_tys[0]
                );
            }
        };
        let llvm_intrinsic = &format!(
            "llvm.{}{}.sat.v{}i{}",
            if signed { 's' } else { 'u' },
            if is_add { "add" } else { "sub" },
            in_len,
            elem_width
        );
        let vec_ty = bx.cx.type_vector(elem_ty, in_len as u64);

        let f = bx.declare_cfn(&llvm_intrinsic, bx.type_func(&[vec_ty, vec_ty], vec_ty));
        llvm::SetUnnamedAddress(f, llvm::UnnamedAddr::No);
        let v = bx.call(f, &[lhs, rhs], None);
        return Ok(v);
    }

    span_bug!(span, "unknown SIMD intrinsic");
}

// Returns the width of an int Ty, and if it's signed or not
// Returns None if the type is not an integer
// FIXME: there???s multiple of this functions, investigate using some of the already existing
// stuffs.
fn int_type_width_signed(ty: Ty<'_>, cx: &CodegenCx<'_, '_>) -> Option<(u64, bool)> {
    match ty.kind() {
        ty::Int(t) => Some((
            match t {
                ast::IntTy::Isize => u64::from(cx.tcx.sess.target.ptr_width),
                ast::IntTy::I8 => 8,
                ast::IntTy::I16 => 16,
                ast::IntTy::I32 => 32,
                ast::IntTy::I64 => 64,
                ast::IntTy::I128 => 128,
            },
            true,
        )),
        ty::Uint(t) => Some((
            match t {
                ast::UintTy::Usize => u64::from(cx.tcx.sess.target.ptr_width),
                ast::UintTy::U8 => 8,
                ast::UintTy::U16 => 16,
                ast::UintTy::U32 => 32,
                ast::UintTy::U64 => 64,
                ast::UintTy::U128 => 128,
            },
            false,
        )),
        _ => None,
    }
}

// Returns the width of a float Ty
// Returns None if the type is not a float
fn float_type_width(ty: Ty<'_>) -> Option<u64> {
    match ty.kind() {
        ty::Float(t) => Some(t.bit_width()),
        _ => None,
    }
}
