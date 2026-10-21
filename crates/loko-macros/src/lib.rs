//! loko's macros, used by its service crates and re-exported by `loko`.
//!
//! - `#[service]` on a trait `X` adds a trait `HasX` (a type that holds an `X`) and implements `X`
//!   for every `HasX`. Each method goes through a `HasX` method (`wrapped_<method>`) that forwards
//!   to the held service unless the implementation says otherwise.
//! - `#[bundle]` on a struct implements `HasX` for its fields marked `#[loko::has(X, …)]`, so the
//!   struct is every service its fields hold.
//! - `#[wraps(field: Type)]` on an `impl X for Wrapper` implements `HasX` instead: the methods
//!   written replace the forwarding ones, and the rest forward to the field.
//!
//! The `HasX` traits follow RIO's "Has" type classes, and choosing a field to forward to with an attribute
//! follows ambassador (`PRIOR_ART.md`).

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{ToTokens, format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;
use syn::{
    Error, FnArg, Ident, ImplItem, ItemImpl, ItemTrait, Member, Pat, PatIdent, Path, ReturnType, Signature, Token,
    TraitItem, TraitItemFn, Type, TypePath, parse_macro_input, parse_quote,
};

/// Makes a trait a loko service: adds `Has<Trait>` and implements the trait for every type that
/// has one. Methods must take `&self`, so one value can be several services and clones can share
/// state; an implementation whose state changes keeps it behind a lock or an atomic, never held
/// across `.await`. The trait can't be generic.
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

/// The accessor of service `name`: `Clock` → `clock`, `ObjectStore` → `object_store`,
/// `HTTPClient` → `http_client`. A word starts at a capital after a lowercase letter or digit, or at
/// the last capital of a run followed by a lowercase letter.
fn accessor(name: &Ident) -> Ident {
    let chars: Vec<char> = name.to_string().chars().collect();
    let mut snake = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_uppercase() && i > 0 {
            let after_lower = chars[i - 1].is_lowercase() || chars[i - 1].is_ascii_digit();
            let ends_run = chars[i - 1].is_uppercase() && chars.get(i + 1).is_some_and(|n| n.is_lowercase());
            if after_lower || ends_run {
                snake.push('_');
            }
        }
        snake.extend(c.to_lowercase());
    }
    Ident::new(&snake, name.span())
}

/// The `HasX` method behind method `method` of service `X`.
fn wrapped(method: &Ident) -> Ident {
    format_ident!("wrapped_{}", method)
}

/// `Has<last segment>`, at the same path: `loko::Clock` → `loko::HasClock`.
fn has_path(service: &Path) -> syn::Result<(Path, Ident)> {
    let name = service.segments.last().ok_or_else(|| Error::new(service.span(), "a service path"))?.ident.clone();
    let mut has = service.clone();
    if let Some(last) = has.segments.last_mut() {
        last.ident = format_ident!("Has{}", name);
        last.arguments = syn::PathArguments::None;
    }
    Ok((has, name))
}

/// Rewrites `Self::Assoc` to the held service's `Assoc`, for signatures moved into `HasX`, where
/// `Self` is the holder. `Self::X` (the `HasX` type itself) stays.
struct HeldAssoc<'a> {
    has: &'a Path,
    name: &'a Ident,
}

impl VisitMut for HeldAssoc<'_> {
    fn visit_type_path_mut(&mut self, path: &mut TypePath) {
        let segments = &path.path.segments;
        if path.qself.is_none()
            && segments.len() >= 2
            && segments[0].ident == "Self"
            && segments[0].arguments.is_none()
            && segments[1].ident != *self.name
        {
            let (has, name) = (self.has, self.name);
            let rest: Vec<_> = segments.iter().skip(1).collect();
            *path = parse_quote! { <<Self as #has>::#name as #name>::#(#rest)::* };
        }
        syn::visit_mut::visit_type_path_mut(self, path);
    }
}

/// Finds `Self` used as a type on its own (`-> Self`, `Option<Self>`), not as `Self::Assoc`.
#[derive(Default)]
struct BareSelf(Option<Span>);

impl syn::visit::Visit<'_> for BareSelf {
    fn visit_type_path(&mut self, path: &TypePath) {
        if path.qself.is_none() && path.path.segments.len() == 1 && path.path.segments[0].ident == "Self" {
            self.0.get_or_insert(path.span());
        }
        syn::visit::visit_type_path(self, path);
    }
}

/// An error if `sig` uses `Self` as a type: the holder of a service isn't the service held, so a
/// forwarded `Self` would be the wrong type.
fn reject_bare_self(sig: &Signature) -> syn::Result<()> {
    let mut found = BareSelf::default();
    for input in sig.inputs.iter().skip(1) {
        syn::visit::Visit::visit_fn_arg(&mut found, input);
    }
    syn::visit::Visit::visit_return_type(&mut found, &sig.output);
    match found.0 {
        Some(span) => Err(Error::new(
            span,
            "loko service methods can't take or return `Self`: a type holding the service isn't the service",
        )),
        None => Ok(()),
    }
}

/// Names the arguments of `sig` (after `&self`) `arg0`, `arg1`, …, so a body can pass them on.
fn name_arguments(sig: &mut Signature) -> syn::Result<Vec<Ident>> {
    match sig.inputs.first() {
        Some(FnArg::Receiver(r)) if r.reference.is_some() && r.mutability.is_none() => {}
        _ => return Err(Error::new(sig.span(), "loko service methods take `&self`")),
    }
    let mut args = vec![];
    for (i, input) in sig.inputs.iter_mut().skip(1).enumerate() {
        let FnArg::Typed(typed) = input else { continue };
        let ident = format_ident!("arg{}", i);
        *typed.pat =
            Pat::Ident(PatIdent { attrs: vec![], by_ref: None, mutability: None, ident: ident.clone(), subpat: None });
        args.push(ident);
    }
    Ok(args)
}

/// The `#[cfg]` and `#[cfg_attr]` attributes of a trait item, which its generated items need too.
fn cfgs(attrs: &[syn::Attribute]) -> Vec<&syn::Attribute> {
    attrs.iter().filter(|a| a.path().is_ident("cfg") || a.path().is_ident("cfg_attr")).collect()
}

/// `sig { call }`, awaiting the call in an async method.
fn method(sig: &Signature, call: &TokenStream2) -> TokenStream2 {
    let body = if sig.asyncness.is_some() {
        quote! { #call.await }
    } else {
        call.clone()
    };
    match sig.output {
        ReturnType::Default => quote! { #[inline] #sig { #body; } },
        ReturnType::Type(..) => quote! { #[inline] #sig { #body } },
    }
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
    let has: Path = format_ident!("Has{}", name).into();
    let get = accessor(name);
    // A name for the blanket impl's type that a method's own generics won't use.
    let holder = format_ident!("Loko{}Holder", name);
    let held = quote! { <<#holder as #has>::#name as #name> };

    let (mut has_items, mut impl_items) = (vec![], vec![]);
    for trait_item in &item.items {
        match trait_item {
            TraitItem::Type(t) => {
                let (ident, cfgs) = (&t.ident, cfgs(&t.attrs));
                let (_, ty_generics, where_clause) = t.generics.split_for_impl();
                impl_items
                    .push(quote! { #(#cfgs)* type #ident #ty_generics = #held::#ident #ty_generics #where_clause; });
            }
            TraitItem::Const(c) => {
                let (ident, ty, cfgs) = (&c.ident, &c.ty, cfgs(&c.attrs));
                impl_items.push(quote! { #(#cfgs)* const #ident: #ty = #held::#ident; });
            }
            TraitItem::Fn(f) => {
                let (has_item, impl_item) = forward_fn(f, &has, name, &get, &holder)?;
                has_items.push(has_item);
                impl_items.push(impl_item);
            }
            other => return Err(Error::new(other.span(), "loko services can hold types, constants, and methods")),
        }
    }

    let doc = format!(
        "A type that holds a [`{name}`], and so is one: `#[loko::bundle]` implements this for a field, \
         and `#[loko::wraps]` for a wrapper that changes some methods."
    );
    let get_doc = format!("The [`{name}`] this holds.");
    let supertraits = &item.supertraits;
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
            #(#has_items)*
        }

        #[automatically_derived]
        impl<#holder: #has #bound> #name for #holder {
            #(#impl_items)*
        }
    })
}

/// For a method of the service: its forwarding method in `HasX` (`wrapped_<method>`, calling the
/// held service), and its method in the blanket impl (calling the `HasX` one).
fn forward_fn(
    f: &TraitItemFn,
    has: &Path,
    name: &Ident,
    get: &Ident,
    holder: &Ident,
) -> syn::Result<(TokenStream2, TokenStream2)> {
    reject_bare_self(&f.sig)?;
    let mut sig = f.sig.clone();
    let args = name_arguments(&mut sig)?;
    let cfgs = cfgs(&f.attrs);
    let ident = sig.ident.clone();
    let wrapped_ident = wrapped(&ident);

    let mut has_sig = sig.clone();
    has_sig.ident = wrapped_ident.clone();
    HeldAssoc { has, name }.visit_signature_mut(&mut has_sig);
    let has_call = quote! { <<Self as #has>::#name as #name>::#ident(#has::#get(self), #(#args),*) };
    let doc =
        format!("Forwards [`{name}::{ident}`] to the held service; a `#[loko::wraps]` implementation may replace it.");
    let has_method = method(&has_sig, &has_call);

    let impl_call = quote! { <#holder as #has>::#wrapped_ident(self, #(#args),*) };
    let impl_method = method(&sig, &impl_call);
    Ok((quote! { #(#cfgs)* #[doc = #doc] #[doc(hidden)] #has_method }, quote! { #(#cfgs)* #impl_method }))
}

/// Whether `attr` is `#[loko::has(…)]` (or `#[has(…)]`, or a re-export's path ending in `has`).
fn is_has(attr: &syn::Attribute) -> bool {
    attr.path().segments.last().is_some_and(|s| s.ident == "has")
}

/// Makes a struct a bundle: for each field marked `#[loko::has(Service, …)]`, implements
/// `Has<Service>`, so the struct is each of those services, forwarding to the field.
#[proc_macro_attribute]
pub fn bundle(args: TokenStream, input: TokenStream) -> TokenStream {
    if !args.is_empty() {
        return Error::new(Span::call_site(), "#[bundle] takes no arguments").to_compile_error().into();
    }
    let mut item = parse_macro_input!(input as syn::ItemStruct);
    match expand_bundle(&mut item) {
        Ok(impls) => quote! { #item #impls }.into(),
        Err(e) => {
            let e = e.to_compile_error();
            quote! { #item #e }.into()
        }
    }
}

/// The `Has` impls of `item`'s `#[has(…)]` fields, which it removes from the struct.
fn expand_bundle(item: &mut syn::ItemStruct) -> syn::Result<TokenStream2> {
    let name = item.ident.clone();
    let generics = item.generics.clone();
    let (impl_generics, ty_generics, _) = generics.split_for_impl();
    let mut impls = vec![];
    for (i, field) in item.fields.iter_mut().enumerate() {
        let index = syn::Index::from(i);
        let access = field.ident.as_ref().map_or_else(|| quote! { #index }, |ident| quote! { #ident });
        let ty = field.ty.clone();
        let (has_attrs, others): (Vec<_>, Vec<_>) = field.attrs.drain(..).partition(is_has);
        field.attrs = others;
        for attr in has_attrs {
            let services = attr.parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)?;
            for service in services {
                let (has, service_name) = has_path(&service)?;
                let get = accessor(&service_name);
                // A generic field type must be the service it holds.
                let mut generics = generics.clone();
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
        return Err(Error::new(name.span(), "no field has #[loko::has(…)]"));
    }
    Ok(quote! { #(#impls)* })
}

/// Marks a field of a `#[loko::bundle]` struct with the services it holds:
/// `#[loko::has(Clock, Random)]`. `#[loko::bundle]` reads and removes it; anywhere else it's an
/// error.
#[proc_macro_attribute]
pub fn has(_args: TokenStream, input: TokenStream) -> TokenStream {
    let e =
        Error::new(Span::call_site(), "#[loko::has(…)] goes on a field of a #[loko::bundle] struct").to_compile_error();
    let input = TokenStream2::from(input);
    quote! { #e #input }.into()
}

/// `field: Type`, the arguments of `#[wraps]`.
struct WrapsArgs {
    field: Member,
    ty: Type,
}

impl Parse for WrapsArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let field = input.parse()?;
        input.parse::<Token![:]>()?;
        Ok(Self { field, ty: input.parse()? })
    }
}

/// Makes `impl Service for Wrapper` a wrapper around a field: `#[wraps(inner: S)]`. The methods
/// written replace the service's; the others forward to `self.inner`. A wrapper keeps the inner
/// service's associated types.
#[proc_macro_attribute]
pub fn wraps(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as WrapsArgs);
    let item = parse_macro_input!(input as ItemImpl);
    match expand_wraps(&args, &item) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_wraps(args: &WrapsArgs, item: &ItemImpl) -> syn::Result<TokenStream2> {
    let Some((None, service, _)) = &item.trait_ else {
        return Err(Error::new(item.span(), "#[wraps] goes on an `impl Service for Wrapper`"));
    };
    let (has, name) = has_path(service)?;
    let get = accessor(&name);
    let mut methods = vec![];
    for impl_item in &item.items {
        let ImplItem::Fn(f) = impl_item else {
            return Err(Error::new(
                impl_item.span(),
                "a wrapper only replaces methods; it keeps the inner service's types and constants",
            ));
        };
        let mut f = f.clone();
        f.sig.ident = wrapped(&f.sig.ident);
        HeldAssoc { has: &has, name: &name }.visit_signature_mut(&mut f.sig);
        methods.push(f.into_token_stream());
    }
    let (impl_generics, _, where_clause) = item.generics.split_for_impl();
    let (attrs, self_ty, (field, ty)) = (&item.attrs, &item.self_ty, (&args.field, &args.ty));
    Ok(quote! {
        #(#attrs)*
        impl #impl_generics #has for #self_ty #where_clause {
            type #name = #ty;
            #[inline]
            fn #get(&self) -> &#ty {
                &self.#field
            }
            #(#methods)*
        }
    })
}

#[cfg(test)]
mod tests {
    use syn::parse::Parser;

    use super::*;

    #[test]
    fn accessors_are_snake_case() {
        assert_eq!(accessor(&format_ident!("Clock")), "clock");
        assert_eq!(accessor(&format_ident!("ObjectStore")), "object_store");
        assert_eq!(accessor(&format_ident!("HTTPClient")), "http_client");
        assert_eq!(accessor(&format_ident!("S3Store")), "s3_store");
        assert_eq!(accessor(&format_ident!("IOError")), "io_error");
    }

    #[test]
    fn services_parse() {
        let parser = Punctuated::<Path, Token![,]>::parse_terminated;
        let paths = parser.parse_str("Clock, loko::Random").unwrap();
        assert_eq!(paths.len(), 2);
        let (has, name) = has_path(&paths[1]).unwrap();
        assert_eq!(
            (has.to_token_stream().to_string(), name.to_string()),
            ("loko :: HasRandom".into(), "Random".into())
        );
    }

    #[test]
    fn bare_self_is_rejected() {
        for sig in
            [quote! { fn duplicate(&self) -> Self where Self: Sized }, quote! { fn merge(&self, other: Option<Self>) }]
        {
            assert!(reject_bare_self(&syn::parse2(sig).unwrap()).is_err());
        }
        let ok: Signature = parse_quote! { fn upload(&self) -> Result<Self::Upload> where Self: Sized };
        assert!(reject_bare_self(&ok).is_ok(), "associated types and where clauses are fine");
    }

    #[test]
    fn self_types_become_the_held_services() {
        let mut sig: Signature = parse_quote! { fn upload(&self) -> Result<Self::Upload> };
        let has: Path = parse_quote! { HasStore };
        HeldAssoc { has: &has, name: &format_ident!("Store") }.visit_signature_mut(&mut sig);
        assert_eq!(
            sig.output.to_token_stream().to_string(),
            "-> Result < < < Self as HasStore > :: Store as Store > :: Upload >"
        );
    }
}
