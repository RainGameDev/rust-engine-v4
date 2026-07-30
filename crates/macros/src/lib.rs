use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::{DeriveInput, Ident, parse_macro_input};

#[proc_macro_derive(Component)]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = &input.ident;

    let engine_core = match crate_name("engine_core") {
        Ok(FoundCrate::Itself) => quote!(crate),
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!(::#ident)
        }
        Err(_) => quote!(::engine_core),
    };

    // let register_fn_name = Ident::new(&format!("__register_component_{}", ident), ident.span());
    let name_str = ident.to_string();

    let expanded = quote! {
        const _: fn() = || {
            fn assert_component<T: #engine_core::ecs::components::Component>() {}
            assert_component::<#ident>();
        };

        #engine_core::inventory::submit! {
            #engine_core::ecs::components::component_registry::ComponentRegistration {
                type_id: ::std::any::TypeId::of::<#ident>,
                type_name: #name_str,
                create_column: || #engine_core::ecs::components::archetype::Column::new::<#ident>(),
            }
        }
    };

    expanded.into()
}

#[proc_macro_derive(Resource)]
pub fn derive_resource(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = &input.ident;
    let name_str = ident.to_string();

    let engine_core = match crate_name("engine_core") {
        Ok(FoundCrate::Itself) => quote!(crate),
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!(::#ident)
        }
        Err(_) => quote!(::engine_core),
    };

    // let register_fn_name = Ident::new(&format!("__register_resource_{}", ident), ident.span());

    let expanded = quote! {
        const _: fn() = || {
            fn assert_resource<T: #engine_core::ecs::resources::Resource>() {}
            assert_resource::<#ident>();
        };

        #engine_core::inventory::submit! {
            #engine_core::ecs::resources::resource_registration::ResourceRegistration {
                type_id: ::std::any::TypeId::of::<#ident>,
                type_name: #name_str,
            }
        }
    };

    expanded.into()
}

#[proc_macro_derive(Asset)]
pub fn derive_asset(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let name_str = input.ident.to_string();

    let engine_core = match crate_name("engine_core") {
        Ok(FoundCrate::Itself) => quote!(crate),
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!(::#ident)
        }
        Err(_) => quote!(::engine_core),
    };

    let expanded = quote! {
        impl #impl_generics #engine_core::assets::Asset for #ident #ty_generics #where_clause {}

        #engine_core::inventory::submit! {
            #engine_core::assets::AssetRegistration{
                type_id: ::std::any::TypeId::of::<#ident>,
                type_name: #name_str,
        create_asset_map: || Box::new(#engine_core::assets::core::storage::AssetMap::<#ident>::default()),
            }
        }


    };

    TokenStream::from(expanded)
}
