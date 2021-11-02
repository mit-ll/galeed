//! Performs various peephole optimizations.

use crate::transform::{MirPass, MirSource};
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_hir::Mutability;
use rustc_index::vec::Idx;
use rustc_middle::mir::visit::{MutVisitor, Visitor};
use rustc_middle::mir::{
    BinOp, Body, Constant, Local, Location, Operand, Place, PlaceRef, ProjectionElem, Rvalue,
};
use rustc_middle::ty::{self, TyCtxt};
use std::mem;

pub struct InstCombine;

impl<'tcx> MirPass<'tcx> for InstCombine {
    fn run_pass(&self, tcx: TyCtxt<'tcx>, _: MirSource<'tcx>, body: &mut Body<'tcx>) {
        // First, find optimization opportunities. This is done in a pre-pass to keep the MIR
        // read-only so that we can do global analyses on the MIR in the process (e.g.
        // `Place::ty()`).
        let optimizations = {
            let mut optimization_finder = OptimizationFinder::new(body, tcx);
            optimization_finder.visit_body(body);
            optimization_finder.optimizations
        };

        // Then carry out those optimizations.
        MutVisitor::visit_body(&mut InstCombineVisitor { optimizations, tcx }, body);
    }
}

pub struct InstCombineVisitor<'tcx> {
    optimizations: OptimizationList<'tcx>,
    tcx: TyCtxt<'tcx>,
}

impl<'tcx> MutVisitor<'tcx> for InstCombineVisitor<'tcx> {
    fn tcx(&self) -> TyCtxt<'tcx> {
        self.tcx
    }

    fn visit_rvalue(&mut self, rvalue: &mut Rvalue<'tcx>, location: Location) {
        if self.optimizations.and_stars.remove(&location) {
            debug!("replacing `&*`: {:?}", rvalue);
            let new_place = match rvalue {
                Rvalue::Ref(_, _, place) => {
                    if let &[ref proj_l @ .., proj_r] = place.projection.as_ref() {
                        place.projection = self.tcx().intern_place_elems(&[proj_r]);

                        Place {
                            // Replace with dummy
                            local: mem::replace(&mut place.local, Local::new(0)),
                            projection: self.tcx().intern_place_elems(proj_l),
                        }
                    } else {
                        unreachable!();
                    }
                }
                _ => bug!("Detected `&*` but didn't find `&*`!"),
            };
            *rvalue = Rvalue::Use(Operand::Copy(new_place))
        }

        if let Some(constant) = self.optimizations.arrays_lengths.remove(&location) {
            debug!("replacing `Len([_; N])`: {:?}", rvalue);
            *rvalue = Rvalue::Use(Operand::Constant(box constant));
        }

        if let Some(operand) = self.optimizations.unneeded_equality_comparison.remove(&location) {
            debug!("replacing {:?} with {:?}", rvalue, operand);
            *rvalue = Rvalue::Use(operand);
        }

        self.super_rvalue(rvalue, location)
    }
}

/// Finds optimization opportunities on the MIR.
struct OptimizationFinder<'b, 'tcx> {
    body: &'b Body<'tcx>,
    tcx: TyCtxt<'tcx>,
    optimizations: OptimizationList<'tcx>,
}

impl OptimizationFinder<'b, 'tcx> {
    fn new(body: &'b Body<'tcx>, tcx: TyCtxt<'tcx>) -> OptimizationFinder<'b, 'tcx> {
        OptimizationFinder { body, tcx, optimizations: OptimizationList::default() }
    }

    fn find_unneeded_equality_comparison(&mut self, rvalue: &Rvalue<'tcx>, location: Location) {
        // find Ne(_place, false) or Ne(false, _place)
        // or   Eq(_place, true) or Eq(true, _place)
        if let Rvalue::BinaryOp(op, l, r) = rvalue {
            let const_to_find = if *op == BinOp::Ne {
                false
            } else if *op == BinOp::Eq {
                true
            } else {
                return;
            };
            // (const, _place)
            if let Some(o) = self.find_operand_in_equality_comparison_pattern(l, r, const_to_find) {
                self.optimizations.unneeded_equality_comparison.insert(location, o.clone());
            }
            // (_place, const)
            else if let Some(o) =
                self.find_operand_in_equality_comparison_pattern(r, l, const_to_find)
            {
                self.optimizations.unneeded_equality_comparison.insert(location, o.clone());
            }
        }
    }

    fn find_operand_in_equality_comparison_pattern(
        &self,
        l: &Operand<'tcx>,
        r: &'a Operand<'tcx>,
        const_to_find: bool,
    ) -> Option<&'a Operand<'tcx>> {
        let const_ = l.constant()?;
        if const_.literal.ty == self.tcx.types.bool
            && const_.literal.val.try_to_bool() == Some(const_to_find)
        {
            if r.place().is_some() {
                return Some(r);
            }
        }

        None
    }
}

impl Visitor<'tcx> for OptimizationFinder<'b, 'tcx> {
    fn visit_rvalue(&mut self, rvalue: &Rvalue<'tcx>, location: Location) {
        if let Rvalue::Ref(_, _, place) = rvalue {
            if let PlaceRef { local, projection: &[ref proj_base @ .., ProjectionElem::Deref] } =
                place.as_ref()
            {
                // The dereferenced place must have type `&_`.
                let ty = Place::ty_from(local, proj_base, self.body, self.tcx).ty;
                if let ty::Ref(_, _, Mutability::Not) = ty.kind() {
                    self.optimizations.and_stars.insert(location);
                }
            }
        }

        if let Rvalue::Len(ref place) = *rvalue {
            let place_ty = place.ty(&self.body.local_decls, self.tcx).ty;
            if let ty::Array(_, len) = place_ty.kind() {
                let span = self.body.source_info(location).span;
                let constant = Constant { span, literal: len, user_ty: None };
                self.optimizations.arrays_lengths.insert(location, constant);
            }
        }

        self.find_unneeded_equality_comparison(rvalue, location);

        self.super_rvalue(rvalue, location)
    }
}

#[derive(Default)]
struct OptimizationList<'tcx> {
    and_stars: FxHashSet<Location>,
    arrays_lengths: FxHashMap<Location, Constant<'tcx>>,
    unneeded_equality_comparison: FxHashMap<Location, Operand<'tcx>>,
}
