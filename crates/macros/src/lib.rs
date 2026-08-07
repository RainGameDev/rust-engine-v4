use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::{DeriveInput, Ident, parse_macro_input};

fn resolve_engine_core() -> proc_macro2::TokenStream {
    match crate_name("engine_core") {
        Ok(FoundCrate::Itself) => quote!(crate),
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!(::#ident)
        }
        Err(_) => quote!(::engine_core),
    }
}

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

                serialize_raw: |_| panic!("{} is not networked — cannot serialize", #name_str),
                deserialize_raw: |_| panic!("{} is not networked — cannot deserialize", #name_str),
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

// crates/macros/src/lib.rs

#[proc_macro_attribute]
pub fn update(_attr: TokenStream, item: TokenStream) -> TokenStream {
    system_attribute(item, "UpdateSystem", false)
}

#[proc_macro_attribute]
pub fn late_update(_attr: TokenStream, item: TokenStream) -> TokenStream {
    system_attribute(item, "LateUpdateSystem", false)
}

#[proc_macro_attribute]
pub fn fixed_update(_attr: TokenStream, item: TokenStream) -> TokenStream {
    system_attribute(item, "FixedUpdateSystem", true)
}

#[proc_macro_attribute]
pub fn start(_attr: TokenStream, item: TokenStream) -> TokenStream {
    system_attribute(item, "StartSystem", false)
}
fn system_attribute(item: TokenStream, kind: &str, takes_delta: bool) -> TokenStream {
    let input = parse_macro_input!(item as syn::ItemFn);
    let fn_ident = &input.sig.ident;
    let fn_name_str = fn_ident.to_string();
    let engine_core = resolve_engine_core();
    let kind_ident = syn::Ident::new(kind, proc_macro2::Span::call_site());
    let wrapper_ident = syn::Ident::new(&format!("__system_{}", fn_ident), fn_ident.span());

    let mut fetch_exprs = Vec::new();
    let mut seen_delta = false;

    for arg in &input.sig.inputs {
        let syn::FnArg::Typed(pat_type) = arg else {
            panic!("systems cannot take `self`");
        };
        let ty = &*pat_type.ty;
        let is_f32 = matches!(ty, syn::Type::Path(p) if p.path.is_ident("f32"));
        let is_commands_ref = matches!(
            ty,
            syn::Type::Reference(r) if matches!(
                &*r.elem,
                syn::Type::Path(p) if p.path.segments.last().map(|s| s.ident == "Commands").unwrap_or(false)
            )
        );

        if is_f32 && takes_delta {
            if seen_delta {
                panic!("only one f32 (delta) parameter is allowed per fixed_update system");
            }
            seen_delta = true;
            fetch_exprs.push(quote! { delta });
        } else if is_commands_ref {
            fetch_exprs.push(quote! { &mut commands });
        } else {
            fetch_exprs.push(quote! {
                <#ty as #engine_core::ecs::systems::param::SystemParam>::fetch(world_ref)?
            });
        }
    }

    if takes_delta && !seen_delta {
        panic!("#[fixed_update] systems must take a `delta: f32` parameter");
    }

    let wrapper_sig = if takes_delta {
        quote! { fn #wrapper_ident(world: &mut #engine_core::ecs::World, delta: f32) -> ::anyhow::Result<()> }
    } else {
        quote! { fn #wrapper_ident(world: &mut #engine_core::ecs::World) -> ::anyhow::Result<()> }
    };

    let expanded = quote! {
        #input

        #[doc(hidden)]
        #wrapper_sig {

            let mut commands = {
                let world_ref: &#engine_core::ecs::World = world;
                let mut commands = #engine_core::ecs::commands::Commands::new(world.entity_counter());
                #fn_ident(#(#fetch_exprs),*)?;
                commands
            };
            commands.apply(world);
            Ok(())
        }

        #engine_core::inventory::submit! {
            #engine_core::ecs::systems::#kind_ident {
                name: #fn_name_str,
                func: #wrapper_ident,
                priority: 0,
        }
        }
    };

    expanded.into()
}
#[proc_macro_attribute]
pub fn component(attr: TokenStream, item: TokenStream) -> TokenStream {
    let networked = attr.to_string().contains("networked"); 
    component_attribute(item, networked)
}

fn component_attribute(item: TokenStream, networked: bool) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let ident = &input.ident;
    let name_str = ident.to_string();
    let engine_core = resolve_engine_core();

    let derives = if networked {
        quote! { #[derive(Debug, Clone, ::serde::Serialize, ::serde::Deserialize)] }
    } else {
        quote! { #[derive(Debug, Clone)] }
    };

    let serialize_fields = if networked {
        quote! {
            serialize_raw: |ptr| {
                let concrete = unsafe { &*(ptr as *const #ident) };
                ::bincode::serialize(concrete).unwrap()
            },
            deserialize_raw: |bytes| Box::new(::bincode::deserialize::<#ident>(bytes).unwrap()),
        }
    } else {
        quote! {
            serialize_raw: |_| panic!("{} is not networked — cannot serialize", #name_str),
            deserialize_raw: |_| panic!("{} is not networked — cannot deserialize", #name_str),
        }
    };

    let expanded = quote! {
        #derives
        #input

        const _: fn() = || {
            fn assert_component<T: #engine_core::ecs::components::Component>() {}
            assert_component::<#ident>();
        };

        #engine_core::inventory::submit! {
            #engine_core::ecs::components::component_registry::ComponentRegistration {
                type_id: ::std::any::TypeId::of::<#ident>,
                type_name: #name_str,
                create_column: || #engine_core::ecs::components::archetype::Column::new::<#ident>(),
                #serialize_fields
            }
        }
    };

    expanded.into()
}
