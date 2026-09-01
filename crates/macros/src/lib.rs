use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::parse::Parser;
use syn::{Attribute, DeriveInput, Fields, Ident, ItemStruct, Lit, Meta, Type, parse_macro_input};

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

                serialize_raw: |_| panic!("{} is not networked - cannot serialize", #name_str),
                deserialize_raw: |_| panic!("{} is not networked - cannot deserialize", #name_str),
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
            serialize_raw: |_| panic!("{} is not networked - cannot serialize", #name_str),
            deserialize_raw: |_| panic!("{} is not networked - cannot deserialize", #name_str),
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

#[proc_macro_attribute]
pub fn resource(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr_str = attr.to_string();
    let networked = attr_str.contains("networked");
    let clone = attr_str.contains("clone") || networked;
    resource_attribute(item, networked, clone)
}

fn resource_attribute(item: TokenStream, networked: bool, clone: bool) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let ident = &input.ident;
    let name_str = ident.to_string();
    let engine_core = resolve_engine_core();

    let derives = match (networked, clone) {
        (true, _) => quote! { #[derive(Debug, Clone, ::serde::Serialize, ::serde::Deserialize)] },
        (false, true) => quote! { #[derive(Debug, Clone)] },
        (false, false) => quote! { #[derive(Debug)] },
    };

    let expanded = quote! {
        #derives
        #input

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

#[proc_macro_attribute]
pub fn inspectable(attr: TokenStream, item: TokenStream) -> TokenStream {
    let target_ty = parse_macro_input!(attr as syn::Path);
    let input = parse_macro_input!(item as syn::ItemFn);
    let fn_ident = &input.sig.ident;
    let engine_core = resolve_engine_core();

    let expanded = quote! {
        #input

        #engine_core::inventory::submit! {
            #engine_core::ecs::components::component_registry::ComponentInspector {
                type_id: ::std::any::TypeId::of::<#target_ty>,
                inspect: #fn_ident,
            }
        }
    };

    expanded.into()
}

#[proc_macro_attribute]
pub fn vertex(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemStruct);

    // optional `#[vertex(binding = 1, vertex_shader = "skin.vert.spv")]`
    let (binding, vertex_shader): (u32, Option<syn::LitStr>) = parse_vertex_args(&attr.into());

    let struct_name = input.ident.clone();

    let engine_core = resolve_engine_core();

    let Fields::Named(fields) = &mut input.fields else {
        return syn::Error::new_spanned(&input, "#[vertex] only supports named fields")
            .to_compile_error()
            .into();
    };

    let mut attr_descs = Vec::new();
    let mut location: u32 = 0;

    for field in fields.named.iter_mut() {
        let field_ident = field.ident.clone().unwrap();

        let format =
            format_override(&field.attrs).unwrap_or_else(|| infer_format(&field.ty, &field_ident));

        // strip #[format(...)] so it doesn't leak into the emitted struct
        field.attrs.retain(|a| !a.path().is_ident("format"));

        attr_descs.push(quote! {
            ::ash::vk::VertexInputAttributeDescription::default()
                .binding(#binding)
                .location(#location)
                .format(#format)
                .offset(::std::mem::offset_of!(#struct_name, #field_ident) as u32)
        });
        location += 1;
    }

    input.attrs.push(syn::parse_quote!(#[repr(C)]));
    input
        .attrs
        .push(syn::parse_quote!(#[derive(Clone, Copy, Debug)]));

    let vertex_shader_tokens = match &vertex_shader {
        Some(lit) => quote! { Some(#lit) },
        None => quote! { None },
    };

    let expanded = quote! {
        #input

        impl VertexDefinition for #struct_name {
            fn get_binding_description() -> ::ash::vk::VertexInputBindingDescription {
                ::ash::vk::VertexInputBindingDescription::default()
                    .binding(#binding)
                    .stride(::std::mem::size_of::<#struct_name>() as u32)
                    .input_rate(::ash::vk::VertexInputRate::VERTEX)
            }

            fn get_attribute_descriptions() -> Vec<::ash::vk::VertexInputAttributeDescription> {
                vec![ #(#attr_descs),* ]
            }

            fn vertex_type_name() -> &'static str {
                stringify!(#struct_name)
            }
        }

        ::inventory::submit! {
            #engine_core::rendering::core::vertex::VertexTypeInfo {
                name: stringify!(#struct_name),
                binding_description: <#struct_name as VertexDefinition>::get_binding_description,
                attribute_descriptions: <#struct_name as VertexDefinition>::get_attribute_descriptions,
                size: ::std::mem::size_of::<#struct_name>(),
                vertex_shader: #vertex_shader_tokens,
            }
        }
    };

    expanded.into()
}
/// Infer a vk::Format from a Rust field type.
fn infer_format(ty: &Type, field_ident: &syn::Ident) -> proc_macro2::TokenStream {
    let (base, len) = match ty {
        Type::Array(arr) => {
            let len = match &arr.len {
                syn::Expr::Lit(syn::ExprLit {
                    lit: Lit::Int(n), ..
                }) => n.base10_parse::<usize>().unwrap(),
                _ => panic!("array length must be a literal integer"),
            };
            (type_name(&arr.elem), len)
        }
        _ => (type_name(ty), 1),
    };

    let fmt = match (base.as_str(), len) {
        ("f32", 1) => "R32_SFLOAT",
        ("f32", 2) => "R32G32_SFLOAT",
        ("f32", 3) => "R32G32B32_SFLOAT",
        ("f32", 4) => "R32G32B32A32_SFLOAT",
        ("u32", 1) => "R32_UINT",
        ("u32", 2) => "R32G32_UINT",
        ("u32", 3) => "R32G32B32_UINT",
        ("u32", 4) => "R32G32B32A32_UINT",
        ("i32", 1) => "R32_SINT",
        ("i32", 2) => "R32G32_SINT",
        ("i32", 3) => "R32G32B32_SINT",
        ("i32", 4) => "R32G32B32A32_SINT",
        ("u16", 1) => "R16_UINT",
        ("u16", 2) => "R16G16_UINT",
        ("u16", 4) => "R16G16B16A16_UINT",
        (b, n) => panic!(
            "no default vk::Format for `{b}` x{n} on field `{field_ident}` - \
             add an explicit #[format(...)] override (e.g. R8G8B8A8_UNORM)"
        ),
    };

    let ident = syn::Ident::new(fmt, proc_macro2::Span::call_site());
    quote! { ::ash::vk::Format::#ident }
}
fn type_name(ty: &Type) -> String {
    match ty {
        Type::Path(p) => p.path.segments.last().unwrap().ident.to_string(),
        _ => panic!("unsupported field type"),
    }
}

/// Look for a `#[format(R16G16B16A16_UINT)]` override on a field.
fn format_override(attrs: &[Attribute]) -> Option<proc_macro2::TokenStream> {
    for attr in attrs {
        if attr.path().is_ident("format") {
            if let Meta::List(list) = &attr.meta {
                let ident: syn::Ident = list.parse_args().ok()?;
                return Some(quote! { ::ash::vk::Format::#ident });
            }
        }
    }
    None
}

/// Parses the `#[vertex(binding = N, vertex_shader = "x.spv")]` arguments.
fn parse_vertex_args(attr: &proc_macro2::TokenStream) -> (u32, Option<syn::LitStr>) {
    let mut binding = 0u32;
    let mut vertex_shader = None;

    if attr.is_empty() {
        return (binding, vertex_shader);
    }

    let metas = match syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated
        .parse2(attr.clone())
    {
        Ok(m) => m,
        Err(_) => return (binding, vertex_shader),
    };

    for meta in metas {
        if let syn::Meta::NameValue(nv) = meta {
            if nv.path.is_ident("binding") {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: Lit::Int(n), ..
                }) = nv.value
                {
                    binding = n.base10_parse().unwrap_or(0);
                }
            } else if nv.path.is_ident("vertex_shader") {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: Lit::Str(s), ..
                }) = nv.value
                {
                    vertex_shader = Some(s);
                }
            }
        }
    }

    (binding, vertex_shader)
}
