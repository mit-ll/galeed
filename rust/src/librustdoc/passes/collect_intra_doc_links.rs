use rustc_ast as ast;
use rustc_data_structures::stable_set::FxHashSet;
use rustc_errors::{Applicability, DiagnosticBuilder};
use rustc_expand::base::SyntaxExtensionKind;
use rustc_feature::UnstableFeatures;
use rustc_hir as hir;
use rustc_hir::def::{
    DefKind,
    Namespace::{self, *},
    PerNS, Res,
};
use rustc_hir::def_id::DefId;
use rustc_middle::ty;
use rustc_resolve::ParentScope;
use rustc_session::lint;
use rustc_span::hygiene::MacroKind;
use rustc_span::symbol::Ident;
use rustc_span::symbol::Symbol;
use rustc_span::DUMMY_SP;
use smallvec::{smallvec, SmallVec};

use std::borrow::Cow;
use std::cell::Cell;
use std::ops::Range;

use crate::clean::*;
use crate::core::DocContext;
use crate::fold::DocFolder;
use crate::html::markdown::markdown_links;
use crate::passes::Pass;

use super::span_of_attrs;

pub const COLLECT_INTRA_DOC_LINKS: Pass = Pass {
    name: "collect-intra-doc-links",
    run: collect_intra_doc_links,
    description: "reads a crate's documentation to resolve intra-doc-links",
};

pub fn collect_intra_doc_links(krate: Crate, cx: &DocContext<'_>) -> Crate {
    if !UnstableFeatures::from_environment().is_nightly_build() {
        krate
    } else {
        let mut coll = LinkCollector::new(cx);

        coll.fold_crate(krate)
    }
}

enum ErrorKind<'a> {
    Resolve(Box<ResolutionFailure<'a>>),
    AnchorFailure(AnchorFailure),
}

impl<'a> From<ResolutionFailure<'a>> for ErrorKind<'a> {
    fn from(err: ResolutionFailure<'a>) -> Self {
        ErrorKind::Resolve(box err)
    }
}

#[derive(Debug)]
enum ResolutionFailure<'a> {
    /// This resolved, but with the wrong namespace.
    /// `Namespace` is the expected namespace (as opposed to the actual).
    WrongNamespace(Res, Namespace),
    /// This has a partial resolution, but is not in the TypeNS and so cannot
    /// have associated items or fields.
    CannotHaveAssociatedItems(Res, Namespace),
    /// `name` is the base name of the path (not necessarily the whole link)
    NotInScope { module_id: DefId, name: Cow<'a, str> },
    /// this is a primitive type without an impls (no associated methods)
    /// when will this actually happen?
    /// the `Res` is the primitive it resolved to
    NoPrimitiveImpl(Res, String),
    /// `[u8::not_found]`
    /// the `Res` is the primitive it resolved to
    NoPrimitiveAssocItem { res: Res, prim_name: &'a str, assoc_item: Symbol },
    /// `[S::not_found]`
    /// the `String` is the associated item that wasn't found
    NoAssocItem(Res, Symbol),
    /// should not ever happen
    NoParentItem,
    /// this could be an enum variant, but the last path fragment wasn't resolved.
    /// the `String` is the variant that didn't exist
    NotAVariant(Res, Symbol),
    /// used to communicate that this should be ignored, but shouldn't be reported to the user
    Dummy,
}

impl ResolutionFailure<'a> {
    // A partial or full resolution
    fn res(&self) -> Option<Res> {
        use ResolutionFailure::*;
        match self {
            NoPrimitiveAssocItem { res, .. }
            | NoAssocItem(res, _)
            | NoPrimitiveImpl(res, _)
            | NotAVariant(res, _)
            | WrongNamespace(res, _)
            | CannotHaveAssociatedItems(res, _) => Some(*res),
            NotInScope { .. } | NoParentItem | Dummy => None,
        }
    }

    // This resolved fully (not just partially) but is erroneous for some other reason
    fn full_res(&self) -> Option<Res> {
        match self {
            Self::WrongNamespace(res, _) => Some(*res),
            _ => None,
        }
    }
}

enum AnchorFailure {
    MultipleAnchors,
    RustdocAnchorConflict(Res),
}

struct LinkCollector<'a, 'tcx> {
    cx: &'a DocContext<'tcx>,
    // NOTE: this may not necessarily be a module in the current crate
    mod_ids: Vec<DefId>,
    /// This is used to store the kind of associated items,
    /// because `clean` and the disambiguator code expect them to be different.
    /// See the code for associated items on inherent impls for details.
    kind_side_channel: Cell<Option<(DefKind, DefId)>>,
}

impl<'a, 'tcx> LinkCollector<'a, 'tcx> {
    fn new(cx: &'a DocContext<'tcx>) -> Self {
        LinkCollector { cx, mod_ids: Vec::new(), kind_side_channel: Cell::new(None) }
    }

    fn variant_field(
        &self,
        path_str: &'path str,
        current_item: &Option<String>,
        module_id: DefId,
        extra_fragment: &Option<String>,
    ) -> Result<(Res, Option<String>), ErrorKind<'path>> {
        let cx = self.cx;

        debug!("looking for enum variant {}", path_str);
        let mut split = path_str.rsplitn(3, "::");
        let variant_field_name = split
            .next()
            .map(|f| Symbol::intern(f))
            .expect("fold_item should ensure link is non-empty");
        let variant_name =
            // we're not sure this is a variant at all, so use the full string
            split.next().map(|f| Symbol::intern(f)).ok_or_else(|| ResolutionFailure::NotInScope {
                module_id,
                name: path_str.into(),
            })?;
        let path = split
            .next()
            .map(|f| {
                if f == "self" || f == "Self" {
                    if let Some(name) = current_item.as_ref() {
                        return name.clone();
                    }
                }
                f.to_owned()
            })
            .ok_or_else(|| ResolutionFailure::NotInScope {
                module_id,
                name: variant_name.to_string().into(),
            })?;
        let ty_res = cx
            .enter_resolver(|resolver| {
                resolver.resolve_str_path_error(DUMMY_SP, &path, TypeNS, module_id)
            })
            .map(|(_, res)| res)
            .unwrap_or(Res::Err);
        if let Res::Err = ty_res {
            return Err(ResolutionFailure::NotInScope { module_id, name: path.into() }.into());
        }
        let ty_res = ty_res.map_id(|_| panic!("unexpected node_id"));
        match ty_res {
            Res::Def(DefKind::Enum, did) => {
                if cx
                    .tcx
                    .inherent_impls(did)
                    .iter()
                    .flat_map(|imp| cx.tcx.associated_items(*imp).in_definition_order())
                    .any(|item| item.ident.name == variant_name)
                {
                    // This is just to let `fold_item` know that this shouldn't be considered;
                    // it's a bug for the error to make it to the user
                    return Err(ResolutionFailure::Dummy.into());
                }
                match cx.tcx.type_of(did).kind() {
                    ty::Adt(def, _) if def.is_enum() => {
                        if def.all_fields().any(|item| item.ident.name == variant_field_name) {
                            Ok((
                                ty_res,
                                Some(format!(
                                    "variant.{}.field.{}",
                                    variant_name, variant_field_name
                                )),
                            ))
                        } else {
                            Err(ResolutionFailure::NotAVariant(ty_res, variant_field_name).into())
                        }
                    }
                    _ => unreachable!(),
                }
            }
            // `variant_field` looks at 3 different path segments in a row.
            // But `NoAssocItem` assumes there are only 2. Check to see if there's
            // an intermediate segment that resolves.
            _ => {
                let intermediate_path = format!("{}::{}", path, variant_name);
                // NOTE: we have to be careful here, because we're already in `resolve`.
                // We know this doesn't recurse forever because we use a shorter path each time.
                // NOTE: this uses `TypeNS` because nothing else has a valid path segment after
                let kind = if let Some(intermediate) = self.check_full_res(
                    TypeNS,
                    &intermediate_path,
                    module_id,
                    current_item,
                    extra_fragment,
                ) {
                    ResolutionFailure::NoAssocItem(intermediate, variant_field_name)
                } else {
                    // Even with the shorter path, it didn't resolve, so say that.
                    ResolutionFailure::NoAssocItem(ty_res, variant_name)
                };
                Err(kind.into())
            }
        }
    }

    /// Resolves a string as a macro.
    fn macro_resolve(
        &self,
        path_str: &'a str,
        module_id: DefId,
    ) -> Result<Res, ResolutionFailure<'a>> {
        let cx = self.cx;
        let path = ast::Path::from_ident(Ident::from_str(path_str));
        cx.enter_resolver(|resolver| {
            if let Ok((Some(ext), res)) = resolver.resolve_macro_path(
                &path,
                None,
                &ParentScope::module(resolver.graph_root()),
                false,
                false,
            ) {
                if let SyntaxExtensionKind::LegacyBang { .. } = ext.kind {
                    return Some(Ok(res.map_id(|_| panic!("unexpected id"))));
                }
            }
            if let Some(res) = resolver.all_macros().get(&Symbol::intern(path_str)) {
                return Some(Ok(res.map_id(|_| panic!("unexpected id"))));
            }
            debug!("resolving {} as a macro in the module {:?}", path_str, module_id);
            if let Ok((_, res)) =
                resolver.resolve_str_path_error(DUMMY_SP, path_str, MacroNS, module_id)
            {
                // don't resolve builtins like `#[derive]`
                if let Res::Def(..) = res {
                    let res = res.map_id(|_| panic!("unexpected node_id"));
                    return Some(Ok(res));
                }
            }
            None
        })
        // This weird control flow is so we don't borrow the resolver more than once at a time
        .unwrap_or_else(|| {
            let mut split = path_str.rsplitn(2, "::");
            if let Some((parent, base)) = split.next().and_then(|x| Some((split.next()?, x))) {
                if let Some(res) = self.check_full_res(TypeNS, parent, module_id, &None, &None) {
                    return Err(if matches!(res, Res::PrimTy(_)) {
                        ResolutionFailure::NoPrimitiveAssocItem {
                            res,
                            prim_name: parent,
                            assoc_item: Symbol::intern(base),
                        }
                    } else {
                        ResolutionFailure::NoAssocItem(res, Symbol::intern(base))
                    });
                }
            }
            Err(ResolutionFailure::NotInScope { module_id, name: path_str.into() })
        })
    }

    /// Resolves a string as a path within a particular namespace. Also returns an optional
    /// URL fragment in the case of variants and methods.
    fn resolve<'path>(
        &self,
        path_str: &'path str,
        ns: Namespace,
        current_item: &Option<String>,
        module_id: DefId,
        extra_fragment: &Option<String>,
    ) -> Result<(Res, Option<String>), ErrorKind<'path>> {
        let cx = self.cx;

        let result = cx.enter_resolver(|resolver| {
            resolver.resolve_str_path_error(DUMMY_SP, &path_str, ns, module_id)
        });
        debug!("{} resolved to {:?} in namespace {:?}", path_str, result, ns);
        let result = match result {
            Ok((_, Res::Err)) => Err(()),
            x => x,
        };

        if let Ok((_, res)) = result {
            let res = res.map_id(|_| panic!("unexpected node_id"));
            // In case this is a trait item, skip the
            // early return and try looking for the trait.
            let value = match res {
                Res::Def(DefKind::AssocFn | DefKind::AssocConst, _) => true,
                Res::Def(DefKind::AssocTy, _) => false,
                Res::Def(DefKind::Variant, _) => {
                    return handle_variant(cx, res, extra_fragment);
                }
                // Not a trait item; just return what we found.
                Res::PrimTy(..) => {
                    if extra_fragment.is_some() {
                        return Err(ErrorKind::AnchorFailure(
                            AnchorFailure::RustdocAnchorConflict(res),
                        ));
                    }
                    return Ok((res, Some(path_str.to_owned())));
                }
                Res::Def(DefKind::Mod, _) => {
                    return Ok((res, extra_fragment.clone()));
                }
                _ => {
                    return Ok((res, extra_fragment.clone()));
                }
            };

            if value != (ns == ValueNS) {
                return Err(ResolutionFailure::WrongNamespace(res, ns).into());
            }
        } else if let Some((path, prim)) = is_primitive(path_str, ns) {
            if extra_fragment.is_some() {
                return Err(ErrorKind::AnchorFailure(AnchorFailure::RustdocAnchorConflict(prim)));
            }
            return Ok((prim, Some(path.to_owned())));
        }

        // Try looking for methods and associated items.
        let mut split = path_str.rsplitn(2, "::");
        // this can be an `unwrap()` because we ensure the link is never empty
        let item_name = Symbol::intern(split.next().unwrap());
        let path_root = split
            .next()
            .map(|f| {
                if f == "self" || f == "Self" {
                    if let Some(name) = current_item.as_ref() {
                        return name.clone();
                    }
                }
                f.to_owned()
            })
            // If there's no `::`, it's not an associated item.
            // So we can be sure that `rustc_resolve` was accurate when it said it wasn't resolved.
            .ok_or_else(|| {
                debug!("found no `::`, assumming {} was correctly not in scope", item_name);
                ResolutionFailure::NotInScope { module_id, name: item_name.to_string().into() }
            })?;

        if let Some((path, prim)) = is_primitive(&path_root, TypeNS) {
            let impls = primitive_impl(cx, &path)
                .ok_or_else(|| ResolutionFailure::NoPrimitiveImpl(prim, path_root.into()))?;
            for &impl_ in impls {
                let link = cx
                    .tcx
                    .associated_items(impl_)
                    .find_by_name_and_namespace(
                        cx.tcx,
                        Ident::with_dummy_span(item_name),
                        ns,
                        impl_,
                    )
                    .map(|item| match item.kind {
                        ty::AssocKind::Fn => "method",
                        ty::AssocKind::Const => "associatedconstant",
                        ty::AssocKind::Type => "associatedtype",
                    })
                    .map(|out| (prim, Some(format!("{}#{}.{}", path, out, item_name))));
                if let Some(link) = link {
                    return Ok(link);
                }
            }
            debug!(
                "returning primitive error for {}::{} in {} namespace",
                path,
                item_name,
                ns.descr()
            );
            return Err(ResolutionFailure::NoPrimitiveAssocItem {
                res: prim,
                prim_name: path,
                assoc_item: item_name,
            }
            .into());
        }

        let ty_res = cx
            .enter_resolver(|resolver| {
                // only types can have associated items
                resolver.resolve_str_path_error(DUMMY_SP, &path_root, TypeNS, module_id)
            })
            .map(|(_, res)| res);
        let ty_res = match ty_res {
            Err(()) | Ok(Res::Err) => {
                return if ns == Namespace::ValueNS {
                    self.variant_field(path_str, current_item, module_id, extra_fragment)
                } else {
                    // See if it only broke because of the namespace.
                    let kind = cx.enter_resolver(|resolver| {
                        // NOTE: this doesn't use `check_full_res` because we explicitly want to ignore `TypeNS` (we already checked it)
                        for &ns in &[MacroNS, ValueNS] {
                            match resolver
                                .resolve_str_path_error(DUMMY_SP, &path_root, ns, module_id)
                            {
                                Ok((_, Res::Err)) | Err(()) => {}
                                Ok((_, res)) => {
                                    let res = res.map_id(|_| panic!("unexpected node_id"));
                                    return ResolutionFailure::CannotHaveAssociatedItems(res, ns);
                                }
                            }
                        }
                        ResolutionFailure::NotInScope { module_id, name: path_root.into() }
                    });
                    Err(kind.into())
                };
            }
            Ok(res) => res,
        };
        let ty_res = ty_res.map_id(|_| panic!("unexpected node_id"));
        let res = match ty_res {
            Res::Def(DefKind::Struct | DefKind::Union | DefKind::Enum | DefKind::TyAlias, did) => {
                debug!("looking for associated item named {} for item {:?}", item_name, did);
                // Checks if item_name belongs to `impl SomeItem`
                let assoc_item = cx
                    .tcx
                    .inherent_impls(did)
                    .iter()
                    .flat_map(|&imp| {
                        cx.tcx.associated_items(imp).find_by_name_and_namespace(
                            cx.tcx,
                            Ident::with_dummy_span(item_name),
                            ns,
                            imp,
                        )
                    })
                    .map(|item| (item.kind, item.def_id))
                    // There should only ever be one associated item that matches from any inherent impl
                    .next()
                    // Check if item_name belongs to `impl SomeTrait for SomeItem`
                    // This gives precedence to `impl SomeItem`:
                    // Although having both would be ambiguous, use impl version for compat. sake.
                    // To handle that properly resolve() would have to support
                    // something like [`ambi_fn`](<SomeStruct as SomeTrait>::ambi_fn)
                    .or_else(|| {
                        let kind =
                            resolve_associated_trait_item(did, module_id, item_name, ns, &self.cx);
                        debug!("got associated item kind {:?}", kind);
                        kind
                    });

                if let Some((kind, id)) = assoc_item {
                    let out = match kind {
                        ty::AssocKind::Fn => "method",
                        ty::AssocKind::Const => "associatedconstant",
                        ty::AssocKind::Type => "associatedtype",
                    };
                    Some(if extra_fragment.is_some() {
                        Err(ErrorKind::AnchorFailure(AnchorFailure::RustdocAnchorConflict(ty_res)))
                    } else {
                        // HACK(jynelson): `clean` expects the type, not the associated item.
                        // but the disambiguator logic expects the associated item.
                        // Store the kind in a side channel so that only the disambiguator logic looks at it.
                        self.kind_side_channel.set(Some((kind.as_def_kind(), id)));
                        Ok((ty_res, Some(format!("{}.{}", out, item_name))))
                    })
                } else if ns == Namespace::ValueNS {
                    debug!("looking for variants or fields named {} for {:?}", item_name, did);
                    match cx.tcx.type_of(did).kind() {
                        ty::Adt(def, _) => {
                            let field = if def.is_enum() {
                                def.all_fields().find(|item| item.ident.name == item_name)
                            } else {
                                def.non_enum_variant()
                                    .fields
                                    .iter()
                                    .find(|item| item.ident.name == item_name)
                            };
                            field.map(|item| {
                                if extra_fragment.is_some() {
                                    let res = Res::Def(
                                        if def.is_enum() {
                                            DefKind::Variant
                                        } else {
                                            DefKind::Field
                                        },
                                        item.did,
                                    );
                                    Err(ErrorKind::AnchorFailure(
                                        AnchorFailure::RustdocAnchorConflict(res),
                                    ))
                                } else {
                                    Ok((
                                        ty_res,
                                        Some(format!(
                                            "{}.{}",
                                            if def.is_enum() { "variant" } else { "structfield" },
                                            item.ident
                                        )),
                                    ))
                                }
                            })
                        }
                        _ => None,
                    }
                } else {
                    // We already know this isn't in ValueNS, so no need to check variant_field
                    return Err(ResolutionFailure::NoAssocItem(ty_res, item_name).into());
                }
            }
            Res::Def(DefKind::Trait, did) => cx
                .tcx
                .associated_items(did)
                .find_by_name_and_namespace(cx.tcx, Ident::with_dummy_span(item_name), ns, did)
                .map(|item| {
                    let kind = match item.kind {
                        ty::AssocKind::Const => "associatedconstant",
                        ty::AssocKind::Type => "associatedtype",
                        ty::AssocKind::Fn => {
                            if item.defaultness.has_value() {
                                "method"
                            } else {
                                "tymethod"
                            }
                        }
                    };

                    if extra_fragment.is_some() {
                        Err(ErrorKind::AnchorFailure(AnchorFailure::RustdocAnchorConflict(ty_res)))
                    } else {
                        let res = Res::Def(item.kind.as_def_kind(), item.def_id);
                        Ok((res, Some(format!("{}.{}", kind, item_name))))
                    }
                }),
            _ => None,
        };
        res.unwrap_or_else(|| {
            if ns == Namespace::ValueNS {
                self.variant_field(path_str, current_item, module_id, extra_fragment)
            } else {
                Err(ResolutionFailure::NoAssocItem(ty_res, item_name).into())
            }
        })
    }

    /// Used for reporting better errors.
    ///
    /// Returns whether the link resolved 'fully' in another namespace.
    /// 'fully' here means that all parts of the link resolved, not just some path segments.
    /// This returns the `Res` even if it was erroneous for some reason
    /// (such as having invalid URL fragments or being in the wrong namespace).
    fn check_full_res(
        &self,
        ns: Namespace,
        path_str: &str,
        module_id: DefId,
        current_item: &Option<String>,
        extra_fragment: &Option<String>,
    ) -> Option<Res> {
        let check_full_res_inner = |this: &Self, result: Result<Res, ErrorKind<'_>>| {
            let res = match result {
                Ok(res) => Some(res),
                Err(ErrorKind::Resolve(box kind)) => kind.full_res(),
                Err(ErrorKind::AnchorFailure(AnchorFailure::RustdocAnchorConflict(res))) => {
                    Some(res)
                }
                Err(ErrorKind::AnchorFailure(AnchorFailure::MultipleAnchors)) => None,
            };
            this.kind_side_channel.take().map(|(kind, id)| Res::Def(kind, id)).or(res)
        };
        // cannot be used for macro namespace
        let check_full_res = |this: &Self, ns| {
            let result = this.resolve(path_str, ns, current_item, module_id, extra_fragment);
            check_full_res_inner(this, result.map(|(res, _)| res))
        };
        let check_full_res_macro = |this: &Self| {
            let result = this.macro_resolve(path_str, module_id);
            check_full_res_inner(this, result.map_err(ErrorKind::from))
        };
        match ns {
            Namespace::MacroNS => check_full_res_macro(self),
            Namespace::TypeNS | Namespace::ValueNS => check_full_res(self, ns),
        }
    }
}

fn resolve_associated_trait_item(
    did: DefId,
    module: DefId,
    item_name: Symbol,
    ns: Namespace,
    cx: &DocContext<'_>,
) -> Option<(ty::AssocKind, DefId)> {
    let ty = cx.tcx.type_of(did);
    // First consider automatic impls: `impl From<T> for T`
    let implicit_impls = crate::clean::get_auto_trait_and_blanket_impls(cx, ty, did);
    let mut candidates: Vec<_> = implicit_impls
        .flat_map(|impl_outer| {
            match impl_outer.inner {
                ImplItem(impl_) => {
                    debug!("considering auto or blanket impl for trait {:?}", impl_.trait_);
                    // Give precedence to methods that were overridden
                    if !impl_.provided_trait_methods.contains(&*item_name.as_str()) {
                        let mut items = impl_.items.into_iter().filter_map(|assoc| {
                            if assoc.name.as_deref() != Some(&*item_name.as_str()) {
                                return None;
                            }
                            let kind = assoc
                                .inner
                                .as_assoc_kind()
                                .expect("inner items for a trait should be associated items");
                            if kind.namespace() != ns {
                                return None;
                            }

                            trace!("considering associated item {:?}", assoc.inner);
                            // We have a slight issue: normal methods come from `clean` types,
                            // but provided methods come directly from `tcx`.
                            // Fortunately, we don't need the whole method, we just need to know
                            // what kind of associated item it is.
                            Some((kind, assoc.def_id))
                        });
                        let assoc = items.next();
                        debug_assert_eq!(items.count(), 0);
                        assoc
                    } else {
                        // These are provided methods or default types:
                        // ```
                        // trait T {
                        //   type A = usize;
                        //   fn has_default() -> A { 0 }
                        // }
                        // ```
                        let trait_ = impl_.trait_.unwrap().def_id().unwrap();
                        cx.tcx
                            .associated_items(trait_)
                            .find_by_name_and_namespace(
                                cx.tcx,
                                Ident::with_dummy_span(item_name),
                                ns,
                                trait_,
                            )
                            .map(|assoc| (assoc.kind, assoc.def_id))
                    }
                }
                _ => panic!("get_impls returned something that wasn't an impl"),
            }
        })
        .collect();

    // Next consider explicit impls: `impl MyTrait for MyType`
    // Give precedence to inherent impls.
    if candidates.is_empty() {
        let traits = traits_implemented_by(cx, did, module);
        debug!("considering traits {:?}", traits);
        candidates.extend(traits.iter().filter_map(|&trait_| {
            cx.tcx
                .associated_items(trait_)
                .find_by_name_and_namespace(cx.tcx, Ident::with_dummy_span(item_name), ns, trait_)
                .map(|assoc| (assoc.kind, assoc.def_id))
        }));
    }
    // FIXME: warn about ambiguity
    debug!("the candidates were {:?}", candidates);
    candidates.pop()
}

/// Given a type, return all traits in scope in `module` implemented by that type.
///
/// NOTE: this cannot be a query because more traits could be available when more crates are compiled!
/// So it is not stable to serialize cross-crate.
fn traits_implemented_by(cx: &DocContext<'_>, type_: DefId, module: DefId) -> FxHashSet<DefId> {
    let mut cache = cx.module_trait_cache.borrow_mut();
    let in_scope_traits = cache.entry(module).or_insert_with(|| {
        cx.enter_resolver(|resolver| {
            resolver.traits_in_scope(module).into_iter().map(|candidate| candidate.def_id).collect()
        })
    });

    let ty = cx.tcx.type_of(type_);
    let iter = in_scope_traits.iter().flat_map(|&trait_| {
        trace!("considering explicit impl for trait {:?}", trait_);
        let mut saw_impl = false;
        // Look at each trait implementation to see if it's an impl for `did`
        cx.tcx.for_each_relevant_impl(trait_, ty, |impl_| {
            // FIXME: this is inefficient, find a way to short-circuit for_each_* so this doesn't take as long
            if saw_impl {
                return;
            }

            let trait_ref = cx.tcx.impl_trait_ref(impl_).expect("this is not an inherent impl");
            // Check if these are the same type.
            let impl_type = trait_ref.self_ty();
            trace!(
                "comparing type {} with kind {:?} against type {:?}",
                impl_type,
                impl_type.kind(),
                type_
            );
            // Fast path: if this is a primitive simple `==` will work
            saw_impl = impl_type == ty
                || match impl_type.kind() {
                    // Check if these are the same def_id
                    ty::Adt(def, _) => {
                        debug!("adt def_id: {:?}", def.did);
                        def.did == type_
                    }
                    ty::Foreign(def_id) => *def_id == type_,
                    _ => false,
                };
        });
        if saw_impl { Some(trait_) } else { None }
    });
    iter.collect()
}

/// Check for resolve collisions between a trait and its derive
///
/// These are common and we should just resolve to the trait in that case
fn is_derive_trait_collision<T>(ns: &PerNS<Result<(Res, T), ResolutionFailure<'_>>>) -> bool {
    if let PerNS {
        type_ns: Ok((Res::Def(DefKind::Trait, _), _)),
        macro_ns: Ok((Res::Def(DefKind::Macro(MacroKind::Derive), _), _)),
        ..
    } = *ns
    {
        true
    } else {
        false
    }
}

impl<'a, 'tcx> DocFolder for LinkCollector<'a, 'tcx> {
    fn fold_item(&mut self, mut item: Item) -> Option<Item> {
        use rustc_middle::ty::DefIdTree;

        let parent_node = if item.is_fake() {
            // FIXME: is this correct?
            None
        // If we're documenting the crate root itself, it has no parent. Use the root instead.
        } else if item.def_id.is_top_level_module() {
            Some(item.def_id)
        } else {
            let mut current = item.def_id;
            // The immediate parent might not always be a module.
            // Find the first parent which is.
            loop {
                if let Some(parent) = self.cx.tcx.parent(current) {
                    if self.cx.tcx.def_kind(parent) == DefKind::Mod {
                        break Some(parent);
                    }
                    current = parent;
                } else {
                    debug!(
                        "{:?} has no parent (kind={:?}, original was {:?})",
                        current,
                        self.cx.tcx.def_kind(current),
                        item.def_id
                    );
                    break None;
                }
            }
        };

        if parent_node.is_some() {
            trace!("got parent node for {:?} {:?}, id {:?}", item.type_(), item.name, item.def_id);
        }

        let current_item = match item.inner {
            ModuleItem(..) => {
                if item.attrs.inner_docs {
                    if item.def_id.is_top_level_module() { item.name.clone() } else { None }
                } else {
                    match parent_node.or(self.mod_ids.last().copied()) {
                        Some(parent) if !parent.is_top_level_module() => {
                            // FIXME: can we pull the parent module's name from elsewhere?
                            Some(self.cx.tcx.item_name(parent).to_string())
                        }
                        _ => None,
                    }
                }
            }
            ImplItem(Impl { ref for_, .. }) => {
                for_.def_id().map(|did| self.cx.tcx.item_name(did).to_string())
            }
            // we don't display docs on `extern crate` items anyway, so don't process them.
            ExternCrateItem(..) => {
                debug!("ignoring extern crate item {:?}", item.def_id);
                return self.fold_item_recur(item);
            }
            ImportItem(Import::Simple(ref name, ..)) => Some(name.clone()),
            MacroItem(..) => None,
            _ => item.name.clone(),
        };

        if item.is_mod() && item.attrs.inner_docs {
            self.mod_ids.push(item.def_id);
        }

        let dox = item.attrs.collapsed_doc_value().unwrap_or_else(String::new);
        trace!("got documentation '{}'", dox);

        // find item's parent to resolve `Self` in item's docs below
        let parent_name = self.cx.as_local_hir_id(item.def_id).and_then(|item_hir| {
            let parent_hir = self.cx.tcx.hir().get_parent_item(item_hir);
            let item_parent = self.cx.tcx.hir().find(parent_hir);
            match item_parent {
                Some(hir::Node::Item(hir::Item {
                    kind:
                        hir::ItemKind::Impl {
                            self_ty:
                                hir::Ty {
                                    kind:
                                        hir::TyKind::Path(hir::QPath::Resolved(
                                            _,
                                            hir::Path { segments, .. },
                                        )),
                                    ..
                                },
                            ..
                        },
                    ..
                })) => segments.first().map(|seg| seg.ident.to_string()),
                Some(hir::Node::Item(hir::Item {
                    ident, kind: hir::ItemKind::Enum(..), ..
                }))
                | Some(hir::Node::Item(hir::Item {
                    ident, kind: hir::ItemKind::Struct(..), ..
                }))
                | Some(hir::Node::Item(hir::Item {
                    ident, kind: hir::ItemKind::Union(..), ..
                }))
                | Some(hir::Node::Item(hir::Item {
                    ident, kind: hir::ItemKind::Trait(..), ..
                })) => Some(ident.to_string()),
                _ => None,
            }
        });

        for (ori_link, link_range) in markdown_links(&dox) {
            self.resolve_link(
                &mut item,
                &dox,
                &current_item,
                parent_node,
                &parent_name,
                ori_link,
                link_range,
            );
        }

        if item.is_mod() && !item.attrs.inner_docs {
            self.mod_ids.push(item.def_id);
        }

        if item.is_mod() {
            let ret = self.fold_item_recur(item);

            self.mod_ids.pop();

            ret
        } else {
            self.fold_item_recur(item)
        }
    }
}

impl LinkCollector<'_, '_> {
    fn resolve_link(
        &self,
        item: &mut Item,
        dox: &str,
        current_item: &Option<String>,
        parent_node: Option<DefId>,
        parent_name: &Option<String>,
        ori_link: String,
        link_range: Option<Range<usize>>,
    ) {
        trace!("considering link '{}'", ori_link);

        // Bail early for real links.
        if ori_link.contains('/') {
            return;
        }

        // [] is mostly likely not supposed to be a link
        if ori_link.is_empty() {
            return;
        }

        let cx = self.cx;
        let link = ori_link.replace("`", "");
        let parts = link.split('#').collect::<Vec<_>>();
        let (link, extra_fragment) = if parts.len() > 2 {
            anchor_failure(cx, &item, &link, dox, link_range, AnchorFailure::MultipleAnchors);
            return;
        } else if parts.len() == 2 {
            if parts[0].trim().is_empty() {
                // This is an anchor to an element of the current page, nothing to do in here!
                return;
            }
            (parts[0], Some(parts[1].to_owned()))
        } else {
            (parts[0], None)
        };
        let resolved_self;
        let link_text;
        let mut path_str;
        let disambiguator;
        let (mut res, mut fragment) = {
            path_str = if let Ok((d, path)) = Disambiguator::from_str(&link) {
                disambiguator = Some(d);
                path
            } else {
                disambiguator = None;
                &link
            }
            .trim();

            if path_str.contains(|ch: char| !(ch.is_alphanumeric() || ch == ':' || ch == '_')) {
                return;
            }

            // We stripped `()` and `!` when parsing the disambiguator.
            // Add them back to be displayed, but not prefix disambiguators.
            link_text = disambiguator
                .map(|d| d.display_for(path_str))
                .unwrap_or_else(|| path_str.to_owned());

            // In order to correctly resolve intra-doc-links we need to
            // pick a base AST node to work from.  If the documentation for
            // this module came from an inner comment (//!) then we anchor
            // our name resolution *inside* the module.  If, on the other
            // hand it was an outer comment (///) then we anchor the name
            // resolution in the parent module on the basis that the names
            // used are more likely to be intended to be parent names.  For
            // this, we set base_node to None for inner comments since
            // we've already pushed this node onto the resolution stack but
            // for outer comments we explicitly try and resolve against the
            // parent_node first.
            let base_node = if item.is_mod() && item.attrs.inner_docs {
                self.mod_ids.last().copied()
            } else {
                parent_node
            };

            let module_id = if let Some(id) = base_node {
                id
            } else {
                debug!("attempting to resolve item without parent module: {}", path_str);
                let err_kind = ResolutionFailure::NoParentItem.into();
                resolution_failure(
                    self,
                    &item,
                    path_str,
                    disambiguator,
                    dox,
                    link_range,
                    smallvec![err_kind],
                );
                return;
            };

            // replace `Self` with suitable item's parent name
            if path_str.starts_with("Self::") {
                if let Some(ref name) = parent_name {
                    resolved_self = format!("{}::{}", name, &path_str[6..]);
                    path_str = &resolved_self;
                }
            }

            match self.resolve_with_disambiguator(
                disambiguator,
                item,
                dox,
                path_str,
                current_item,
                module_id,
                extra_fragment,
                &ori_link,
                link_range.clone(),
            ) {
                Some(x) => x,
                None => return,
            }
        };

        // Check for a primitive which might conflict with a module
        // Report the ambiguity and require that the user specify which one they meant.
        // FIXME: could there ever be a primitive not in the type namespace?
        if matches!(
            disambiguator,
            None | Some(Disambiguator::Namespace(Namespace::TypeNS) | Disambiguator::Primitive)
        ) && !matches!(res, Res::PrimTy(_))
        {
            if let Some((path, prim)) = is_primitive(path_str, TypeNS) {
                // `prim@char`
                if matches!(disambiguator, Some(Disambiguator::Primitive)) {
                    if fragment.is_some() {
                        anchor_failure(
                            cx,
                            &item,
                            path_str,
                            dox,
                            link_range,
                            AnchorFailure::RustdocAnchorConflict(prim),
                        );
                        return;
                    }
                    res = prim;
                    fragment = Some(path.to_owned());
                } else {
                    // `[char]` when a `char` module is in scope
                    let candidates = vec![res, prim];
                    ambiguity_error(cx, &item, path_str, dox, link_range, candidates);
                    return;
                }
            }
        }

        let report_mismatch = |specified: Disambiguator, resolved: Disambiguator| {
            // The resolved item did not match the disambiguator; give a better error than 'not found'
            let msg = format!("incompatible link kind for `{}`", path_str);
            report_diagnostic(cx, &msg, &item, dox, &link_range, |diag, sp| {
                let note = format!(
                    "this link resolved to {} {}, which is not {} {}",
                    resolved.article(),
                    resolved.descr(),
                    specified.article(),
                    specified.descr()
                );
                diag.note(&note);
                suggest_disambiguator(resolved, diag, path_str, dox, sp, &link_range);
            });
        };
        if let Res::PrimTy(_) = res {
            match disambiguator {
                Some(Disambiguator::Primitive | Disambiguator::Namespace(_)) | None => {
                    item.attrs.links.push(ItemLink {
                        link: ori_link,
                        link_text: path_str.to_owned(),
                        did: None,
                        fragment,
                    });
                }
                Some(other) => {
                    report_mismatch(other, Disambiguator::Primitive);
                    return;
                }
            }
        } else {
            debug!("intra-doc link to {} resolved to {:?}", path_str, res);

            // Disallow e.g. linking to enums with `struct@`
            if let Res::Def(kind, _) = res {
                debug!("saw kind {:?} with disambiguator {:?}", kind, disambiguator);
                match (self.kind_side_channel.take().map(|(kind, _)| kind).unwrap_or(kind), disambiguator) {
                    | (DefKind::Const | DefKind::ConstParam | DefKind::AssocConst | DefKind::AnonConst, Some(Disambiguator::Kind(DefKind::Const)))
                    // NOTE: this allows 'method' to mean both normal functions and associated functions
                    // This can't cause ambiguity because both are in the same namespace.
                    | (DefKind::Fn | DefKind::AssocFn, Some(Disambiguator::Kind(DefKind::Fn)))
                    // These are namespaces; allow anything in the namespace to match
                    | (_, Some(Disambiguator::Namespace(_)))
                    // If no disambiguator given, allow anything
                    | (_, None)
                    // All of these are valid, so do nothing
                    => {}
                    (actual, Some(Disambiguator::Kind(expected))) if actual == expected => {}
                    (_, Some(specified @ Disambiguator::Kind(_) | specified @ Disambiguator::Primitive)) => {
                        report_mismatch(specified, Disambiguator::Kind(kind));
                        return;
                    }
                }
            }

            // item can be non-local e.g. when using #[doc(primitive = "pointer")]
            if let Some((src_id, dst_id)) = res
                .opt_def_id()
                .and_then(|def_id| def_id.as_local())
                .and_then(|dst_id| item.def_id.as_local().map(|src_id| (src_id, dst_id)))
            {
                use rustc_hir::def_id::LOCAL_CRATE;

                let hir_src = self.cx.tcx.hir().local_def_id_to_hir_id(src_id);
                let hir_dst = self.cx.tcx.hir().local_def_id_to_hir_id(dst_id);

                if self.cx.tcx.privacy_access_levels(LOCAL_CRATE).is_exported(hir_src)
                    && !self.cx.tcx.privacy_access_levels(LOCAL_CRATE).is_exported(hir_dst)
                {
                    privacy_error(cx, &item, &path_str, dox, link_range);
                    return;
                }
            }
            let id = register_res(cx, res);
            item.attrs.links.push(ItemLink { link: ori_link, link_text, did: Some(id), fragment });
        }
    }

    fn resolve_with_disambiguator(
        &self,
        disambiguator: Option<Disambiguator>,
        item: &mut Item,
        dox: &str,
        path_str: &str,
        current_item: &Option<String>,
        base_node: DefId,
        extra_fragment: Option<String>,
        ori_link: &str,
        link_range: Option<Range<usize>>,
    ) -> Option<(Res, Option<String>)> {
        match disambiguator.map(Disambiguator::ns) {
            Some(ns @ (ValueNS | TypeNS)) => {
                match self.resolve(path_str, ns, &current_item, base_node, &extra_fragment) {
                    Ok(res) => Some(res),
                    Err(ErrorKind::Resolve(box mut kind)) => {
                        // We only looked in one namespace. Try to give a better error if possible.
                        if kind.full_res().is_none() {
                            let other_ns = if ns == ValueNS { TypeNS } else { ValueNS };
                            for &new_ns in &[other_ns, MacroNS] {
                                if let Some(res) = self.check_full_res(
                                    new_ns,
                                    path_str,
                                    base_node,
                                    &current_item,
                                    &extra_fragment,
                                ) {
                                    kind = ResolutionFailure::WrongNamespace(res, ns);
                                    break;
                                }
                            }
                        }
                        resolution_failure(
                            self,
                            &item,
                            path_str,
                            disambiguator,
                            dox,
                            link_range,
                            smallvec![kind],
                        );
                        // This could just be a normal link or a broken link
                        // we could potentially check if something is
                        // "intra-doc-link-like" and warn in that case.
                        return None;
                    }
                    Err(ErrorKind::AnchorFailure(msg)) => {
                        anchor_failure(self.cx, &item, &ori_link, dox, link_range, msg);
                        return None;
                    }
                }
            }
            None => {
                // Try everything!
                let mut candidates = PerNS {
                    macro_ns: self
                        .macro_resolve(path_str, base_node)
                        .map(|res| (res, extra_fragment.clone())),
                    type_ns: match self.resolve(
                        path_str,
                        TypeNS,
                        &current_item,
                        base_node,
                        &extra_fragment,
                    ) {
                        Ok(res) => {
                            debug!("got res in TypeNS: {:?}", res);
                            Ok(res)
                        }
                        Err(ErrorKind::AnchorFailure(msg)) => {
                            anchor_failure(self.cx, &item, ori_link, dox, link_range, msg);
                            return None;
                        }
                        Err(ErrorKind::Resolve(box kind)) => Err(kind),
                    },
                    value_ns: match self.resolve(
                        path_str,
                        ValueNS,
                        &current_item,
                        base_node,
                        &extra_fragment,
                    ) {
                        Ok(res) => Ok(res),
                        Err(ErrorKind::AnchorFailure(msg)) => {
                            anchor_failure(self.cx, &item, ori_link, dox, link_range, msg);
                            return None;
                        }
                        Err(ErrorKind::Resolve(box kind)) => Err(kind),
                    }
                    .and_then(|(res, fragment)| {
                        // Constructors are picked up in the type namespace.
                        match res {
                            Res::Def(DefKind::Ctor(..), _) | Res::SelfCtor(..) => {
                                Err(ResolutionFailure::WrongNamespace(res, TypeNS))
                            }
                            _ => match (fragment, extra_fragment) {
                                (Some(fragment), Some(_)) => {
                                    // Shouldn't happen but who knows?
                                    Ok((res, Some(fragment)))
                                }
                                (fragment, None) | (None, fragment) => Ok((res, fragment)),
                            },
                        }
                    }),
                };

                let len = candidates.iter().filter(|res| res.is_ok()).count();

                if len == 0 {
                    resolution_failure(
                        self,
                        &item,
                        path_str,
                        disambiguator,
                        dox,
                        link_range,
                        candidates.into_iter().filter_map(|res| res.err()).collect(),
                    );
                    // this could just be a normal link
                    return None;
                }

                if len == 1 {
                    Some(candidates.into_iter().filter_map(|res| res.ok()).next().unwrap())
                } else if len == 2 && is_derive_trait_collision(&candidates) {
                    Some(candidates.type_ns.unwrap())
                } else {
                    if is_derive_trait_collision(&candidates) {
                        candidates.macro_ns = Err(ResolutionFailure::Dummy);
                    }
                    // If we're reporting an ambiguity, don't mention the namespaces that failed
                    let candidates = candidates.map(|candidate| candidate.ok().map(|(res, _)| res));
                    ambiguity_error(
                        self.cx,
                        &item,
                        path_str,
                        dox,
                        link_range,
                        candidates.present_items().collect(),
                    );
                    return None;
                }
            }
            Some(MacroNS) => {
                match self.macro_resolve(path_str, base_node) {
                    Ok(res) => Some((res, extra_fragment)),
                    Err(mut kind) => {
                        // `macro_resolve` only looks in the macro namespace. Try to give a better error if possible.
                        for &ns in &[TypeNS, ValueNS] {
                            if let Some(res) = self.check_full_res(
                                ns,
                                path_str,
                                base_node,
                                &current_item,
                                &extra_fragment,
                            ) {
                                kind = ResolutionFailure::WrongNamespace(res, MacroNS);
                                break;
                            }
                        }
                        resolution_failure(
                            self,
                            &item,
                            path_str,
                            disambiguator,
                            dox,
                            link_range,
                            smallvec![kind],
                        );
                        return None;
                    }
                }
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Disambiguator {
    Primitive,
    Kind(DefKind),
    Namespace(Namespace),
}

impl Disambiguator {
    /// The text that should be displayed when the path is rendered as HTML.
    ///
    /// NOTE: `path` is not the original link given by the user, but a name suitable for passing to `resolve`.
    fn display_for(&self, path: &str) -> String {
        match self {
            // FIXME: this will have different output if the user had `m!()` originally.
            Self::Kind(DefKind::Macro(MacroKind::Bang)) => format!("{}!", path),
            Self::Kind(DefKind::Fn) => format!("{}()", path),
            _ => path.to_owned(),
        }
    }

    /// (disambiguator, path_str)
    fn from_str(link: &str) -> Result<(Self, &str), ()> {
        use Disambiguator::{Kind, Namespace as NS, Primitive};

        let find_suffix = || {
            let suffixes = [
                ("!()", DefKind::Macro(MacroKind::Bang)),
                ("()", DefKind::Fn),
                ("!", DefKind::Macro(MacroKind::Bang)),
            ];
            for &(suffix, kind) in &suffixes {
                if link.ends_with(suffix) {
                    return Ok((Kind(kind), link.trim_end_matches(suffix)));
                }
            }
            Err(())
        };

        if let Some(idx) = link.find('@') {
            let (prefix, rest) = link.split_at(idx);
            let d = match prefix {
                "struct" => Kind(DefKind::Struct),
                "enum" => Kind(DefKind::Enum),
                "trait" => Kind(DefKind::Trait),
                "union" => Kind(DefKind::Union),
                "module" | "mod" => Kind(DefKind::Mod),
                "const" | "constant" => Kind(DefKind::Const),
                "static" => Kind(DefKind::Static),
                "function" | "fn" | "method" => Kind(DefKind::Fn),
                "derive" => Kind(DefKind::Macro(MacroKind::Derive)),
                "type" => NS(Namespace::TypeNS),
                "value" => NS(Namespace::ValueNS),
                "macro" => NS(Namespace::MacroNS),
                "prim" | "primitive" => Primitive,
                _ => return find_suffix(),
            };
            Ok((d, &rest[1..]))
        } else {
            find_suffix()
        }
    }

    /// WARNING: panics on `Res::Err`
    fn from_res(res: Res) -> Self {
        match res {
            Res::Def(kind, _) => Disambiguator::Kind(kind),
            Res::PrimTy(_) => Disambiguator::Primitive,
            _ => Disambiguator::Namespace(res.ns().expect("can't call `from_res` on Res::err")),
        }
    }

    fn suggestion(self) -> Suggestion {
        let kind = match self {
            Disambiguator::Primitive => return Suggestion::Prefix("prim"),
            Disambiguator::Kind(kind) => kind,
            Disambiguator::Namespace(_) => panic!("display_for cannot be used on namespaces"),
        };
        if kind == DefKind::Macro(MacroKind::Bang) {
            return Suggestion::Macro;
        } else if kind == DefKind::Fn || kind == DefKind::AssocFn {
            return Suggestion::Function;
        }

        let prefix = match kind {
            DefKind::Struct => "struct",
            DefKind::Enum => "enum",
            DefKind::Trait => "trait",
            DefKind::Union => "union",
            DefKind::Mod => "mod",
            DefKind::Const | DefKind::ConstParam | DefKind::AssocConst | DefKind::AnonConst => {
                "const"
            }
            DefKind::Static => "static",
            DefKind::Macro(MacroKind::Derive) => "derive",
            // Now handle things that don't have a specific disambiguator
            _ => match kind
                .ns()
                .expect("tried to calculate a disambiguator for a def without a namespace?")
            {
                Namespace::TypeNS => "type",
                Namespace::ValueNS => "value",
                Namespace::MacroNS => "macro",
            },
        };

        Suggestion::Prefix(prefix)
    }

    fn ns(self) -> Namespace {
        match self {
            Self::Namespace(n) => n,
            Self::Kind(k) => {
                k.ns().expect("only DefKinds with a valid namespace can be disambiguators")
            }
            Self::Primitive => TypeNS,
        }
    }

    fn article(self) -> &'static str {
        match self {
            Self::Namespace(_) => panic!("article() doesn't make sense for namespaces"),
            Self::Kind(k) => k.article(),
            Self::Primitive => "a",
        }
    }

    fn descr(self) -> &'static str {
        match self {
            Self::Namespace(n) => n.descr(),
            // HACK(jynelson): by looking at the source I saw the DefId we pass
            // for `expected.descr()` doesn't matter, since it's not a crate
            Self::Kind(k) => k.descr(DefId::local(hir::def_id::DefIndex::from_usize(0))),
            Self::Primitive => "builtin type",
        }
    }
}

enum Suggestion {
    Prefix(&'static str),
    Function,
    Macro,
}

impl Suggestion {
    fn descr(&self) -> Cow<'static, str> {
        match self {
            Self::Prefix(x) => format!("prefix with `{}@`", x).into(),
            Self::Function => "add parentheses".into(),
            Self::Macro => "add an exclamation mark".into(),
        }
    }

    fn as_help(&self, path_str: &str) -> String {
        // FIXME: if this is an implied shortcut link, it's bad style to suggest `@`
        match self {
            Self::Prefix(prefix) => format!("{}@{}", prefix, path_str),
            Self::Function => format!("{}()", path_str),
            Self::Macro => format!("{}!", path_str),
        }
    }
}

/// Reports a diagnostic for an intra-doc link.
///
/// If no link range is provided, or the source span of the link cannot be determined, the span of
/// the entire documentation block is used for the lint. If a range is provided but the span
/// calculation fails, a note is added to the diagnostic pointing to the link in the markdown.
///
/// The `decorate` callback is invoked in all cases to allow further customization of the
/// diagnostic before emission. If the span of the link was able to be determined, the second
/// parameter of the callback will contain it, and the primary span of the diagnostic will be set
/// to it.
fn report_diagnostic(
    cx: &DocContext<'_>,
    msg: &str,
    item: &Item,
    dox: &str,
    link_range: &Option<Range<usize>>,
    decorate: impl FnOnce(&mut DiagnosticBuilder<'_>, Option<rustc_span::Span>),
) {
    let hir_id = match cx.as_local_hir_id(item.def_id) {
        Some(hir_id) => hir_id,
        None => {
            // If non-local, no need to check anything.
            info!("ignoring warning from parent crate: {}", msg);
            return;
        }
    };

    let attrs = &item.attrs;
    let sp = span_of_attrs(attrs).unwrap_or(item.source.span());

    cx.tcx.struct_span_lint_hir(lint::builtin::BROKEN_INTRA_DOC_LINKS, hir_id, sp, |lint| {
        let mut diag = lint.build(msg);

        let span = link_range
            .as_ref()
            .and_then(|range| super::source_span_for_markdown_range(cx, dox, range, attrs));

        if let Some(link_range) = link_range {
            if let Some(sp) = span {
                diag.set_span(sp);
            } else {
                // blah blah blah\nblah\nblah [blah] blah blah\nblah blah
                //                       ^     ~~~~
                //                       |     link_range
                //                       last_new_line_offset
                let last_new_line_offset = dox[..link_range.start].rfind('\n').map_or(0, |n| n + 1);
                let line = dox[last_new_line_offset..].lines().next().unwrap_or("");

                // Print the line containing the `link_range` and manually mark it with '^'s.
                diag.note(&format!(
                    "the link appears in this line:\n\n{line}\n\
                     {indicator: <before$}{indicator:^<found$}",
                    line = line,
                    indicator = "",
                    before = link_range.start - last_new_line_offset,
                    found = link_range.len(),
                ));
            }
        }

        decorate(&mut diag, span);

        diag.emit();
    });
}

fn resolution_failure(
    collector: &LinkCollector<'_, '_>,
    item: &Item,
    path_str: &str,
    disambiguator: Option<Disambiguator>,
    dox: &str,
    link_range: Option<Range<usize>>,
    kinds: SmallVec<[ResolutionFailure<'_>; 3]>,
) {
    report_diagnostic(
        collector.cx,
        &format!("unresolved link to `{}`", path_str),
        item,
        dox,
        &link_range,
        |diag, sp| {
            let in_scope = kinds.iter().any(|kind| kind.res().is_some());
            let item = |res: Res| {
                format!(
                    "the {} `{}`",
                    res.descr(),
                    collector.cx.tcx.item_name(res.def_id()).to_string()
                )
            };
            let assoc_item_not_allowed = |res: Res| {
                let def_id = res.def_id();
                let name = collector.cx.tcx.item_name(def_id);
                format!(
                    "`{}` is {} {}, not a module or type, and cannot have associated items",
                    name,
                    res.article(),
                    res.descr()
                )
            };
            // ignore duplicates
            let mut variants_seen = SmallVec::<[_; 3]>::new();
            for mut failure in kinds {
                // Check if _any_ parent of the path gets resolved.
                // If so, report it and say the first which failed; if not, say the first path segment didn't resolve.
                if let ResolutionFailure::NotInScope { module_id, name } = &mut failure {
                    let mut current = name.as_ref();
                    loop {
                        current = match current.rsplitn(2, "::").nth(1) {
                            Some(p) => p,
                            None => {
                                *name = current.to_owned().into();
                                break;
                            }
                        };
                        if let Some(res) =
                            collector.check_full_res(TypeNS, &current, *module_id, &None, &None)
                        {
                            failure = ResolutionFailure::NoAssocItem(res, Symbol::intern(current));
                            break;
                        }
                    }
                }
                let variant = std::mem::discriminant(&failure);
                if variants_seen.contains(&variant) {
                    continue;
                }
                variants_seen.push(variant);
                let note = match failure {
                    ResolutionFailure::NotInScope { module_id, name, .. } => {
                        if in_scope {
                            continue;
                        }
                        // NOTE: uses an explicit `continue` so the `note:` will come before the `help:`
                        let module_name = collector.cx.tcx.item_name(module_id);
                        let note = format!("no item named `{}` in `{}`", name, module_name);
                        if let Some(span) = sp {
                            diag.span_label(span, &note);
                        } else {
                            diag.note(&note);
                        }
                        // If the link has `::` in the path, assume it's meant to be an intra-doc link
                        if !path_str.contains("::") {
                            // Otherwise, the `[]` might be unrelated.
                            // FIXME(https://github.com/raphlinus/pulldown-cmark/issues/373):
                            // don't show this for autolinks (`<>`), `()` style links, or reference links
                            diag.help(r#"to escape `[` and `]` characters, add '\' before them like `\[` or `\]`"#);
                        }
                        continue;
                    }
                    ResolutionFailure::Dummy => continue,
                    ResolutionFailure::WrongNamespace(res, expected_ns) => {
                        if let Res::Def(kind, _) = res {
                            let disambiguator = Disambiguator::Kind(kind);
                            suggest_disambiguator(
                                disambiguator,
                                diag,
                                path_str,
                                dox,
                                sp,
                                &link_range,
                            )
                        }

                        format!(
                            "this link resolves to {}, which is not in the {} namespace",
                            item(res),
                            expected_ns.descr()
                        )
                    }
                    ResolutionFailure::NoParentItem => {
                        diag.level = rustc_errors::Level::Bug;
                        "all intra doc links should have a parent item".to_owned()
                    }
                    ResolutionFailure::NoPrimitiveImpl(res, _) => format!(
                        "this link partially resolves to {}, which does not have any associated items",
                        item(res),
                    ),
                    ResolutionFailure::NoPrimitiveAssocItem { prim_name, assoc_item, .. } => {
                        format!(
                            "the builtin type `{}` does not have an associated item named `{}`",
                            prim_name, assoc_item
                        )
                    }
                    ResolutionFailure::NoAssocItem(res, assoc_item) => {
                        use DefKind::*;

                        let (kind, def_id) = match res {
                            Res::Def(kind, def_id) => (kind, def_id),
                            x => unreachable!(
                                "primitives are covered above and other `Res` variants aren't possible at module scope: {:?}",
                                x,
                            ),
                        };
                        let name = collector.cx.tcx.item_name(def_id);
                        let path_description = if let Some(disambiguator) = disambiguator {
                            disambiguator.descr()
                        } else {
                            match kind {
                                Mod | ForeignMod => "inner item",
                                Struct => "field or associated item",
                                Enum | Union => "variant or associated item",
                                Variant
                                | Field
                                | Closure
                                | Generator
                                | AssocTy
                                | AssocConst
                                | AssocFn
                                | Fn
                                | Macro(_)
                                | Const
                                | ConstParam
                                | ExternCrate
                                | Use
                                | LifetimeParam
                                | Ctor(_, _)
                                | AnonConst => {
                                    let note = assoc_item_not_allowed(res);
                                    if let Some(span) = sp {
                                        diag.span_label(span, &note);
                                    } else {
                                        diag.note(&note);
                                    }
                                    return;
                                }
                                Trait | TyAlias | ForeignTy | OpaqueTy | TraitAlias | TyParam
                                | Static => "associated item",
                                Impl | GlobalAsm => unreachable!("not a path"),
                            }
                        };
                        format!(
                            "the {} `{}` has no {} named `{}`",
                            res.descr(),
                            name,
                            path_description,
                            assoc_item
                        )
                    }
                    ResolutionFailure::CannotHaveAssociatedItems(res, _) => {
                        assoc_item_not_allowed(res)
                    }
                    ResolutionFailure::NotAVariant(res, variant) => format!(
                        "this link partially resolves to {}, but there is no variant named {}",
                        item(res),
                        variant
                    ),
                };
                if let Some(span) = sp {
                    diag.span_label(span, &note);
                } else {
                    diag.note(&note);
                }
            }
        },
    );
}

fn anchor_failure(
    cx: &DocContext<'_>,
    item: &Item,
    path_str: &str,
    dox: &str,
    link_range: Option<Range<usize>>,
    failure: AnchorFailure,
) {
    let msg = match failure {
        AnchorFailure::MultipleAnchors => format!("`{}` contains multiple anchors", path_str),
        AnchorFailure::RustdocAnchorConflict(res) => format!(
            "`{}` contains an anchor, but links to {kind}s are already anchored",
            path_str,
            kind = res.descr(),
        ),
    };

    report_diagnostic(cx, &msg, item, dox, &link_range, |diag, sp| {
        if let Some(sp) = sp {
            diag.span_label(sp, "contains invalid anchor");
        }
    });
}

fn ambiguity_error(
    cx: &DocContext<'_>,
    item: &Item,
    path_str: &str,
    dox: &str,
    link_range: Option<Range<usize>>,
    candidates: Vec<Res>,
) {
    let mut msg = format!("`{}` is ", path_str);

    match candidates.as_slice() {
        [first_def, second_def] => {
            msg += &format!(
                "both {} {} and {} {}",
                first_def.article(),
                first_def.descr(),
                second_def.article(),
                second_def.descr(),
            );
        }
        _ => {
            let mut candidates = candidates.iter().peekable();
            while let Some(res) = candidates.next() {
                if candidates.peek().is_some() {
                    msg += &format!("{} {}, ", res.article(), res.descr());
                } else {
                    msg += &format!("and {} {}", res.article(), res.descr());
                }
            }
        }
    }

    report_diagnostic(cx, &msg, item, dox, &link_range, |diag, sp| {
        if let Some(sp) = sp {
            diag.span_label(sp, "ambiguous link");
        } else {
            diag.note("ambiguous link");
        }

        for res in candidates {
            let disambiguator = Disambiguator::from_res(res);
            suggest_disambiguator(disambiguator, diag, path_str, dox, sp, &link_range);
        }
    });
}

fn suggest_disambiguator(
    disambiguator: Disambiguator,
    diag: &mut DiagnosticBuilder<'_>,
    path_str: &str,
    dox: &str,
    sp: Option<rustc_span::Span>,
    link_range: &Option<Range<usize>>,
) {
    let suggestion = disambiguator.suggestion();
    let help = format!("to link to the {}, {}", disambiguator.descr(), suggestion.descr());

    if let Some(sp) = sp {
        let link_range = link_range.as_ref().expect("must have a link range if we have a span");
        let msg = if dox.bytes().nth(link_range.start) == Some(b'`') {
            format!("`{}`", suggestion.as_help(path_str))
        } else {
            suggestion.as_help(path_str)
        };

        diag.span_suggestion(sp, &help, msg, Applicability::MaybeIncorrect);
    } else {
        diag.help(&format!("{}: {}", help, suggestion.as_help(path_str)));
    }
}

fn privacy_error(
    cx: &DocContext<'_>,
    item: &Item,
    path_str: &str,
    dox: &str,
    link_range: Option<Range<usize>>,
) {
    let item_name = item.name.as_deref().unwrap_or("<unknown>");
    let msg =
        format!("public documentation for `{}` links to private item `{}`", item_name, path_str);

    report_diagnostic(cx, &msg, item, dox, &link_range, |diag, sp| {
        if let Some(sp) = sp {
            diag.span_label(sp, "this item is private");
        }

        let note_msg = if cx.render_options.document_private {
            "this link resolves only because you passed `--document-private-items`, but will break without"
        } else {
            "this link will resolve properly if you pass `--document-private-items`"
        };
        diag.note(note_msg);
    });
}

/// Given an enum variant's res, return the res of its enum and the associated fragment.
fn handle_variant(
    cx: &DocContext<'_>,
    res: Res,
    extra_fragment: &Option<String>,
) -> Result<(Res, Option<String>), ErrorKind<'static>> {
    use rustc_middle::ty::DefIdTree;

    if extra_fragment.is_some() {
        return Err(ErrorKind::AnchorFailure(AnchorFailure::RustdocAnchorConflict(res)));
    }
    let parent = if let Some(parent) = cx.tcx.parent(res.def_id()) {
        parent
    } else {
        return Err(ResolutionFailure::NoParentItem.into());
    };
    let parent_def = Res::Def(DefKind::Enum, parent);
    let variant = cx.tcx.expect_variant_res(res);
    Ok((parent_def, Some(format!("variant.{}", variant.ident.name))))
}

const PRIMITIVES: &[(&str, Res)] = &[
    ("u8", Res::PrimTy(hir::PrimTy::Uint(rustc_ast::UintTy::U8))),
    ("u16", Res::PrimTy(hir::PrimTy::Uint(rustc_ast::UintTy::U16))),
    ("u32", Res::PrimTy(hir::PrimTy::Uint(rustc_ast::UintTy::U32))),
    ("u64", Res::PrimTy(hir::PrimTy::Uint(rustc_ast::UintTy::U64))),
    ("u128", Res::PrimTy(hir::PrimTy::Uint(rustc_ast::UintTy::U128))),
    ("usize", Res::PrimTy(hir::PrimTy::Uint(rustc_ast::UintTy::Usize))),
    ("i8", Res::PrimTy(hir::PrimTy::Int(rustc_ast::IntTy::I8))),
    ("i16", Res::PrimTy(hir::PrimTy::Int(rustc_ast::IntTy::I16))),
    ("i32", Res::PrimTy(hir::PrimTy::Int(rustc_ast::IntTy::I32))),
    ("i64", Res::PrimTy(hir::PrimTy::Int(rustc_ast::IntTy::I64))),
    ("i128", Res::PrimTy(hir::PrimTy::Int(rustc_ast::IntTy::I128))),
    ("isize", Res::PrimTy(hir::PrimTy::Int(rustc_ast::IntTy::Isize))),
    ("f32", Res::PrimTy(hir::PrimTy::Float(rustc_ast::FloatTy::F32))),
    ("f64", Res::PrimTy(hir::PrimTy::Float(rustc_ast::FloatTy::F64))),
    ("str", Res::PrimTy(hir::PrimTy::Str)),
    ("bool", Res::PrimTy(hir::PrimTy::Bool)),
    ("true", Res::PrimTy(hir::PrimTy::Bool)),
    ("false", Res::PrimTy(hir::PrimTy::Bool)),
    ("char", Res::PrimTy(hir::PrimTy::Char)),
];

fn is_primitive(path_str: &str, ns: Namespace) -> Option<(&'static str, Res)> {
    if ns == TypeNS {
        PRIMITIVES
            .iter()
            .filter(|x| x.0 == path_str)
            .copied()
            .map(|x| if x.0 == "true" || x.0 == "false" { ("bool", x.1) } else { x })
            .next()
    } else {
        None
    }
}

fn primitive_impl(cx: &DocContext<'_>, path_str: &str) -> Option<&'static SmallVec<[DefId; 4]>> {
    Some(PrimitiveType::from_symbol(Symbol::intern(path_str))?.impls(cx.tcx))
}
