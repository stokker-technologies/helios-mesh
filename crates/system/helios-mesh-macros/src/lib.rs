/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Compile-time node and route authoring for the Helios Mesh facade.

/* === Modules === */

mod channels;

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, ImplItem, ItemEnum, ItemImpl, parse_macro_input};

// Local Imports

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

#[proc_macro_attribute]
pub fn node_catalog(args: TokenStream, item: TokenStream) -> TokenStream {
    if !args.is_empty() {
        return syn::Error::new(proc_macro2::Span::call_site(), "this attribute takes no arguments").into_compile_error().into();
    }
    let catalog = parse_macro_input!(item as ItemEnum);
    expand_catalog(&catalog).unwrap_or_else(syn::Error::into_compile_error).into()
}

#[proc_macro_attribute]
pub fn node(args: TokenStream, item: TokenStream) -> TokenStream {
    if !args.is_empty() {
        return syn::Error::new(proc_macro2::Span::call_site(), "this attribute takes no arguments").into_compile_error().into();
    }
    let item = parse_macro_input!(item as syn::Item);
    let result = match item {
        syn::Item::Struct(node) => channels::expand(&node),
        syn::Item::Impl(implementation) => expand_node(implementation),
        other => Err(syn::Error::new_spanned(other, "expected a node struct or impl of helios_mesh::Node")),
    };
    result.unwrap_or_else(syn::Error::into_compile_error).into()
}

#[proc_macro_attribute]
pub fn route(_args: TokenStream, item: TokenStream) -> TokenStream {
    syn::Error::new_spanned(proc_macro2::TokenStream::from(item), "place routes inside an #[helios_mesh::node] impl")
        .into_compile_error()
        .into()
}

#[proc_macro_attribute]
pub fn jobs(args: TokenStream, item: TokenStream) -> TokenStream {
    if !args.is_empty() {
        return syn::Error::new(proc_macro2::Span::call_site(), "this attribute takes no arguments").into_compile_error().into();
    }
    let catalog = parse_macro_input!(item as ItemEnum);
    expand_jobs(&catalog).unwrap_or_else(syn::Error::into_compile_error).into()
}

/* === Private Functions === */

fn module_name(name: &str) -> String {
    let chars: Vec<_> = name.chars().collect();
    let mut result = String::new();
    for (index, character) in chars.iter().copied().enumerate() {
        if character.is_uppercase()
            && index > 0
            && (chars[index - 1].is_lowercase()
                || chars[index - 1].is_numeric()
                || chars.get(index + 1).is_some_and(|next| next.is_lowercase()))
        {
            result.push('_');
        }
        result.extend(character.to_lowercase());
    }
    result
}

fn expand_jobs(catalog: &ItemEnum) -> syn::Result<proc_macro2::TokenStream> {
    if !catalog.generics.params.is_empty() || catalog.variants.is_empty() {
        return Err(syn::Error::new_spanned(catalog, "a job catalog needs concrete job variants"));
    }
    let mut variants = Vec::new();
    let mut types = Vec::new();
    let mut payloads = std::collections::BTreeSet::new();
    for variant in &catalog.variants {
        let syn::Fields::Unnamed(fields) = &variant.fields else {
            return Err(syn::Error::new_spanned(variant, "expected Variant(JobType)"));
        };
        if fields.unnamed.len() != 1 {
            return Err(syn::Error::new_spanned(variant, "expected one job type"));
        }
        let ty = &fields.unnamed[0].ty;
        if !payloads.insert(quote!(#ty).to_string()) {
            return Err(syn::Error::new_spanned(
                ty,
                "duplicate job payload type: automatic conversion requires one variant per payload type; use distinct wrapper types",
            ));
        }

        variants.push(&variant.ident);
        types.push(ty);
    }
    let name = &catalog.ident;
    let first = types[0];
    let modules: Vec<_> = variants.iter().map(|variant| format_ident!("{}", module_name(&variant.to_string()))).collect();
    Ok(quote! {
        #(mod #modules;)*
        #(pub use #modules::#types;)*
        #catalog

        #(
            impl From<#types> for #name {
                fn from(job: #types) -> Self { Self::#variants(job) }
            }
        )*

        impl helios_mesh::Job for #name {
            type Resources = <#first as helios_mesh::Job>::Resources;
            fn timeout(&self) -> Option<helios_mesh::Duration> {
                match self { #(Self::#variants(job) => helios_mesh::Job::timeout(job)),* }
            }
            fn initialize(&self, resources: &Self::Resources) -> Result<(), helios_mesh::RequestError> {
                match self { #(Self::#variants(job) => helios_mesh::Job::initialize(job, resources)),* }
            }
            async fn run(&mut self, resources: &mut Self::Resources, cx: &mut helios_mesh::JobContext) -> Result<(), helios_mesh::RequestError> {
                match self { #(Self::#variants(job) => helios_mesh::Job::run(job, resources, cx).await),* }
            }
            async fn cleanup(&mut self, resources: &mut Self::Resources, outcome: &Result<(), helios_mesh::JobError>) -> Result<(), helios_mesh::RequestError> {
                match self { #(Self::#variants(job) => helios_mesh::Job::cleanup(job, resources, outcome).await),* }
            }
        }
    })
}

fn expand_node(mut implementation: ItemImpl) -> syn::Result<proc_macro2::TokenStream> {
    if implementation.trait_.is_none() {
        return Err(syn::Error::new_spanned(&implementation.self_ty, "expected an impl of helios_mesh::Node"));
    }
    let mut routes = Vec::new();
    let mut types = std::collections::BTreeSet::new();
    let mut retained = Vec::new();
    for item in implementation.items {
        if let ImplItem::Fn(mut function) = item {
            if function.attrs.iter().any(|attr| attr.path().segments.last().is_some_and(|segment| segment.ident == "route")) {
                function.attrs.retain(|attr| attr.path().segments.last().is_none_or(|segment| segment.ident != "route"));
                if function.sig.asyncness.is_none() || function.sig.inputs.len() != 3 || !function.sig.generics.params.is_empty() {
                    return Err(syn::Error::new_spanned(
                        &function.sig,
                        "route signature: async fn name(&mut self, cx: &mut Context<'_, Self>, request: Payload) -> Result<Response, RequestError>",
                    ));
                }
                let Some(FnArg::Typed(argument)) = function.sig.inputs.last() else { unreachable!() };
                let ty = &argument.ty;
                if let syn::Type::Path(path) = ty.as_ref() {
                    let leaf = path.path.segments.last().map(|segment| segment.ident.to_string()).unwrap_or_default();
                    if matches!(
                        leaf.as_str(),
                        "Describe"
                            | "Status"
                            | "Identify"
                            | "ReadDiagnostics"
                            | "ReadJournal"
                            | "OperationStatus"
                            | "CancelOperation"
                            | "Restart"
                            | "Poll"
                    ) {
                        return Err(syn::Error::new_spanned(
                            ty,
                            "shared requests are owned by the runtime; use middleware to observe them",
                        ));
                    }
                }
                if !types.insert(quote!(#ty).to_string()) {
                    return Err(syn::Error::new_spanned(ty, "duplicate request route"));
                }
                routes.push(function);
            } else {
                retained.push(ImplItem::Fn(function));
            }
        } else {
            retained.push(item);
        }
    }
    implementation.items = retained;
    let dispatch = routes.iter().map(|route| {
        let name = &route.sig.ident;
        let Some(FnArg::Typed(argument)) = route.sig.inputs.last() else { unreachable!() };
        let ty = &argument.ty;
        quote! {
            let request = match <#ty as core::convert::TryFrom<helios_mesh::Request>>::try_from(request) {
                Ok(payload) => return helios_mesh::Outcome::reply(match self.#name(cx, payload).await {
                    Ok(response) => helios_mesh::Response::from(response),
                    Err(error) => helios_mesh::Response::from(error),
                }),
                Err(request) => request,
            };
        }
    });
    implementation.items.push(syn::parse_quote! { type Request = helios_mesh::Request; });
    if routes.is_empty() {
        implementation.items.push(syn::parse_quote! {
            fn handle_request(&mut self, _cx: &mut helios_mesh::Context<'_, Self>, _request: Self::Request) -> impl core::future::Future<Output = helios_mesh::Outcome> {
                core::future::ready(helios_mesh::Outcome::reply(helios_mesh::RequestError::Unsupported))
            }
        });
    } else {
        implementation.items.push(syn::parse_quote! {
        async fn handle_request(&mut self, cx: &mut helios_mesh::Context<'_, Self>, request: Self::Request) -> helios_mesh::Outcome {
            #(#dispatch)*
            let _ = request;
            helios_mesh::Outcome::reply(helios_mesh::RequestError::Unsupported)
        }
        });
    }
    if !implementation.items.iter().any(|item| matches!(item, ImplItem::Fn(function) if function.sig.ident == "channels")) {
        implementation.items.push(syn::parse_quote! {
            fn channels(&self) -> impl Iterator<Item = helios_mesh::ChannelRef<'_>> {
                self.__helios_mesh_channels()
            }
        });
    }
    let self_type = &implementation.self_ty;
    let (generics, _, where_clause) = implementation.generics.split_for_impl();
    Ok(quote! {
        #implementation
        impl #generics #self_type #where_clause { #(#routes)* }
    })
}

fn expand_catalog(catalog: &ItemEnum) -> syn::Result<proc_macro2::TokenStream> {
    let name = &catalog.ident;
    if !catalog.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(&catalog.generics, "node catalogs cannot be generic"));
    }
    for variant in &catalog.variants {
        if !matches!(variant.fields, syn::Fields::Unit) || variant.discriminant.is_some() {
            return Err(syn::Error::new_spanned(variant, "node catalog entries must be unit variants"));
        }
    }
    let variants: Vec<_> = catalog.variants.iter().map(|variant| &variant.ident).collect();
    let modules: Vec<_> = variants.iter().map(|name| format_ident!("{}", module_name(&name.to_string()))).collect();
    let count = variants.len();
    if count == 0 {
        return Err(syn::Error::new_spanned(name, "a node catalog cannot be empty"));
    }
    Ok(quote! {
        #(mod #modules;)*
        #catalog
        impl #name {
                        pub const ALL: [Self; #count] = [#(Self::#variants),*];
                        #[must_use]
            pub const fn definition(self) -> &'static NodeDefinition {
                match self { #(Self::#variants => &#modules::DEFINITION),* }
            }
                        #[must_use]
            pub fn from_identifier(identifier: &str) -> Option<Self> {
                Self::ALL.into_iter().find(|kind| kind.definition().identifier == identifier)
            }
        }
        const _: () = {
            let kinds = #name::ALL;
            let mut i = 0;
            while i < kinds.len() {
                let id = kinds[i].definition().identifier.as_bytes();
                assert!(!id.is_empty() && id.len() <= 24, "node identifiers must have 1..=24 bytes");
                assert!(!kinds[i].definition().label.is_empty() && kinds[i].definition().label.len() <= 24, "node labels must have 1..=24 bytes");
                let mut j = 0;
                while j < i {
                    let other = kinds[j].definition().identifier.as_bytes();
                    let mut same = id.len() == other.len();
                    let mut k = 0;
                    while same && k < id.len() { same = id[k] == other[k]; k += 1; }
                    assert!(!same, "duplicate node identifier");
                    j += 1;
                }
                i += 1;
            }
        };
    })
}

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_names_jobs() {
        assert_eq!(module_name("TurnTo"), "turn_to");
        assert_eq!(module_name("USBRelay"), "usb_relay");
    }

    #[test]
    fn rejects_duplicate_handlers() {
        let duplicate: ItemImpl = syn::parse_quote! {
            impl helios_mesh::Node for Example {
                #[helios_mesh::route]
                async fn first(&mut self, cx: &mut helios_mesh::Context<'_, Self>, request: helios_mesh::PulseAuxGpio) -> Result<helios_mesh::Response, helios_mesh::RequestError> { unreachable!() }
                #[helios_mesh::route]
                async fn second(&mut self, cx: &mut helios_mesh::Context<'_, Self>, request: helios_mesh::PulseAuxGpio) -> Result<helios_mesh::Response, helios_mesh::RequestError> { unreachable!() }
            }
        };
        assert!(expand_node(duplicate).unwrap_err().to_string().contains("duplicate request route"));
        let synchronous: ItemImpl = syn::parse_quote! {
            impl helios_mesh::Node for Example {
                #[helios_mesh::route]
                fn pulse(&mut self, cx: &mut helios_mesh::Context<'_, Self>, request: helios_mesh::PulseAuxGpio) {}
            }
        };
        assert!(expand_node(synchronous).is_err());
    }

    #[test]
    fn rejects_ambiguous_conversions() {
        let duplicate: ItemEnum = syn::parse_quote! {
            enum Jobs { First(Pulse), Second(Pulse) }
        };
        let error = expand_jobs(&duplicate).unwrap_err();
        assert!(error.to_string().contains("duplicate job payload type"));
        assert!(error.to_string().contains("distinct wrapper types"));
    }

    #[test]
    fn rejects_invalid_entries() {
        let invalid: ItemEnum = syn::parse_quote! { enum Nodes { Sidecar(u8) } };
        assert!(expand_catalog(&invalid).is_err());
        let empty: ItemEnum = syn::parse_quote! { enum Jobs {} };
        assert!(expand_jobs(&empty).is_err());
        let invalid: ItemEnum = syn::parse_quote! { enum Jobs { Pulse } };
        assert!(expand_jobs(&invalid).is_err());
    }
}
