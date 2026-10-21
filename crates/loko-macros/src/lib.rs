//! loko's macros, used by its service crates and re-exported by `loko`.
//!
//! - `#[service]` on a trait `X` adds a trait `HasX` (a type that holds an `X`) and implements `X`
//!   for every `HasX` by forwarding each method to the held service.
//! - `#[derive(Services)]` implements `HasX` for a struct's fields marked `#[service(X, …)]`, so the
//!   struct is every service its fields provide.
//!
//! The `HasX` traits follow RIO's "Has" type classes, and choosing a field to forward to with a derive
//! follows ambassador (`PRIOR_ART.md`).

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    Data, DeriveInput, Error, FnArg, Ident, ItemTrait, Pat, PatIdent, Path, ReturnType, Signature, Token, TraitItem,
    TraitItemFn, parse_macro_input, parse_quote,
};

/// Makes a trait a loko service: adds `Has<Trait>` and implements the trait for every type that has
/// one, forwarding each method. Methods must take `&self`; the trait can't be generic.
#[proc_macro_attribute]
pub fn service(args: TokenStream, input: TokenStream) -> TokenStream {
    if !args.is_empty() {
        return Error::new(Span::call_site(), "#[service] takes no arguments").to_compile_error().into();
    }
    let item = parse_macro_input!(input as ItemTrait);
    match expand_service(&item) {
        Ok(extra) => quote! { #item #extra }.into(),
        Err(e) => {
            let e = e.to_compile_error();
            quote! { #item #e }.into()
        }
    }
}

/// The accessor of service `name`: `Clock` → `clock`, `ObjectStore` → `object_store`.
fn accessor(name: &Ident) -> Ident {
    let mut snake = String::new();
    for (i, c) in name.to_string().chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                snake.push('_');
            }
            snake.extend(c.to_lowercase());
        } else {
            snake.push(c);
        }
    }
    Ident::new(&snake, name.span())
}

fn expand_service(item: &ItemTrait) -> syn::Result<TokenStream2> {
    if !item.generics.params.is_empty() {
        return Err(Error::new(item.generics.span(), "a loko service can't be generic"));
    }
    if item.unsafety.is_some() {
        return Err(Error::new(item.span(), "a loko service can't be an unsafe trait"));
    }
    let vis = &item.vis;
    let name = &item.ident;
    let has = format_ident!("Has{}", name);
    let get = accessor(name);
    let supertraits = &item.supertraits;
    let held = quote! { <<T as #has>::#name as #name> };

    let mut items = vec![];
    for trait_item in &item.items {
        match trait_item {
            TraitItem::Type(t) => {
                let ident = &t.ident;
                let (_, ty_generics, where_clause) = t.generics.split_for_impl();
                items.push(quote! { type #ident #ty_generics = #held::#ident #ty_generics #where_clause; });
            }
            TraitItem::Const(c) => {
                let (ident, ty) = (&c.ident, &c.ty);
                items.push(quote! { const #ident: #ty = #held::#ident; });
            }
            TraitItem::Fn(f) => items.push(forward_fn(f, &held, &get)?),
            other => return Err(Error::new(other.span(), "loko services can hold types, constants, and methods")),
        }
    }

    let doc = format!("A type that holds a [`{name}`], and so is one (see `#[loko::service]`).");
    let get_doc = format!("The [`{name}`] this holds.");
    let bound = if supertraits.is_empty() {
        quote! {}
    } else {
        quote! { + #supertraits }
    };
    Ok(quote! {
        #[doc = #doc]
        #vis trait #has {
            /// The service held.
            type #name: #name;
            #[doc = #get_doc]
            fn #get(&self) -> &Self::#name;
        }

        #[automatically_derived]
        impl<T: #has #bound> #name for T {
            #(#items)*
        }
    })
}

/// A method of the blanket impl: the trait's signature, calling the held service's method.
fn forward_fn(f: &TraitItemFn, held: &TokenStream2, get: &Ident) -> syn::Result<TokenStream2> {
    let mut sig: Signature = f.sig.clone();
    match sig.inputs.first() {
        Some(FnArg::Receiver(r)) if r.reference.is_some() && r.mutability.is_none() => {}
        _ => return Err(Error::new(sig.span(), "loko service methods take `&self`")),
    }
    let mut args = vec![];
    for (i, input) in sig.inputs.iter_mut().skip(1).enumerate() {
        let FnArg::Typed(typed) = input else { continue };
        let ident = format_ident!("__loko_arg{}", i);
        *typed.pat =
            Pat::Ident(PatIdent { attrs: vec![], by_ref: None, mutability: None, ident: ident.clone(), subpat: None });
        args.push(ident);
    }
    let method = &sig.ident;
    let call = quote! { #held::#method(self.#get(), #(#args),*) };
    let body = if sig.asyncness.is_some() {
        quote! { #call.await }
    } else {
        call
    };
    Ok(match sig.output {
        ReturnType::Default => quote! { #[inline] #sig { #body; } },
        ReturnType::Type(..) => quote! { #[inline] #sig { #body } },
    })
}

/// Implements `Has<Service>` for each `#[service(Service, …)]` on a field, so the struct is each of
/// those services, forwarding to the field.
#[proc_macro_derive(Services, attributes(service))]
pub fn derive_services(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_services(&input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_services(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let Data::Struct(data) = &input.data else {
        return Err(Error::new(input.span(), "#[derive(Services)] is for structs"));
    };
    let name = &input.ident;
    let (impl_generics, ty_generics, _) = input.generics.split_for_impl();
    let mut impls = vec![];
    for (i, field) in data.fields.iter().enumerate() {
        let index = syn::Index::from(i);
        let access = field.ident.as_ref().map_or_else(|| quote! { #index }, |ident| quote! { #ident });
        let ty = &field.ty;
        for attr in field.attrs.iter().filter(|a| a.path().is_ident("service")) {
            let services = attr.parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)?;
            for service in services {
                let last = service.segments.last().ok_or_else(|| Error::new(service.span(), "a service path"))?;
                let service_name = last.ident.clone();
                let mut has = service.clone();
                if let Some(last) = has.segments.last_mut() {
                    last.ident = format_ident!("Has{}", service_name);
                }
                let get = accessor(&service_name);
                // A generic field type must be the service it provides.
                let mut generics = input.generics.clone();
                generics.make_where_clause().predicates.push(parse_quote! { #ty: #service });
                let where_clause = &generics.where_clause;
                impls.push(quote! {
                    #[automatically_derived]
                    impl #impl_generics #has for #name #ty_generics #where_clause {
                        type #service_name = #ty;
                        #[inline]
                        fn #get(&self) -> &#ty {
                            &self.#access
                        }
                    }
                });
            }
        }
    }
    if impls.is_empty() {
        return Err(Error::new(input.span(), "no field has #[service(…)]"));
    }
    Ok(quote! { #(#impls)* })
}

#[cfg(test)]
mod tests {
    use syn::parse::Parser;

    use super::*;

    #[test]
    fn accessors_are_snake_case() {
        assert_eq!(accessor(&format_ident!("Clock")), "clock");
        assert_eq!(accessor(&format_ident!("ObjectStore")), "object_store");
    }

    #[test]
    fn services_parse() {
        let parser = Punctuated::<Path, Token![,]>::parse_terminated;
        let paths = parser.parse_str("Clock, loko::Random").unwrap();
        assert_eq!(paths.len(), 2);
    }
}
