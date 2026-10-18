/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Node Channel Storage And Task Generation

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use quote::{format_ident, quote};
use syn::{Fields, GenericArgument, ItemStruct, PathArguments, Type};

// Local Imports
use crate::module_name;

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

pub(crate) fn expand(node: &ItemStruct) -> syn::Result<proc_macro2::TokenStream> {
    if !node.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(&node.generics, "node storage requires a concrete struct"));
    }
    let fields = match &node.fields {
        Fields::Named(fields) => fields.named.iter().collect::<Vec<_>>(),
        Fields::Unit => Vec::new(),
        Fields::Unnamed(_) => return Err(syn::Error::new_spanned(node, "use named Channel<Jobs, Resources> fields")),
    };
    let mut names = Vec::new();
    let mut types = Vec::new();
    let mut arguments = Vec::new();
    let mut inputs = Vec::new();
    let mut all_names = Vec::new();
    for field in fields {
        let field_name = field.ident.as_ref().expect("named field");
        all_names.push(field_name);
        let segment = match &field.ty {
            Type::Path(path) => path.path.segments.last().filter(|segment| segment.ident == "Channel"),
            _ => None,
        };
        let Some(segment) = segment else {
            let ty = &field.ty;
            inputs.push(quote!(#field_name: #ty));
            continue;
        };
        let PathArguments::AngleBracketed(args) = &segment.arguments else {
            return Err(syn::Error::new_spanned(field, "expected Channel<Jobs, Resources>"));
        };
        if !(2..=4).contains(&args.args.len()) || !matches!(args.args.first(), Some(GenericArgument::Type(_))) {
            return Err(syn::Error::new_spanned(
                args,
                "expected Channel<Jobs, Resources, optional queue capacity, optional history capacity>",
            ));
        }
        let Some(GenericArgument::Type(resource)) = args.args.iter().nth(1) else {
            return Err(syn::Error::new_spanned(args, "expected the channel's resource type"));
        };
        names.push(field_name);
        types.push(&field.ty);
        arguments.push(args);
        inputs.push(quote!(#field_name: impl Into<Option<#resource>>));
    }
    if names.len() > usize::from(u8::MAX) {
        return Err(syn::Error::new_spanned(node, "at most 255 channels per node"));
    }
    let name = &node.ident;
    let visibility = &node.vis;
    let slots: Vec<_> = names.iter().map(|field| format_ident!("__HNM_{}", field.to_string().to_uppercase())).collect();
    let tasks: Vec<_> = names.iter().map(|field| format_ident!("__hnm_{}_{}", module_name(&name.to_string()), field)).collect();
    let ids: Vec<u8> = (1..=names.len()).map(|index| u8::try_from(index).expect("channel count checked")).collect();
    let constructor = if matches!(node.fields, Fields::Unit) { quote!(Self) } else { quote!(Self { #(#all_names),* }) };
    Ok(quote! {
        #node

        impl #name {
                                                #[cfg(target_arch = "arm")]
            #visibility fn start(
                __hnm_spawner: hnm::__private::embassy_executor::Spawner,
                #(#inputs),*
            ) -> Self {
                #(
                    static #slots: hnm::__private::StaticCell<hnm::__private::ChannelStorage #arguments> = hnm::__private::StaticCell::new();
                    let #names = #slots.init(hnm::__private::ChannelStorage::new(#ids, #names.into())).channel();
                    __hnm_spawner.spawn(#tasks(#names).expect("node channel task started once"));
                )*
                let _ = __hnm_spawner;
                #constructor
            }

            fn __hnm_channels(&self) -> impl Iterator<Item = hnm::ChannelRef<'_>> {
                [#(hnm::ChannelRef::new(&self.#names, stringify!(#names))),*].into_iter()
            }
        }

        #(
            #[cfg(target_arch = "arm")]
            #[hnm::__private::embassy_executor::task(embassy_executor = hnm::__private::embassy_executor)]
            async fn #tasks(channel: #types) -> ! { channel.run().await }
        )*
    })
}

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_storage_channels() {
        let node = syn::parse_quote! {
            pub struct Sidecar {
                channel_1: hnm::Channel<Jobs, Pin>,
                channel_2: hnm::Channel<Jobs, Pin, 2, 8>,
                mode: u8,
            }
        };
        let output = expand(&node).unwrap();
        syn::parse2::<syn::File>(output.clone()).unwrap();
        let text = output.to_string();
        assert!(text.contains("__hnm_sidecar_channel_1"));
        assert!(text.contains("ChannelStorage < Jobs , Pin , 2 , 8 >"));
        assert!(text.contains("__hnm_channels"));
        assert!(text.contains("mode : u8"));
        assert!(text.contains("Self { channel_1 , channel_2 , mode }"));
    }

    #[test]
    fn rejects_ambiguous_declarations() {
        for node in [
            syn::parse_quote! { struct Node<T> { channel: Channel<Jobs, T> } },
            syn::parse_quote! { struct Node(Channel<Jobs, Pin>); },
            syn::parse_quote! { struct Node { channel: Channel<Jobs, 7> } },
            syn::parse_quote! { struct Node { channel: Channel<Jobs> } },
        ] {
            assert!(expand(&node).is_err());
        }
    }
}
