extern crate proc_macro;
use std::{any::Any, fmt::Debug, iter, str::FromStr};

use bevy_macro_utils::derive_label;
use derive_syn_parse::Parse;
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
    token::{Colon, Comma},
    AngleBracketedGenericArguments, AttrStyle, Attribute, Data, DeriveInput, FnArg,
    GenericArgument, GenericParam, Generics, Item, ItemFn, Lifetime, LifetimeDef, Pat, Path,
    PathArguments, PathSegment, Stmt, Token, TraitBound, Type, TypeParam, TypeParamBound, TypePath,
    TypeReference, TypeTraitObject,
};
use zmaxion_macro_utils::ZmaxionManifest;

// use crate::proc_process::ProcessData;

// mod proc_process;

const ACCEPTED_TRAIT_BOUNDS: [&'static str; 4] = ["Bevy or Async", "Ack", "Trace", "Group"];

#[proc_macro_derive(RuntimeLabel)]
pub fn derive_runtime_label(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut trait_path = ZmaxionManifest::default().get_path("zmaxion_rt");
    trait_path
        .segments
        .push(format_ident!("pipe_runtimes").into());
    trait_path
        .segments
        .push(format_ident!("RuntimeLabel").into());
    derive_label(input, &trait_path)
}

#[proc_macro_derive(PipeObserver)]
pub fn derive_pipe_observer(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut path = zmaxion_param_path();
    let ident = input.ident;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let where_clause = where_clause.cloned().unwrap_or_else(|| syn::WhereClause {
        where_token: Default::default(),
        predicates: Default::default(),
    });

    (quote! {
        impl #impl_generics #path::PipeObserver for #ident #ty_generics #where_clause {
            fn pipe_executing(&mut self) {
            }
            fn pipe_executed<T>(&mut self, result: &#path::__export__::AnyResult<T>) -> #path::__export__::ControlFlow {
                #path::__export__::ControlFlow::default()
            }
        }
    })
        .into()
}

#[proc_macro_attribute]
#[proc_macro_error]
pub fn process(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let mut zmaxion = ZmaxionManifest::default().get_path("zmaxion_app");
    let mut input = parse_macro_input!(input as ItemFn);
    add_lifetimes(&mut input);
    let mut features = quote! {PipeFeatures::<AsyncRuntimeMarker>::default()};
    for stmt in &mut input.block.stmts {
        match stmt {
            Stmt::Item(Item::Const(item)) if item.ident == "FEATURES" => {
                let mut path_segments = Punctuated::<PathSegment, Token![::]>::new();
                path_segments.push(PathSegment {
                    ident: Ident::new("allow", Span::call_site()),
                    arguments: Default::default(),
                });
                item.attrs.push(Attribute {
                    pound_token: Default::default(),
                    style: AttrStyle::Outer,
                    bracket_token: Default::default(),
                    path: Path {
                        leading_colon: None,
                        segments: path_segments,
                    },
                    tokens: quote! {(unused)},
                });
                let expr = &item.expr;
                features = quote! {#expr};
            }
            _ => {}
        }
    }
    //    let out = ProcessData::parse(input);
    //    println!("{}", out);
    //    return out;
    let attrs = input.attrs;
    let vis = input.vis;
    let mut sig = input.sig;
    let block = input.block;
    let constness = sig.constness;
    let asyncness = sig.asyncness;
    let unsafety = sig.unsafety;
    let abi = sig.abi;
    let fn_token = sig.fn_token;
    let ident = sig.ident;
    let mut generics = sig.generics;
    let inputs = sig.inputs;
    let variadic = sig.variadic;
    let output = sig.output;
    let mut params = Vec::with_capacity(inputs.len());
    let mut n_impl_trait = 0;
    let mut register_generics = generics.params.clone();
    let param_names = inputs.iter().map(|x| match x {
        FnArg::Receiver(_) => abort!(x, "Expected a typed paramater"),
        FnArg::Typed(pat) => pat.pat.to_token_stream().to_string(),
    });
    let declaration = quote! {
        PipeDeclaration {
            param_names: vec![#(#param_names.to_string()),*],
            features: #features,
        }
    };
    let new_params: Vec<_> = inputs
        .into_iter()
        .map(|param| match param {
            FnArg::Receiver(r) => {
                abort!(r, "Only typed params are supported.");
            }
            FnArg::Typed(pat) => {
                let ty;
                let mut prefix = quote!();
                match *pat.ty {
                    Type::ImplTrait(imp) => {
                        let mut dyn_traits = Vec::new();
                        extend_bounds(imp.bounds.iter(), &mut dyn_traits);
                        let p = ParamBuilder::build(false, dyn_traits);
                        params.push(p);
                        let ident = Ident::new(&format!("__P{}", n_impl_trait), Span::call_site());
                        ty = quote! { #ident };
                        generics.params.push(GenericParam::Type(TypeParam {
                            attrs: vec![],
                            ident,
                            colon_token: Some(Colon::default()),
                            bounds: imp.bounds,
                            eq_token: None,
                            default: None,
                        }));
                        n_impl_trait += 1;
                    }
                    Type::Reference(r) => {
                        // &dyn Trait
                        // &(dyn TraitA + TraitB)
                        let mut dyn_traits = Vec::new();
                        match *r.elem {
                            Type::Paren(paren) => {
                                // (dyn TraitA + TraitB)
                                match *paren.elem {
                                    Type::TraitObject(to) => {
                                        extend_bounds(to.bounds.iter(), &mut dyn_traits);
                                    }
                                    _ => abort!(paren.elem, "Expected a trait object"),
                                }
                            }
                            Type::TraitObject(to) => {
                                extend_bounds(to.bounds.iter(), &mut dyn_traits);
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
                        ty = quote! {path};
                    }
                    typ => {
                        ty = typ.to_token_stream();
                    }
                };

                let attrs = pat.attrs;
                let pat = pat.pat;
                quote! {
                    #(#attrs)* #pat: #prefix #ty
                }
            }
        })
        .collect();
    let out = quote! {
        #(#attrs)* #vis #constness #asyncness #unsafety #abi #fn_token #ident #generics (#(#new_params)*) #variadic #output
        #block

    };
    let register_ident = "register_".to_string() + &ident.to_string();
    let register_ident = Ident::new(&register_ident, Span::call_site());
    let mut param_i = 0;
    let mut combination_is = vec![0; params.len()];
    let mut register = quote!();
    let call_generics: Vec<_> = generics
        .params
        .iter()
        .filter_map(|x| match x {
            GenericParam::Type(x) => {
                if x.ident.to_string().starts_with("__P") {
                    None
                } else {
                    let ident = &x.ident;
                    Some(quote! {#ident})
                }
            }
            _ => None,
        })
        .collect();
    //    let needs_injection = params.iter().fold(false, |acc, x| acc || x.is_injectable);
    //    if needs_injection {
    //        generics_param.push_punct(Default::default());
    //    }

    while param_i < params.len() {
        let iter = call_generics.iter().chain(
            params
                .iter()
                .enumerate()
                .map(|(i, x)| &x.combination_streams[combination_is[i]]),
        );
        register = quote! {
            #register
            app.register_pipe(#ident::<#(#iter),*>, declaration.clone());
        };
        for param_id in param_i..params.len() {
            if combination_is[param_id] < params[param_id].combinations.len() {
                if combination_is[param_id] + 1 == params[param_id].combinations.len() {
                    param_i += 1;
                    for i in 0..param_i {
                        combination_is[i] = 0;
                    }
                } else {
                    combination_is[param_id] += 1;
                }
                break;
            }
        }
    }
    let register_generics = register_generics.iter();

    let out = quote! {
        pub fn #register_ident<'a, 'b #(,#register_generics)*>(app: &'b mut #zmaxion::prelude::AppBuilder<'a>) -> &'b mut #zmaxion::prelude::AppBuilder<'a> {
            let declaration = #declaration;
            #register
            app
        }
        #out
    };

    println!("{}", out.to_string());
    out.into()
}

struct Param {
    is_injectable: bool,
    is_dynamic: bool,
    combinations: Vec<String>,
    combination_streams: Vec<TokenStream2>,
    bound: TokenStream2,
}

impl Param {
    pub fn ty(&self) -> TokenStream2 {
        let bound = &self.bound;
        if self.is_dynamic {
            quote! {
                &mut (dyn #bound)
            }
        } else {
            quote! {
                #bound
            }
        }
    }
}

fn extend_bounds<'a>(
    bounds: impl Iterator<Item = &'a TypeParamBound>,
    serialized: &mut Vec<(Span, String, TokenStream2)>,
) {
    for bound in bounds {
        match bound {
            TypeParamBound::Trait(t) => {
                let mut path = t.path.segments.iter().last().unwrap();
                let stream = path.to_token_stream();
                let mut s = path.to_token_stream().to_string();
                s.retain(|x| !x.is_whitespace());
                serialized.push((t.span(), s, stream));
            }
            _ => {}
        }
    }
}

//#[proc_macro_derive(PipeObserver)]
// pub fn derive_pipe_observer(input: TokenStream) -> TokenStream {
//    let input = parse_macro_input!(input as DeriveInput);
//    let mut trait_path = bevy_ecs_path();
//    trait_path.segments.push(format_ident!("schedule").into());
//    trait_path.segments.push(format_ident!("StageLabel").into());
//    let ident = input.ident;
//
//    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
//    let mut where_clause = where_clause.cloned().unwrap_or_else(|| syn::WhereClause {
//        where_token: Default::default(),
//        predicates: Default::default(),
//    });
//    where_clause.predicates.push(
//        syn::parse2(quote! { Self: Eq + ::std::fmt::Debug + ::std::hash::Hash + Clone + Send +
// Sync + 'static }).unwrap());
//
//    (quote! {
//        impl #impl_generics #trait_path for #ident #ty_generics #where_clause {
//            fn dyn_clone(&self) -> std::boxed::Box<dyn #trait_path> {
//                std::boxed::Box::new(std::clone::Clone::clone(self))
//            }
//        }
//    })
//    .into()
//}
//
pub(crate) fn zmaxion_param_path() -> syn::Path {
    ZmaxionManifest::default().get_path("zmaxion_param")
}

struct ParamBuilder {
    bounds: Vec<(Span, String, TokenStream2)>,
    combinations: Vec<String>,
    bound: TokenStream2,
    is_dynamic: bool,
}

impl ParamBuilder {
    fn build(is_dynamic: bool, bounds: Vec<(Span, String, TokenStream2)>) -> Param {
        let mut s = Self {
            bounds,
            combinations: vec![],
            bound: TokenStream2::default(),
            is_dynamic,
        };
        let mut is_injectable = false;
        if let Some(pos) = s.bounds.iter().position(|x| x.1.contains("TopicReader<")) {
            s.make_topic_combinations(pos, "Reader");
            is_injectable = true;
        } else if let Some(pos) = s.bounds.iter().position(|x| x.1.contains("TopicWriter<")) {
            s.make_topic_combinations(pos, "Writer");
            is_injectable = true;
        } else {
            abort_invalid_trait_bounds(&s.bounds[0].0, s.bounds.iter().map(|x| x.1.as_str()));
        }

        Param {
            is_injectable,
            is_dynamic,
            combination_streams: s.combinations.iter().map(|x| x.parse().unwrap()).collect(),
            combinations: s.combinations,
            bound: s.bound,
        }
    }

    fn make_topic_combinations(&mut self, pos: usize, kind: &str) {
        let rw = self.bounds.remove(pos);
        self.bounds.sort_by(|a, b| a.1.cmp(&b.1));
        let span = rw.0;
        let main = rw.2;
        let rw = rw.1;
        let start = rw.find('<').unwrap_or_else(|| panic!());
        let lifew = rw.find('\'').unwrap();
        let event_t_start = rw[lifew..].find(',').unwrap() + 1;
        let end = rw.rfind('>').unwrap_or_else(|| panic!());
        let event = rw[lifew + event_t_start..end].to_string();
        //        let event = rw[start + 1..end].to_string();
        let rw_name = &rw[..start];
        self.bound = if self.is_dynamic {
            let mut bounds: String = self.bounds.iter().map(|x| &x.2).join_concat().to_string();
            bounds.insert_str(0, rw_name);
            let ident = Ident::new(&bounds, Span::call_site());
            let event: proc_macro2::TokenStream = event.parse().unwrap();
            quote! {
                #ident<#event>
            }
        } else {
            let bounds = self.bounds.iter().map(|x| &x.2);
            quote! {
                #main #(+ #bounds)*
            }
        };
        let combs = &mut self.combinations;
        combs.push(format!("Bevy{}<'w, 's, AckEvent<TraceEvent<!>>>", kind));
        combs.push(format!("Bevy{}<'w, 's, AckEvent<!>>", kind));
        combs.push(format!("Bevy{}<'w, 's, !>", kind));
        combs.push(format!("Async{}<'w, 's, AckEvent<TraceEvent<!>>>", kind));
        combs.push(format!("Async{}<'w, 's, AckEvent<!>>", kind));
        combs.push(format!("Async{}<'w, 's, !>", kind));
        combs.push(format!(
            "BevyGroup{}<'w, 's, AckEvent<TraceEvent<!>>>",
            kind
        ));
        combs.push(format!("BevyGroup{}<'w, 's, AckEvent<!>>", kind));
        combs.push(format!("BevyGroup{}<'w, 's, !>", kind));
        combs.push(format!(
            "AsyncGroup{}<'w, 's, AckEvent<TraceEvent<!>>>",
            kind
        ));
        combs.push(format!("AsyncGroup{}<'w, 's, AckEvent<!>>", kind));
        combs.push(format!("AsyncGroup{}<'w, 's, !>", kind));
        if let Some(pos) = self.bounds.iter().position(|x| x.1.contains("Bevy")) {
            self.bounds.remove(pos);
            combs.retain(|x| x.contains("Bevy"));
        }
        if let Some(pos) = self.bounds.iter().position(|x| x.1.contains("Async")) {
            self.bounds.remove(pos);
            combs.retain(|x| x.contains("Async"));
        }
        if let Some(pos) = self.bounds.iter().position(|x| x.1.contains("Ack")) {
            self.bounds.remove(pos);
            combs.retain(|x| x.contains("Ack"));
        }
        if let Some(pos) = self.bounds.iter().position(|x| x.1.contains("Trace")) {
            self.bounds.remove(pos);
            combs.retain(|x| x.contains("Trace"));
        }
        if let Some(pos) = self.bounds.iter().position(|x| x.1.contains("Group")) {
            self.bounds.remove(pos);
            combs.retain(|x| x.contains("Group"));
        }
        if !self.bounds.is_empty() {
            abort_invalid_trait_bounds(&self.bounds[0].0, self.bounds.iter().map(|x| x.1.as_str()));
        }
        // event name could conflict with traits so replacing it later
        for c in combs {
            let pos = c.rfind('!').unwrap();
            c.replace_range(pos..pos + 1, &event);
        }
    }
}

fn abort_invalid_trait_bounds<'a>(span: &Span, unexpected: impl Iterator<Item = &'a str> + Clone) {
    abort!(
        span,
        "Unexpected trait bounds {}",
        unexpected.join_with(", ");
        help = "The following trait bounds are available only {}",
        ACCEPTED_TRAIT_BOUNDS.iter().join_with(", ");
        note = "To use generic arguments instead of injected ones use generic types rather than impl Trait";
    )
}
#[derive(Parse)]
struct Lifetimes {
    #[call(Punctuated::parse_terminated)]
    lifetimes: Punctuated<Lifetime, Token![,]>,
}

#[derive(Parse)]
struct Types {
    #[call(Punctuated::parse_terminated)]
    types: Punctuated<Type, Token![,]>,
}

#[derive(Parse)]
struct SuperTraits {
    #[bracket]
    _lifetime_bracket: token::Bracket,
    #[inside(_lifetime_bracket)]
    //    #[call(Punctuated::parse_terminated)]
    //    lifetimes: Punctuated<Lifetime, Token![,]>,
    //    #[call(Punctuated::parse_terminated)]
    lifetimes: TokenStream2,
    #[bracket]
    _type_bracket: token::Bracket,
    #[inside(_type_bracket)]
    //    #[call(Punctuated::parse_terminated)]
    //    types: Punctuated<Type, Token![,]>,
    types: TokenStream2,
    traits: TokenStream2,
}
#[derive(Parse)]
struct Traits {
    main: TypePath,
    #[prefix(Token![,])]
    #[call(Punctuated::parse_terminated)]
    traits: Punctuated<TypePath, Token![,]>,
}

#[proc_macro]
pub fn all_supertraits(input: TokenStream) -> TokenStream {
    supertraits(input, true)
}

#[proc_macro]
pub fn all_supertraits_for(input: TokenStream) -> TokenStream {
    supertraits(input, false)
}

fn supertraits(input: TokenStream, all: bool) -> TokenStream {
    let traits = parse_macro_input!(input as SuperTraits);
    let lifetimes = TokenStream::from(traits.lifetimes);
    let mut lifetimes = parse_macro_input!(lifetimes as Lifetimes).lifetimes;
    let types = TokenStream::from(traits.types);
    let mut types = parse_macro_input!(types as Types).types;
    let traits = TokenStream::from(traits.traits);
    let traits = parse_macro_input!(traits as Traits);
    if !lifetimes.is_empty() && !types.is_empty() {
        lifetimes.push_punct(Default::default());
    }
    if !types.is_empty() {
        types.push_punct(Default::default());
    }
    let main = traits.main;
    let target = if all {
        None
    } else {
        Some(traits.traits[0].path.segments.last().unwrap().ident.clone())
    };
    let mut trait_idents: Vec<_> = traits
        .traits
        .into_iter()
        .map(|mut x| x.path.segments.pop().unwrap().into_value())
        .collect();
    trait_idents.sort_by(|a, b| a.ident.cmp(&b.ident));
    let mut len = 2;
    let mut out = TokenStream2::new();
    while len <= trait_idents.len() + 1 {
        for offset in 0..=trait_idents.len() - (len - 1) {
            if let Some(target) = &target {
                if !trait_idents[offset..offset + len - 1]
                    .iter()
                    .map(|x| &x.ident)
                    .any(|x| x == target)
                {
                    continue;
                }
            }
            println!("{}, {}", len, offset);
            let mut super_trait = String::new();
            super_trait.extend(
                iter::once(&main.path.segments.last().unwrap().ident)
                    .chain(
                        trait_idents[offset..offset + len - 1]
                            .iter()
                            .map(|x| &x.ident),
                    )
                    .map(|x| x.to_string()),
            );
            let super_trait = Ident::new(&super_trait, Span::call_site());
            let bounds = trait_idents[offset..offset + len - 1].iter();
            let bounds2 = trait_idents[offset..offset + len - 1].iter();
            out = quote! {
                #out
                pub trait #super_trait<#lifetimes #types>: #main #(+ #bounds)* {

                }
                impl<#lifetimes #types __T: #main #(+ #bounds2)*> #super_trait<#lifetimes #types> for __T {

                }
            };
        }
        len += 1;
    }
    out.into()
}

fn add_lifetimes(item: &mut ItemFn) {
    let mut state_lifetime_present = false;
    let mut world_lifetime_present = false;
    item.sig.generics.params.iter().for_each(|x| match x {
        GenericParam::Lifetime(def) => {
            if def.lifetime.ident == Ident::new("'w", def.span()) {
                world_lifetime_present = true;
            }
            if def.lifetime.ident == Ident::new("'s", def.span()) {
                state_lifetime_present = true;
            }
        }
        _ => {}
    });
    if !state_lifetime_present {
        item.sig.generics.params.insert(
            0,
            GenericParam::Lifetime(LifetimeDef::new(Lifetime::new("'s", Span::call_site()))),
        );
    }
    if !world_lifetime_present {
        item.sig.generics.params.insert(
            0,
            GenericParam::Lifetime(LifetimeDef::new(Lifetime::new("'w", Span::call_site()))),
        );
    }
}

fn add_lifetimes_to_type(path: &mut TypePath) {
    let last_segment = path.path.segments.last_mut().unwrap();
    match &mut last_segment.arguments {
        PathArguments::None => {
            let mut lifetimes = Punctuated::<GenericArgument, Token![,]>::new();
            lifetimes.push(GenericArgument::Lifetime(Lifetime::new(
                "'w",
                Span::call_site(),
            )));
            lifetimes.push(GenericArgument::Lifetime(Lifetime::new(
                "'s",
                Span::call_site(),
            )));
            last_segment.arguments =
                PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                    colon2_token: None,
                    lt_token: Default::default(),
                    args: lifetimes,
                    gt_token: Default::default(),
                });
        }
        PathArguments::AngleBracketed(args) => {
            args.args.insert(
                0,
                GenericArgument::Lifetime(Lifetime::new("'w", Span::call_site())),
            );
            args.args.insert(
                0,
                GenericArgument::Lifetime(Lifetime::new("'s", Span::call_site())),
            );
        }
        PathArguments::Parenthesized(_) => {}
    }
}
