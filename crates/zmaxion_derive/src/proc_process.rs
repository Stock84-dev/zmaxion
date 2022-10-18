use joinery::JoinableIterator;
use proc_macro::{TokenStream, TokenTree};
use proc_macro2::{Ident, Span, TokenStream as TokenStream2, TokenTree as TokenTree2};
use proc_macro_error::{abort, proc_macro_error};
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    token,
    token::{Colon, Comma, Dyn},
    AngleBracketedGenericArguments, Data, DeriveInput, FnArg, GenericArgument, GenericParam,
    Generics, ItemFn, Lifetime, LifetimeDef, Pat, PatType, Path, PathArguments, PathSegment, Token,
    TraitBound, TraitBoundModifier, Type, TypeParam, TypeParamBound, TypePath, TypeReference,
    TypeTraitObject,
};
use zmaxion_macro_utils::ZmaxionManifest;

pub struct InjectedParam {
    pos: usize,
    streams: Vec<TokenStream2>,
}

impl InjectedParam {
    fn new(pos: usize, mut bounds: Vec<(Span, String)>) -> Self {
        let kind;
        if let Some(pos) = bounds.iter().position(|x| x.1.contains("TopicReader<")) {
            bounds.remove(pos);
            kind = "Reader";
        } else if let Some(pos) = bounds.iter().position(|x| x.1.contains("TopicWriter<")) {
            bounds.remove(pos);
            kind = "Writer";
        } else {
            abort_invalid_trait_bounds(&bounds[0].0, bounds.iter().map(|x| x.1.as_str()));
        }
        let mut combs = vec![];
        combs.push(format!("Bevy{}<AckEvent<TraceEvent<!>>>", kind));
        combs.push(format!("Bevy{}<AckEvent<!>>", kind));
        combs.push(format!("Bevy{}<!>", kind));
        combs.push(format!("Async{}<AckEvent<TraceEvent<!>>>", kind));
        combs.push(format!("Async{}<AckEvent<!>>", kind));
        combs.push(format!("Async{}<!>", kind));
        combs.push(format!("BevyGroup{}<AckEvent<TraceEvent<!>>>", kind));
        combs.push(format!("BevyGroup{}<AckEvent<!>>", kind));
        combs.push(format!("BevyGroup{}<!>", kind));
        combs.push(format!("AsyncGroup{}<AckEvent<TraceEvent<!>>>", kind));
        combs.push(format!("AsyncGroup{}<AckEvent<!>>", kind));
        combs.push(format!("AsyncGroup{}<!>", kind));
        if let Some(pos) = bounds.iter().position(|x| x.1.contains("Bevy")) {
            bounds.remove(pos);
            combs.retain(|x| x.contains("Bevy"));
        }
        if let Some(pos) = bounds.iter().position(|x| x.1.contains("Async")) {
            bounds.remove(pos);
            combs.retain(|x| x.contains("Async"));
        }
        if let Some(pos) = bounds.iter().position(|x| x.1.contains("Ack")) {
            bounds.remove(pos);
            combs.retain(|x| x.contains("Ack"));
        }
        if let Some(pos) = bounds.iter().position(|x| x.1.contains("Trace")) {
            bounds.remove(pos);
            combs.retain(|x| x.contains("Trace"));
        }
        if let Some(pos) = bounds.iter().position(|x| x.1.contains("Group")) {
            bounds.remove(pos);
            combs.retain(|x| x.contains("Group"));
        }
        if !bounds.is_empty() {
            abort_invalid_trait_bounds(&bounds[0].0, bounds.iter().map(|x| x.1.as_str()));
        }

        Self {
            pos,
            streams: combs.iter().map(|x| x.parse().unwrap()).collect(),
        }
    }
}

pub struct Param {
    injected: Option<InjectedParam>,
    name: Ident,
    ty: Type,
}

pub struct ProcessData {}

impl ProcessData {
    pub fn parse(mut item: ItemFn) -> TokenStream {
        add_lifetimes(&mut item);
        let mut injected_params = vec![];
        let mut n_impl_trait = 0;
        for (param_pos, x) in item.sig.inputs.iter_mut().enumerate() {
            match x {
                FnArg::Receiver(r) => {
                    abort!(r, "Only typed params are supported.");
                }
                FnArg::Typed(pat) => {
                    let ty = (*pat.ty).clone();
                    match ty {
                        Type::ImplTrait(imp) => {
                            // replace impl Trait with __P\d
                            let mut segments = Punctuated::<PathSegment, Token![::]>::new();
                            let ident =
                                Ident::new(&format!("__P{}", n_impl_trait), Span::call_site());
                            segments.push_value(PathSegment {
                                ident: ident.clone(),
                                arguments: Default::default(),
                            });
                            pat.ty = Box::new(Type::Path(TypePath {
                                qself: None,
                                path: Path {
                                    leading_colon: None,
                                    segments,
                                },
                            }));
                            let bounds = imp
                                .bounds
                                .iter()
                                .map(|x| {
                                    let mut x = quote!(#x).to_string();
                                    x.retain(|x| !x.is_whitespace());
                                    (imp.span(), x)
                                })
                                .collect();
                            injected_params.push(InjectedParam::new(param_pos, bounds));
                            item.sig.generics.params.push(GenericParam::Type(TypeParam {
                                attrs: vec![],
                                ident,
                                colon_token: Some(Colon::default()),
                                bounds: imp.bounds,
                                eq_token: None,
                                default: None,
                            }));
                            n_impl_trait += 1;
                        }
                        Type::Reference(mut r) => {
                            // &dyn Trait
                            // &(dyn TraitA + TraitB)
                            let mut dyn_traits = Vec::new();
                            match *r.elem {
                                Type::Paren(paren) => {
                                    // (dyn TraitA + TraitB)
                                    let mut supertrait =
                                        Punctuated::<TypeParamBound, Token![,]>::new();
                                    match *paren.elem {
                                        Type::TraitObject(to) => {
                                            //                                            let bounds
                                            // = to
                                            //                                                
                                            // .bounds
                                            //                                                
                                            // .iter()
                                            //                                                
                                            // .map(|x| {
                                            //                                                    
                                            // let mut x = x.to_string();
                                            //                                                    
                                            // x.retain(|x| !x.is_whitespace());
                                            //                                                    x
                                            //                                                })
                                            //                                                
                                            // .collect();
                                            //
                                            //                                            
                                            // extend_bounds(to.bounds.iter(), &mut dyn_traits);

                                            let mut lifetimes = Vec::<Lifetime>::new();
                                            let mut types = Vec::<Type>::new();
                                            to.bounds.into_iter().for_each(|x| match x {
                                                TypeParamBound::Trait(b) => {
                                                    match b.path.segments.last().unwrap().arguments
                                                    {
                                                        PathArguments::None => {}
                                                        PathArguments::AngleBracketed(a) => {
                                                            for arg in a.args {
                                                                match arg {
                                                                    GenericArgument::Lifetime(
                                                                        l,
                                                                    ) => {
                                                                        if !lifetimes.contains(&l) {
                                                                            lifetimes.push(l);
                                                                        }
                                                                    }
                                                                    GenericArgument::Type(t) => {
                                                                        if !types.contains(&t) {
                                                                            types.push(t);
                                                                        }
                                                                    }
                                                                    _ => {}
                                                                }
                                                            }
                                                        }
                                                        PathArguments::Parenthesized(_) => {}
                                                    }
                                                }
                                                TypeParamBound::Lifetime(s) => {
                                                    abort!(s, "expected trait bound")
                                                }
                                            });
                                            let generics = lifetimes
                                                .into_iter()
                                                .map(|x| GenericArgument::Lifetime(x))
                                                .chain(
                                                    types
                                                        .into_iter()
                                                        .map(|x| GenericArgument::Type(x)),
                                                );
                                            let mut bounds: String = to
                                                .bounds
                                                .iter()
                                                .map(|x| match x {
                                                    TypeParamBound::Trait(b) => {
                                                        b.path.segments.last().unwrap().ident
                                                    }
                                                    TypeParamBound::Lifetime(s) => {
                                                        abort!(s, "expected trait bound")
                                                    }
                                                })
                                                .join_concat()
                                                .to_string();
                                            //                                            
                                            // bounds.insert_str(0, rw_name);
                                            let ident = Ident::new(&bounds, Span::call_site());
                                            let mut segment = Punctuated::<PathSegment, Token![,]>::new();
                                            segment.push(PathSegment {
                                                ident,
                                                arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                                                    colon2_token: None,
                                                    lt_token: Default::default(),
                                                    args: Default::default(),
                                                    gt_token: Default::default()
                                                })
                                            });
                                            supertrait.push(TypeParamBound::Trait(TraitBound {
                                                paren_token: None,
                                                modifier: TraitBoundModifier::None,
                                                lifetimes: None,
                                                path: Path {
                                                    leading_colon: None,
                                                    segments:
                                                },
                                            }));
                                            let event: proc_macro2::TokenStream =
                                                event.parse().unwrap();
                                            quote! {
                                                #ident<#(#generics)*>
                                            }
                                        }
                                        _ => abort!(paren.elem, "Expected a trait object"),
                                    }
                                    r.elem = Box::new(Type::TraitObject(TypeTraitObject {
                                        dyn_token: Some(Dyn::default()),
                                        bounds: Default::default(),
                                    }));
                                }
                                Type::TraitObject(to) => {
                                    // extend_bounds(to.bounds.iter(), &mut dyn_traits);
                                }
                                x => abort!(
                                    x,
                                    "Expected either a trait object or a group of trait objects."
                                ),
                            }
                            let param = ParamBuilder::build(true, dyn_traits);
                            ty = param.ty();
                            params.push(param);
                        }
                        Type::Path(mut path) => {
                            add_lifetimes_to_type(&mut path);
                            pat.ty = Box::new(Type::Path(path));
                        }
                        typ => {}
                    };
                }
            }
        }
        let out = quote! {
            #item
        };
        out.into()
    }
}

fn abort_invalid_trait_bounds<'a>(
    span: &Span,
    unexpected: impl Iterator<Item = &'a str> + Clone,
) -> ! {
    abort!(
        span,
        "Unexpected trait bounds {}",
        unexpected.join_with(", ");
        help = "The following trait bounds are available: {}",
        ACCEPTED_TRAIT_BOUNDS.iter().join_with(", ");
        note = "To use generic arguments instead of injected ones use generic types rather than impl Trait";
    )
}

const ACCEPTED_TRAIT_BOUNDS: [&'static str; 4] = ["Bevy or Async", "Ack", "Trace", "Group"];
