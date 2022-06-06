extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput};
use zmaxion_macro_utils::ZmaxionManifest;

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
