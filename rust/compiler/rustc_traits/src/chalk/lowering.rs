//! Contains the logic to lower rustc types into Chalk types
//!
//! In many cases there is a 1:1 relationship between a rustc type and a Chalk type.
//! For example, a `SubstsRef` maps almost directly to a `Substitution`. In some
//! other cases, such as `Param`s, there is no Chalk type, so we have to handle
//! accordingly.
//!
//! ## `Ty` lowering
//! Much of the `Ty` lowering is 1:1 with Chalk. (Or will be eventually). A
//! helpful table for what types lower to what can be found in the
//! [Chalk book](http://rust-lang.github.io/chalk/book/types/rust_types.html).
//! The most notable difference lies with `Param`s. To convert from rustc to
//! Chalk, we eagerly and deeply convert `Param`s to placeholders (in goals) or
//! bound variables (for clause generation through functions in `db`).
//!
//! ## `Region` lowering
//! Regions are handled in rustc and Chalk is quite differently. In rustc, there
//! is a difference between "early bound" and "late bound" regions, where only
//! the late bound regions have a `DebruijnIndex`. Moreover, in Chalk all
//! regions (Lifetimes) have an associated index. In rustc, only `BrAnon`s have
//! an index, whereas `BrNamed` don't. In order to lower regions to Chalk, we
//! convert all regions into `BrAnon` late-bound regions.
//!
//! ## `Const` lowering
//! Chalk doesn't handle consts currently, so consts are currently lowered to
//! an empty tuple.
//!
//! ## Bound variable collection
//! Another difference between rustc and Chalk lies in the handling of binders.
//! Chalk requires that we store the bound parameter kinds, whereas rustc does
//! not. To lower anything wrapped in a `Binder`, we first deeply find any bound
//! variables from the current `Binder`.

use rustc_middle::traits::{ChalkEnvironmentAndGoal, ChalkRustInterner as RustInterner};
use rustc_middle::ty::fold::TypeFolder;
use rustc_middle::ty::subst::{GenericArg, GenericArgKind, SubstsRef};
use rustc_middle::ty::{
    self, Binder, BoundRegion, Region, RegionKind, Ty, TyCtxt, TyKind, TypeFoldable, TypeVisitor,
};
use rustc_span::def_id::DefId;

use std::collections::btree_map::{BTreeMap, Entry};

/// Essentially an `Into` with a `&RustInterner` parameter
crate trait LowerInto<'tcx, T> {
    /// Lower a rustc construct (e.g., `ty::TraitPredicate`) to a chalk type, consuming `self`.
    fn lower_into(self, interner: &RustInterner<'tcx>) -> T;
}

impl<'tcx> LowerInto<'tcx, chalk_ir::Substitution<RustInterner<'tcx>>> for SubstsRef<'tcx> {
    fn lower_into(
        self,
        interner: &RustInterner<'tcx>,
    ) -> chalk_ir::Substitution<RustInterner<'tcx>> {
        chalk_ir::Substitution::from_iter(interner, self.iter().map(|s| s.lower_into(interner)))
    }
}

impl<'tcx> LowerInto<'tcx, SubstsRef<'tcx>> for &chalk_ir::Substitution<RustInterner<'tcx>> {
    fn lower_into(self, interner: &RustInterner<'tcx>) -> SubstsRef<'tcx> {
        interner.tcx.mk_substs(self.iter(interner).map(|subst| subst.lower_into(interner)))
    }
}

impl<'tcx> LowerInto<'tcx, chalk_ir::AliasTy<RustInterner<'tcx>>> for ty::ProjectionTy<'tcx> {
    fn lower_into(self, interner: &RustInterner<'tcx>) -> chalk_ir::AliasTy<RustInterner<'tcx>> {
        chalk_ir::AliasTy::Projection(chalk_ir::ProjectionTy {
            associated_ty_id: chalk_ir::AssocTypeId(self.item_def_id),
            substitution: self.substs.lower_into(interner),
        })
    }
}

impl<'tcx> LowerInto<'tcx, chalk_ir::InEnvironment<chalk_ir::Goal<RustInterner<'tcx>>>>
    for ChalkEnvironmentAndGoal<'tcx>
{
    fn lower_into(
        self,
        interner: &RustInterner<'tcx>,
    ) -> chalk_ir::InEnvironment<chalk_ir::Goal<RustInterner<'tcx>>> {
        let clauses = self.environment.into_iter().map(|predicate| {
            let (predicate, binders, _named_regions) =
                collect_bound_vars(interner, interner.tcx, &predicate.bound_atom(interner.tcx));
            let consequence = match predicate {
                ty::PredicateAtom::TypeWellFormedFromEnv(ty) => {
                    chalk_ir::DomainGoal::FromEnv(chalk_ir::FromEnv::Ty(ty.lower_into(interner)))
                }
                ty::PredicateAtom::Trait(predicate, _) => chalk_ir::DomainGoal::FromEnv(
                    chalk_ir::FromEnv::Trait(predicate.trait_ref.lower_into(interner)),
                ),
                ty::PredicateAtom::RegionOutlives(predicate) => chalk_ir::DomainGoal::Holds(
                    chalk_ir::WhereClause::LifetimeOutlives(chalk_ir::LifetimeOutlives {
                        a: predicate.0.lower_into(interner),
                        b: predicate.1.lower_into(interner),
                    }),
                ),
                ty::PredicateAtom::TypeOutlives(predicate) => chalk_ir::DomainGoal::Holds(
                    chalk_ir::WhereClause::TypeOutlives(chalk_ir::TypeOutlives {
                        ty: predicate.0.lower_into(interner),
                        lifetime: predicate.1.lower_into(interner),
                    }),
                ),
                ty::PredicateAtom::Projection(predicate) => chalk_ir::DomainGoal::Holds(
                    chalk_ir::WhereClause::AliasEq(predicate.lower_into(interner)),
                ),
                ty::PredicateAtom::WellFormed(..)
                | ty::PredicateAtom::ObjectSafe(..)
                | ty::PredicateAtom::ClosureKind(..)
                | ty::PredicateAtom::Subtype(..)
                | ty::PredicateAtom::ConstEvaluatable(..)
                | ty::PredicateAtom::ConstEquate(..) => bug!("unexpected predicate {}", predicate),
            };
            let value = chalk_ir::ProgramClauseImplication {
                consequence,
                conditions: chalk_ir::Goals::empty(interner),
                priority: chalk_ir::ClausePriority::High,
                constraints: chalk_ir::Constraints::empty(interner),
            };
            chalk_ir::ProgramClauseData(chalk_ir::Binders::new(binders, value)).intern(interner)
        });

        let goal: chalk_ir::GoalData<RustInterner<'tcx>> = self.goal.lower_into(&interner);
        chalk_ir::InEnvironment {
            environment: chalk_ir::Environment {
                clauses: chalk_ir::ProgramClauses::from_iter(&interner, clauses),
            },
            goal: goal.intern(&interner),
        }
    }
}

impl<'tcx> LowerInto<'tcx, chalk_ir::GoalData<RustInterner<'tcx>>> for ty::Predicate<'tcx> {
    fn lower_into(self, interner: &RustInterner<'tcx>) -> chalk_ir::GoalData<RustInterner<'tcx>> {
        let (predicate, binders, _named_regions) =
            collect_bound_vars(interner, interner.tcx, &self.bound_atom(interner.tcx));

        let value = match predicate {
            ty::PredicateAtom::Trait(predicate, _) => {
                chalk_ir::GoalData::DomainGoal(chalk_ir::DomainGoal::Holds(
                    chalk_ir::WhereClause::Implemented(predicate.trait_ref.lower_into(interner)),
                ))
            }
            ty::PredicateAtom::RegionOutlives(predicate) => {
                chalk_ir::GoalData::DomainGoal(chalk_ir::DomainGoal::Holds(
                    chalk_ir::WhereClause::LifetimeOutlives(chalk_ir::LifetimeOutlives {
                        a: predicate.0.lower_into(interner),
                        b: predicate.1.lower_into(interner),
                    }),
                ))
            }
            ty::PredicateAtom::TypeOutlives(predicate) => {
                chalk_ir::GoalData::DomainGoal(chalk_ir::DomainGoal::Holds(
                    chalk_ir::WhereClause::TypeOutlives(chalk_ir::TypeOutlives {
                        ty: predicate.0.lower_into(interner),
                        lifetime: predicate.1.lower_into(interner),
                    }),
                ))
            }
            ty::PredicateAtom::Projection(predicate) => {
                chalk_ir::GoalData::DomainGoal(chalk_ir::DomainGoal::Holds(
                    chalk_ir::WhereClause::AliasEq(predicate.lower_into(interner)),
                ))
            }
            ty::PredicateAtom::WellFormed(arg) => match arg.unpack() {
                GenericArgKind::Type(ty) => match ty.kind() {
                    // FIXME(chalk): In Chalk, a placeholder is WellFormed if it
                    // `FromEnv`. However, when we "lower" Params, we don't update
                    // the environment.
                    ty::Placeholder(..) => {
                        chalk_ir::GoalData::All(chalk_ir::Goals::empty(interner))
                    }

                    _ => chalk_ir::GoalData::DomainGoal(chalk_ir::DomainGoal::WellFormed(
                        chalk_ir::WellFormed::Ty(ty.lower_into(interner)),
                    )),
                },
                // FIXME(chalk): handle well formed consts
                GenericArgKind::Const(..) => {
                    chalk_ir::GoalData::All(chalk_ir::Goals::empty(interner))
                }
                GenericArgKind::Lifetime(lt) => bug!("unexpect well formed predicate: {:?}", lt),
            },

            ty::PredicateAtom::ObjectSafe(t) => chalk_ir::GoalData::DomainGoal(
                chalk_ir::DomainGoal::ObjectSafe(chalk_ir::TraitId(t)),
            ),

            // FIXME(chalk): other predicates
            //
            // We can defer this, but ultimately we'll want to express
            // some of these in terms of chalk operations.
            ty::PredicateAtom::ClosureKind(..)
            | ty::PredicateAtom::Subtype(..)
            | ty::PredicateAtom::ConstEvaluatable(..)
            | ty::PredicateAtom::ConstEquate(..) => {
                chalk_ir::GoalData::All(chalk_ir::Goals::empty(interner))
            }
            ty::PredicateAtom::TypeWellFormedFromEnv(ty) => chalk_ir::GoalData::DomainGoal(
                chalk_ir::DomainGoal::FromEnv(chalk_ir::FromEnv::Ty(ty.lower_into(interner))),
            ),
        };

        chalk_ir::GoalData::Quantified(
            chalk_ir::QuantifierKind::ForAll,
            chalk_ir::Binders::new(binders, value.intern(interner)),
        )
    }
}

impl<'tcx> LowerInto<'tcx, chalk_ir::TraitRef<RustInterner<'tcx>>>
    for rustc_middle::ty::TraitRef<'tcx>
{
    fn lower_into(self, interner: &RustInterner<'tcx>) -> chalk_ir::TraitRef<RustInterner<'tcx>> {
        chalk_ir::TraitRef {
            trait_id: chalk_ir::TraitId(self.def_id),
            substitution: self.substs.lower_into(interner),
        }
    }
}

impl<'tcx> LowerInto<'tcx, chalk_ir::AliasEq<RustInterner<'tcx>>>
    for rustc_middle::ty::ProjectionPredicate<'tcx>
{
    fn lower_into(self, interner: &RustInterner<'tcx>) -> chalk_ir::AliasEq<RustInterner<'tcx>> {
        chalk_ir::AliasEq {
            ty: self.ty.lower_into(interner),
            alias: self.projection_ty.lower_into(interner),
        }
    }
}

impl<'tcx> LowerInto<'tcx, chalk_ir::Ty<RustInterner<'tcx>>> for Ty<'tcx> {
    fn lower_into(self, interner: &RustInterner<'tcx>) -> chalk_ir::Ty<RustInterner<'tcx>> {
        use chalk_ir::TyData;
        use rustc_ast as ast;
        use TyKind::*;

        let empty = || chalk_ir::Substitution::empty(interner);
        let struct_ty =
            |def_id| chalk_ir::TypeName::Adt(chalk_ir::AdtId(interner.tcx.adt_def(def_id)));
        let apply = |name, substitution| {
            TyData::Apply(chalk_ir::ApplicationTy { name, substitution }).intern(interner)
        };
        let int = |i| apply(chalk_ir::TypeName::Scalar(chalk_ir::Scalar::Int(i)), empty());
        let uint = |i| apply(chalk_ir::TypeName::Scalar(chalk_ir::Scalar::Uint(i)), empty());
        let float = |f| apply(chalk_ir::TypeName::Scalar(chalk_ir::Scalar::Float(f)), empty());

        match *self.kind() {
            Bool => apply(chalk_ir::TypeName::Scalar(chalk_ir::Scalar::Bool), empty()),
            Char => apply(chalk_ir::TypeName::Scalar(chalk_ir::Scalar::Char), empty()),
            Int(ty) => match ty {
                ast::IntTy::Isize => int(chalk_ir::IntTy::Isize),
                ast::IntTy::I8 => int(chalk_ir::IntTy::I8),
                ast::IntTy::I16 => int(chalk_ir::IntTy::I16),
                ast::IntTy::I32 => int(chalk_ir::IntTy::I32),
                ast::IntTy::I64 => int(chalk_ir::IntTy::I64),
                ast::IntTy::I128 => int(chalk_ir::IntTy::I128),
            },
            Uint(ty) => match ty {
                ast::UintTy::Usize => uint(chalk_ir::UintTy::Usize),
                ast::UintTy::U8 => uint(chalk_ir::UintTy::U8),
                ast::UintTy::U16 => uint(chalk_ir::UintTy::U16),
                ast::UintTy::U32 => uint(chalk_ir::UintTy::U32),
                ast::UintTy::U64 => uint(chalk_ir::UintTy::U64),
                ast::UintTy::U128 => uint(chalk_ir::UintTy::U128),
            },
            Float(ty) => match ty {
                ast::FloatTy::F32 => float(chalk_ir::FloatTy::F32),
                ast::FloatTy::F64 => float(chalk_ir::FloatTy::F64),
            },
            Adt(def, substs) => apply(struct_ty(def.did), substs.lower_into(interner)),
            // FIXME(chalk): lower Foreign
            Foreign(def_id) => apply(chalk_ir::TypeName::FnDef(chalk_ir::FnDefId(def_id)), empty()),
            Str => apply(chalk_ir::TypeName::Str, empty()),
            Array(ty, len) => {
                let value = match len.val {
                    ty::ConstKind::Value(val) => {
                        chalk_ir::ConstValue::Concrete(chalk_ir::ConcreteConst { interned: val })
                    }
                    ty::ConstKind::Bound(db, bound) => {
                        chalk_ir::ConstValue::BoundVar(chalk_ir::BoundVar::new(
                            chalk_ir::DebruijnIndex::new(db.as_u32()),
                            bound.index(),
                        ))
                    }
                    _ => unimplemented!("Const not implemented. {:?}", len.val),
                };
                apply(
                    chalk_ir::TypeName::Array,
                    chalk_ir::Substitution::from_iter(
                        interner,
                        &[
                            chalk_ir::GenericArgData::Ty(ty.lower_into(interner)).intern(interner),
                            chalk_ir::GenericArgData::Const(
                                chalk_ir::ConstData { ty: len.ty.lower_into(interner), value }
                                    .intern(interner),
                            )
                            .intern(interner),
                        ],
                    ),
                )
            }
            Slice(ty) => apply(
                chalk_ir::TypeName::Slice,
                chalk_ir::Substitution::from1(
                    interner,
                    chalk_ir::GenericArgData::Ty(ty.lower_into(interner)).intern(interner),
                ),
            ),
            RawPtr(ptr) => {
                let name = match ptr.mutbl {
                    ast::Mutability::Mut => chalk_ir::TypeName::Raw(chalk_ir::Mutability::Mut),
                    ast::Mutability::Not => chalk_ir::TypeName::Raw(chalk_ir::Mutability::Not),
                };
                apply(name, chalk_ir::Substitution::from1(interner, ptr.ty.lower_into(interner)))
            }
            Ref(region, ty, mutability) => {
                let name = match mutability {
                    ast::Mutability::Mut => chalk_ir::TypeName::Ref(chalk_ir::Mutability::Mut),
                    ast::Mutability::Not => chalk_ir::TypeName::Ref(chalk_ir::Mutability::Not),
                };
                apply(
                    name,
                    chalk_ir::Substitution::from_iter(
                        interner,
                        &[
                            chalk_ir::GenericArgData::Lifetime(region.lower_into(interner))
                                .intern(interner),
                            chalk_ir::GenericArgData::Ty(ty.lower_into(interner)).intern(interner),
                        ],
                    ),
                )
            }
            FnDef(def_id, substs) => apply(
                chalk_ir::TypeName::FnDef(chalk_ir::FnDefId(def_id)),
                substs.lower_into(interner),
            ),
            FnPtr(sig) => {
                let (inputs_and_outputs, binders, _named_regions) =
                    collect_bound_vars(interner, interner.tcx, &sig.inputs_and_output());
                TyData::Function(chalk_ir::FnPointer {
                    num_binders: binders.len(interner),
                    substitution: chalk_ir::Substitution::from_iter(
                        interner,
                        inputs_and_outputs.iter().map(|ty| {
                            chalk_ir::GenericArgData::Ty(ty.lower_into(interner)).intern(interner)
                        }),
                    ),
                    abi: sig.abi(),
                    safety: match sig.unsafety() {
                        rustc_hir::Unsafety::Normal => chalk_ir::Safety::Safe,
                        rustc_hir::Unsafety::Unsafe => chalk_ir::Safety::Unsafe,
                    },
                    variadic: sig.c_variadic(),
                })
                .intern(interner)
            }
            Dynamic(predicates, region) => TyData::Dyn(chalk_ir::DynTy {
                bounds: predicates.lower_into(interner),
                lifetime: region.lower_into(interner),
            })
            .intern(interner),
            Closure(def_id, substs) => apply(
                chalk_ir::TypeName::Closure(chalk_ir::ClosureId(def_id)),
                substs.lower_into(interner),
            ),
            Generator(_def_id, _substs, _) => unimplemented!(),
            GeneratorWitness(_) => unimplemented!(),
            Never => apply(chalk_ir::TypeName::Never, empty()),
            Tuple(substs) => {
                apply(chalk_ir::TypeName::Tuple(substs.len()), substs.lower_into(interner))
            }
            Projection(proj) => TyData::Alias(proj.lower_into(interner)).intern(interner),
            Opaque(def_id, substs) => {
                TyData::Alias(chalk_ir::AliasTy::Opaque(chalk_ir::OpaqueTy {
                    opaque_ty_id: chalk_ir::OpaqueTyId(def_id),
                    substitution: substs.lower_into(interner),
                }))
                .intern(interner)
            }
            // This should have been done eagerly prior to this, and all Params
            // should have been substituted to placeholders
            Param(_) => panic!("Lowering Param when not expected."),
            Bound(db, bound) => TyData::BoundVar(chalk_ir::BoundVar::new(
                chalk_ir::DebruijnIndex::new(db.as_u32()),
                bound.var.index(),
            ))
            .intern(interner),
            Placeholder(_placeholder) => TyData::Placeholder(chalk_ir::PlaceholderIndex {
                ui: chalk_ir::UniverseIndex { counter: _placeholder.universe.as_usize() },
                idx: _placeholder.name.as_usize(),
            })
            .intern(interner),
            Infer(_infer) => unimplemented!(),
            Error(_) => apply(chalk_ir::TypeName::Error, empty()),
        }
    }
}

impl<'tcx> LowerInto<'tcx, Ty<'tcx>> for &chalk_ir::Ty<RustInterner<'tcx>> {
    fn lower_into(self, interner: &RustInterner<'tcx>) -> Ty<'tcx> {
        use chalk_ir::TyData;
        use rustc_ast::ast;

        let kind = match self.data(interner) {
            TyData::Apply(application_ty) => match application_ty.name {
                chalk_ir::TypeName::Adt(struct_id) => {
                    ty::Adt(struct_id.0, application_ty.substitution.lower_into(interner))
                }
                chalk_ir::TypeName::Scalar(scalar) => match scalar {
                    chalk_ir::Scalar::Bool => ty::Bool,
                    chalk_ir::Scalar::Char => ty::Char,
                    chalk_ir::Scalar::Int(int_ty) => match int_ty {
                        chalk_ir::IntTy::Isize => ty::Int(ast::IntTy::Isize),
                        chalk_ir::IntTy::I8 => ty::Int(ast::IntTy::I8),
                        chalk_ir::IntTy::I16 => ty::Int(ast::IntTy::I16),
                        chalk_ir::IntTy::I32 => ty::Int(ast::IntTy::I32),
                        chalk_ir::IntTy::I64 => ty::Int(ast::IntTy::I64),
                        chalk_ir::IntTy::I128 => ty::Int(ast::IntTy::I128),
                    },
                    chalk_ir::Scalar::Uint(int_ty) => match int_ty {
                        chalk_ir::UintTy::Usize => ty::Uint(ast::UintTy::Usize),
                        chalk_ir::UintTy::U8 => ty::Uint(ast::UintTy::U8),
                        chalk_ir::UintTy::U16 => ty::Uint(ast::UintTy::U16),
                        chalk_ir::UintTy::U32 => ty::Uint(ast::UintTy::U32),
                        chalk_ir::UintTy::U64 => ty::Uint(ast::UintTy::U64),
                        chalk_ir::UintTy::U128 => ty::Uint(ast::UintTy::U128),
                    },
                    chalk_ir::Scalar::Float(float_ty) => match float_ty {
                        chalk_ir::FloatTy::F32 => ty::Float(ast::FloatTy::F32),
                        chalk_ir::FloatTy::F64 => ty::Float(ast::FloatTy::F64),
                    },
                },
                chalk_ir::TypeName::Array => unimplemented!(),
                chalk_ir::TypeName::FnDef(id) => {
                    ty::FnDef(id.0, application_ty.substitution.lower_into(interner))
                }
                chalk_ir::TypeName::Closure(closure) => {
                    ty::Closure(closure.0, application_ty.substitution.lower_into(interner))
                }
                chalk_ir::TypeName::Never => ty::Never,
                chalk_ir::TypeName::Tuple(_size) => {
                    ty::Tuple(application_ty.substitution.lower_into(interner))
                }
                chalk_ir::TypeName::Slice => ty::Slice(
                    application_ty.substitution.as_slice(interner)[0]
                        .ty(interner)
                        .unwrap()
                        .lower_into(interner),
                ),
                chalk_ir::TypeName::Raw(mutbl) => ty::RawPtr(ty::TypeAndMut {
                    ty: application_ty.substitution.as_slice(interner)[0]
                        .ty(interner)
                        .unwrap()
                        .lower_into(interner),
                    mutbl: match mutbl {
                        chalk_ir::Mutability::Mut => ast::Mutability::Mut,
                        chalk_ir::Mutability::Not => ast::Mutability::Not,
                    },
                }),
                chalk_ir::TypeName::Ref(mutbl) => ty::Ref(
                    application_ty.substitution.as_slice(interner)[0]
                        .lifetime(interner)
                        .unwrap()
                        .lower_into(interner),
                    application_ty.substitution.as_slice(interner)[1]
                        .ty(interner)
                        .unwrap()
                        .lower_into(interner),
                    match mutbl {
                        chalk_ir::Mutability::Mut => ast::Mutability::Mut,
                        chalk_ir::Mutability::Not => ast::Mutability::Not,
                    },
                ),
                chalk_ir::TypeName::Str => ty::Str,
                chalk_ir::TypeName::OpaqueType(opaque_ty) => {
                    ty::Opaque(opaque_ty.0, application_ty.substitution.lower_into(interner))
                }
                chalk_ir::TypeName::AssociatedType(assoc_ty) => ty::Projection(ty::ProjectionTy {
                    substs: application_ty.substitution.lower_into(interner),
                    item_def_id: assoc_ty.0,
                }),
                chalk_ir::TypeName::Error => unimplemented!(),
            },
            TyData::Placeholder(placeholder) => ty::Placeholder(ty::Placeholder {
                universe: ty::UniverseIndex::from_usize(placeholder.ui.counter),
                name: ty::BoundVar::from_usize(placeholder.idx),
            }),
            TyData::Alias(_alias_ty) => unimplemented!(),
            TyData::Function(_quantified_ty) => unimplemented!(),
            TyData::BoundVar(_bound) => ty::Bound(
                ty::DebruijnIndex::from_usize(_bound.debruijn.depth() as usize),
                ty::BoundTy {
                    var: ty::BoundVar::from_usize(_bound.index),
                    kind: ty::BoundTyKind::Anon,
                },
            ),
            TyData::InferenceVar(_, _) => unimplemented!(),
            TyData::Dyn(_) => unimplemented!(),
        };
        interner.tcx.mk_ty(kind)
    }
}

impl<'tcx> LowerInto<'tcx, chalk_ir::Lifetime<RustInterner<'tcx>>> for Region<'tcx> {
    fn lower_into(self, interner: &RustInterner<'tcx>) -> chalk_ir::Lifetime<RustInterner<'tcx>> {
        use rustc_middle::ty::RegionKind::*;

        match self {
            ReEarlyBound(_) => {
                panic!("Should have already been substituted.");
            }
            ReLateBound(db, br) => match br {
                ty::BoundRegion::BrAnon(var) => {
                    chalk_ir::LifetimeData::BoundVar(chalk_ir::BoundVar::new(
                        chalk_ir::DebruijnIndex::new(db.as_u32()),
                        *var as usize,
                    ))
                    .intern(interner)
                }
                ty::BoundRegion::BrNamed(_def_id, _name) => unimplemented!(),
                ty::BrEnv => unimplemented!(),
            },
            ReFree(_) => unimplemented!(),
            // FIXME(chalk): need to handle ReStatic
            ReStatic => unimplemented!(),
            ReVar(_) => unimplemented!(),
            RePlaceholder(placeholder_region) => {
                chalk_ir::LifetimeData::Placeholder(chalk_ir::PlaceholderIndex {
                    ui: chalk_ir::UniverseIndex { counter: placeholder_region.universe.index() },
                    idx: 0,
                })
                .intern(interner)
            }
            ReEmpty(_) => unimplemented!(),
            // FIXME(chalk): need to handle ReErased
            ReErased => unimplemented!(),
        }
    }
}

impl<'tcx> LowerInto<'tcx, Region<'tcx>> for &chalk_ir::Lifetime<RustInterner<'tcx>> {
    fn lower_into(self, interner: &RustInterner<'tcx>) -> Region<'tcx> {
        let kind = match self.data(interner) {
            chalk_ir::LifetimeData::BoundVar(var) => ty::RegionKind::ReLateBound(
                ty::DebruijnIndex::from_u32(var.debruijn.depth()),
                ty::BoundRegion::BrAnon(var.index as u32),
            ),
            chalk_ir::LifetimeData::InferenceVar(_var) => unimplemented!(),
            chalk_ir::LifetimeData::Placeholder(p) => {
                ty::RegionKind::RePlaceholder(ty::Placeholder {
                    universe: ty::UniverseIndex::from_usize(p.ui.counter),
                    name: ty::BoundRegion::BrAnon(p.idx as u32),
                })
            }
            chalk_ir::LifetimeData::Phantom(_, _) => unimplemented!(),
        };
        interner.tcx.mk_region(kind)
    }
}

impl<'tcx> LowerInto<'tcx, chalk_ir::Const<RustInterner<'tcx>>> for ty::Const<'tcx> {
    fn lower_into(self, interner: &RustInterner<'tcx>) -> chalk_ir::Const<RustInterner<'tcx>> {
        let ty = self.ty.lower_into(interner);
        let value = match self.val {
            ty::ConstKind::Value(val) => {
                chalk_ir::ConstValue::Concrete(chalk_ir::ConcreteConst { interned: val })
            }
            ty::ConstKind::Bound(db, bound) => chalk_ir::ConstValue::BoundVar(
                chalk_ir::BoundVar::new(chalk_ir::DebruijnIndex::new(db.as_u32()), bound.index()),
            ),
            _ => unimplemented!("Const not implemented. {:?}", self),
        };
        chalk_ir::ConstData { ty, value }.intern(interner)
    }
}

impl<'tcx> LowerInto<'tcx, ty::Const<'tcx>> for &chalk_ir::Const<RustInterner<'tcx>> {
    fn lower_into(self, interner: &RustInterner<'tcx>) -> ty::Const<'tcx> {
        let data = self.data(interner);
        let ty = data.ty.lower_into(interner);
        let val = match data.value {
            chalk_ir::ConstValue::BoundVar(var) => ty::ConstKind::Bound(
                ty::DebruijnIndex::from_u32(var.debruijn.depth()),
                ty::BoundVar::from_u32(var.index as u32),
            ),
            chalk_ir::ConstValue::InferenceVar(_var) => unimplemented!(),
            chalk_ir::ConstValue::Placeholder(_p) => unimplemented!(),
            chalk_ir::ConstValue::Concrete(c) => ty::ConstKind::Value(c.interned),
        };
        ty::Const { ty, val }
    }
}

impl<'tcx> LowerInto<'tcx, chalk_ir::GenericArg<RustInterner<'tcx>>> for GenericArg<'tcx> {
    fn lower_into(self, interner: &RustInterner<'tcx>) -> chalk_ir::GenericArg<RustInterner<'tcx>> {
        match self.unpack() {
            ty::subst::GenericArgKind::Type(ty) => {
                chalk_ir::GenericArgData::Ty(ty.lower_into(interner))
            }
            ty::subst::GenericArgKind::Lifetime(lifetime) => {
                chalk_ir::GenericArgData::Lifetime(lifetime.lower_into(interner))
            }
            ty::subst::GenericArgKind::Const(c) => {
                chalk_ir::GenericArgData::Const(c.lower_into(interner))
            }
        }
        .intern(interner)
    }
}

impl<'tcx> LowerInto<'tcx, ty::subst::GenericArg<'tcx>>
    for &chalk_ir::GenericArg<RustInterner<'tcx>>
{
    fn lower_into(self, interner: &RustInterner<'tcx>) -> ty::subst::GenericArg<'tcx> {
        match self.data(interner) {
            chalk_ir::GenericArgData::Ty(ty) => {
                let t: Ty<'tcx> = ty.lower_into(interner);
                t.into()
            }
            chalk_ir::GenericArgData::Lifetime(lifetime) => {
                let r: Region<'tcx> = lifetime.lower_into(interner);
                r.into()
            }
            chalk_ir::GenericArgData::Const(c) => {
                let c: ty::Const<'tcx> = c.lower_into(interner);
                interner.tcx.mk_const(c).into()
            }
        }
    }
}

// We lower into an Option here since there are some predicates which Chalk
// doesn't have a representation for yet (as a `WhereClause`), but are so common
// that we just are accepting the unsoundness for now. The `Option` will
// eventually be removed.
impl<'tcx> LowerInto<'tcx, Option<chalk_ir::QuantifiedWhereClause<RustInterner<'tcx>>>>
    for ty::Predicate<'tcx>
{
    fn lower_into(
        self,
        interner: &RustInterner<'tcx>,
    ) -> Option<chalk_ir::QuantifiedWhereClause<RustInterner<'tcx>>> {
        let (predicate, binders, _named_regions) =
            collect_bound_vars(interner, interner.tcx, &self.bound_atom(interner.tcx));
        let value = match predicate {
            ty::PredicateAtom::Trait(predicate, _) => {
                Some(chalk_ir::WhereClause::Implemented(predicate.trait_ref.lower_into(interner)))
            }
            ty::PredicateAtom::RegionOutlives(predicate) => {
                Some(chalk_ir::WhereClause::LifetimeOutlives(chalk_ir::LifetimeOutlives {
                    a: predicate.0.lower_into(interner),
                    b: predicate.1.lower_into(interner),
                }))
            }
            ty::PredicateAtom::TypeOutlives(predicate) => {
                Some(chalk_ir::WhereClause::TypeOutlives(chalk_ir::TypeOutlives {
                    ty: predicate.0.lower_into(interner),
                    lifetime: predicate.1.lower_into(interner),
                }))
            }
            ty::PredicateAtom::Projection(predicate) => {
                Some(chalk_ir::WhereClause::AliasEq(predicate.lower_into(interner)))
            }
            ty::PredicateAtom::WellFormed(_ty) => None,

            ty::PredicateAtom::ObjectSafe(..)
            | ty::PredicateAtom::ClosureKind(..)
            | ty::PredicateAtom::Subtype(..)
            | ty::PredicateAtom::ConstEvaluatable(..)
            | ty::PredicateAtom::ConstEquate(..)
            | ty::PredicateAtom::TypeWellFormedFromEnv(..) => {
                bug!("unexpected predicate {}", &self)
            }
        };
        value.map(|value| chalk_ir::Binders::new(binders, value))
    }
}

impl<'tcx> LowerInto<'tcx, chalk_ir::Binders<chalk_ir::QuantifiedWhereClauses<RustInterner<'tcx>>>>
    for Binder<&'tcx ty::List<ty::ExistentialPredicate<'tcx>>>
{
    fn lower_into(
        self,
        interner: &RustInterner<'tcx>,
    ) -> chalk_ir::Binders<chalk_ir::QuantifiedWhereClauses<RustInterner<'tcx>>> {
        let (predicates, binders, _named_regions) =
            collect_bound_vars(interner, interner.tcx, &self);
        let self_ty = interner.tcx.mk_ty(ty::Bound(
            // This is going to be wrapped in a binder
            ty::DebruijnIndex::from_usize(1),
            ty::BoundTy { var: ty::BoundVar::from_usize(0), kind: ty::BoundTyKind::Anon },
        ));
        let where_clauses = predicates.into_iter().map(|predicate| match predicate {
            ty::ExistentialPredicate::Trait(ty::ExistentialTraitRef { def_id, substs }) => {
                chalk_ir::Binders::new(
                    chalk_ir::VariableKinds::empty(interner),
                    chalk_ir::WhereClause::Implemented(chalk_ir::TraitRef {
                        trait_id: chalk_ir::TraitId(def_id),
                        substitution: interner
                            .tcx
                            .mk_substs_trait(self_ty, substs)
                            .lower_into(interner),
                    }),
                )
            }
            ty::ExistentialPredicate::Projection(_predicate) => unimplemented!(),
            ty::ExistentialPredicate::AutoTrait(def_id) => chalk_ir::Binders::new(
                chalk_ir::VariableKinds::empty(interner),
                chalk_ir::WhereClause::Implemented(chalk_ir::TraitRef {
                    trait_id: chalk_ir::TraitId(def_id),
                    substitution: interner.tcx.mk_substs_trait(self_ty, &[]).lower_into(interner),
                }),
            ),
        });
        let value = chalk_ir::QuantifiedWhereClauses::from_iter(interner, where_clauses);
        chalk_ir::Binders::new(binders, value)
    }
}

/// To collect bound vars, we have to do two passes. In the first pass, we
/// collect all `BoundRegion`s and `ty::Bound`s. In the second pass, we then
/// replace `BrNamed` into `BrAnon`. The two separate passes are important,
/// since we can only replace `BrNamed` with `BrAnon`s with indices *after* all
/// "real" `BrAnon`s.
///
/// It's important to note that because of prior substitution, we may have
/// late-bound regions, even outside of fn contexts, since this is the best way
/// to prep types for chalk lowering.
crate fn collect_bound_vars<'a, 'tcx, T: TypeFoldable<'tcx>>(
    interner: &RustInterner<'tcx>,
    tcx: TyCtxt<'tcx>,
    ty: &'a Binder<T>,
) -> (T, chalk_ir::VariableKinds<RustInterner<'tcx>>, BTreeMap<DefId, u32>) {
    let mut bound_vars_collector = BoundVarsCollector::new();
    ty.as_ref().skip_binder().visit_with(&mut bound_vars_collector);
    let mut parameters = bound_vars_collector.parameters;
    let named_parameters: BTreeMap<DefId, u32> = bound_vars_collector
        .named_parameters
        .into_iter()
        .enumerate()
        .map(|(i, def_id)| (def_id, (i + parameters.len()) as u32))
        .collect();

    let mut bound_var_substitutor = NamedBoundVarSubstitutor::new(tcx, &named_parameters);
    let new_ty = ty.as_ref().skip_binder().fold_with(&mut bound_var_substitutor);

    for var in named_parameters.values() {
        parameters.insert(*var, chalk_ir::VariableKind::Lifetime);
    }

    (0..parameters.len()).for_each(|i| {
        parameters
            .get(&(i as u32))
            .or_else(|| bug!("Skipped bound var index: ty={:?}, parameters={:?}", ty, parameters));
    });

    let binders =
        chalk_ir::VariableKinds::from_iter(interner, parameters.into_iter().map(|(_, v)| v));

    (new_ty, binders, named_parameters)
}

crate struct BoundVarsCollector<'tcx> {
    binder_index: ty::DebruijnIndex,
    crate parameters: BTreeMap<u32, chalk_ir::VariableKind<RustInterner<'tcx>>>,
    crate named_parameters: Vec<DefId>,
}

impl<'tcx> BoundVarsCollector<'tcx> {
    crate fn new() -> Self {
        BoundVarsCollector {
            binder_index: ty::INNERMOST,
            parameters: BTreeMap::new(),
            named_parameters: vec![],
        }
    }
}

impl<'tcx> TypeVisitor<'tcx> for BoundVarsCollector<'tcx> {
    fn visit_binder<T: TypeFoldable<'tcx>>(&mut self, t: &Binder<T>) -> bool {
        self.binder_index.shift_in(1);
        let result = t.super_visit_with(self);
        self.binder_index.shift_out(1);
        result
    }

    fn visit_ty(&mut self, t: Ty<'tcx>) -> bool {
        match *t.kind() {
            ty::Bound(debruijn, bound_ty) if debruijn == self.binder_index => {
                match self.parameters.entry(bound_ty.var.as_u32()) {
                    Entry::Vacant(entry) => {
                        entry.insert(chalk_ir::VariableKind::Ty(chalk_ir::TyKind::General));
                    }
                    Entry::Occupied(entry) => match entry.get() {
                        chalk_ir::VariableKind::Ty(_) => {}
                        _ => panic!(),
                    },
                }
            }

            _ => (),
        };

        t.super_visit_with(self)
    }

    fn visit_region(&mut self, r: Region<'tcx>) -> bool {
        match r {
            ty::ReLateBound(index, br) if *index == self.binder_index => match br {
                ty::BoundRegion::BrNamed(def_id, _name) => {
                    if self.named_parameters.iter().find(|d| *d == def_id).is_none() {
                        self.named_parameters.push(*def_id);
                    }
                }

                ty::BoundRegion::BrAnon(var) => match self.parameters.entry(*var) {
                    Entry::Vacant(entry) => {
                        entry.insert(chalk_ir::VariableKind::Lifetime);
                    }
                    Entry::Occupied(entry) => match entry.get() {
                        chalk_ir::VariableKind::Lifetime => {}
                        _ => panic!(),
                    },
                },

                ty::BrEnv => unimplemented!(),
            },

            ty::ReEarlyBound(_re) => {
                // FIXME(chalk): jackh726 - I think we should always have already
                // substituted away `ReEarlyBound`s for `ReLateBound`s, but need to confirm.
                unimplemented!();
            }

            _ => (),
        };

        r.super_visit_with(self)
    }
}

/// This is used to replace `BoundRegion::BrNamed` with `BoundRegion::BrAnon`.
/// Note: we assume that we will always have room for more bound vars. (i.e. we
/// won't ever hit the `u32` limit in `BrAnon`s).
struct NamedBoundVarSubstitutor<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    binder_index: ty::DebruijnIndex,
    named_parameters: &'a BTreeMap<DefId, u32>,
}

impl<'a, 'tcx> NamedBoundVarSubstitutor<'a, 'tcx> {
    fn new(tcx: TyCtxt<'tcx>, named_parameters: &'a BTreeMap<DefId, u32>) -> Self {
        NamedBoundVarSubstitutor { tcx, binder_index: ty::INNERMOST, named_parameters }
    }
}

impl<'a, 'tcx> TypeFolder<'tcx> for NamedBoundVarSubstitutor<'a, 'tcx> {
    fn tcx<'b>(&'b self) -> TyCtxt<'tcx> {
        self.tcx
    }

    fn fold_binder<T: TypeFoldable<'tcx>>(&mut self, t: &Binder<T>) -> Binder<T> {
        self.binder_index.shift_in(1);
        let result = t.super_fold_with(self);
        self.binder_index.shift_out(1);
        result
    }

    fn fold_region(&mut self, r: Region<'tcx>) -> Region<'tcx> {
        match r {
            ty::ReLateBound(index, br) if *index == self.binder_index => match br {
                ty::BoundRegion::BrNamed(def_id, _name) => {
                    match self.named_parameters.get(def_id) {
                        Some(idx) => {
                            return self.tcx.mk_region(RegionKind::ReLateBound(
                                *index,
                                BoundRegion::BrAnon(*idx),
                            ));
                        }
                        None => panic!("Missing `BrNamed`."),
                    }
                }
                ty::BrEnv => unimplemented!(),
                ty::BoundRegion::BrAnon(_) => {}
            },
            _ => (),
        };

        r.super_fold_with(self)
    }
}

/// Used to substitute `Param`s with placeholders. We do this since Chalk
/// have a notion of `Param`s.
crate struct ParamsSubstitutor<'tcx> {
    tcx: TyCtxt<'tcx>,
    binder_index: ty::DebruijnIndex,
    list: Vec<rustc_middle::ty::ParamTy>,
    next_ty_placeholder: usize,
    crate params: rustc_data_structures::fx::FxHashMap<usize, rustc_middle::ty::ParamTy>,
    crate named_regions: BTreeMap<DefId, u32>,
}

impl<'tcx> ParamsSubstitutor<'tcx> {
    crate fn new(tcx: TyCtxt<'tcx>, next_ty_placeholder: usize) -> Self {
        ParamsSubstitutor {
            tcx,
            binder_index: ty::INNERMOST,
            list: vec![],
            next_ty_placeholder,
            params: rustc_data_structures::fx::FxHashMap::default(),
            named_regions: BTreeMap::default(),
        }
    }
}

impl<'tcx> TypeFolder<'tcx> for ParamsSubstitutor<'tcx> {
    fn tcx<'b>(&'b self) -> TyCtxt<'tcx> {
        self.tcx
    }

    fn fold_binder<T: TypeFoldable<'tcx>>(&mut self, t: &Binder<T>) -> Binder<T> {
        self.binder_index.shift_in(1);
        let result = t.super_fold_with(self);
        self.binder_index.shift_out(1);
        result
    }

    fn fold_ty(&mut self, t: Ty<'tcx>) -> Ty<'tcx> {
        match *t.kind() {
            // FIXME(chalk): currently we convert params to placeholders starting at
            // index `0`. To support placeholders, we'll actually need to do a
            // first pass to collect placeholders. Then we can insert params after.
            ty::Placeholder(_) => unimplemented!(),
            ty::Param(param) => match self.list.iter().position(|r| r == &param) {
                Some(idx) => self.tcx.mk_ty(ty::Placeholder(ty::PlaceholderType {
                    universe: ty::UniverseIndex::from_usize(0),
                    name: ty::BoundVar::from_usize(idx),
                })),
                None => {
                    self.list.push(param);
                    let idx = self.list.len() - 1 + self.next_ty_placeholder;
                    self.params.insert(idx, param);
                    self.tcx.mk_ty(ty::Placeholder(ty::PlaceholderType {
                        universe: ty::UniverseIndex::from_usize(0),
                        name: ty::BoundVar::from_usize(idx),
                    }))
                }
            },

            _ => t.super_fold_with(self),
        }
    }

    fn fold_region(&mut self, r: Region<'tcx>) -> Region<'tcx> {
        match r {
            // FIXME(chalk) - jackh726 - this currently isn't hit in any tests.
            // This covers any region variables in a goal, right?
            ty::ReEarlyBound(_re) => match self.named_regions.get(&_re.def_id) {
                Some(idx) => self.tcx.mk_region(RegionKind::ReLateBound(
                    self.binder_index,
                    BoundRegion::BrAnon(*idx),
                )),
                None => {
                    let idx = self.named_regions.len() as u32;
                    self.named_regions.insert(_re.def_id, idx);
                    self.tcx.mk_region(RegionKind::ReLateBound(
                        self.binder_index,
                        BoundRegion::BrAnon(idx),
                    ))
                }
            },

            _ => r.super_fold_with(self),
        }
    }
}

/// Used to collect `Placeholder`s.
crate struct PlaceholdersCollector {
    universe_index: ty::UniverseIndex,
    crate next_ty_placeholder: usize,
    crate next_anon_region_placeholder: u32,
}

impl PlaceholdersCollector {
    crate fn new() -> Self {
        PlaceholdersCollector {
            universe_index: ty::UniverseIndex::ROOT,
            next_ty_placeholder: 0,
            next_anon_region_placeholder: 0,
        }
    }
}

impl<'tcx> TypeVisitor<'tcx> for PlaceholdersCollector {
    fn visit_ty(&mut self, t: Ty<'tcx>) -> bool {
        match t.kind() {
            ty::Placeholder(p) if p.universe == self.universe_index => {
                self.next_ty_placeholder = self.next_ty_placeholder.max(p.name.as_usize() + 1);
            }

            _ => (),
        };

        t.super_visit_with(self)
    }

    fn visit_region(&mut self, r: Region<'tcx>) -> bool {
        match r {
            ty::RePlaceholder(p) if p.universe == self.universe_index => {
                if let ty::BoundRegion::BrAnon(anon) = p.name {
                    self.next_anon_region_placeholder = self.next_anon_region_placeholder.max(anon);
                }
            }

            _ => (),
        };

        r.super_visit_with(self)
    }
}

/// Used to substitute specific `Regions`s with placeholders.
crate struct RegionsSubstitutor<'tcx> {
    tcx: TyCtxt<'tcx>,
    restatic_placeholder: ty::Region<'tcx>,
    reempty_placeholder: ty::Region<'tcx>,
}

impl<'tcx> RegionsSubstitutor<'tcx> {
    crate fn new(
        tcx: TyCtxt<'tcx>,
        restatic_placeholder: ty::Region<'tcx>,
        reempty_placeholder: ty::Region<'tcx>,
    ) -> Self {
        RegionsSubstitutor { tcx, restatic_placeholder, reempty_placeholder }
    }
}

impl<'tcx> TypeFolder<'tcx> for RegionsSubstitutor<'tcx> {
    fn tcx<'b>(&'b self) -> TyCtxt<'tcx> {
        self.tcx
    }

    fn fold_region(&mut self, r: Region<'tcx>) -> Region<'tcx> {
        match r {
            ty::ReStatic => self.restatic_placeholder,
            ty::ReEmpty(ui) => {
                assert_eq!(ui.as_usize(), 0);
                self.reempty_placeholder
            }

            _ => r.super_fold_with(self),
        }
    }
}
