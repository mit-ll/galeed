extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

use quote::{quote, format_ident};
use syn::{parse_macro_input};

// TODO - going to use DeriveInput for now, but ideally should be able to modify to take multiple structs at once
//        writing modular code to facilitate this transition later

#[proc_macro_attribute]
pub fn fakify_proc(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let _args = parse_macro_input!(attr as syn::AttributeArgs);
    let input = parse_macro_input!(item as syn::DeriveInput);

    // Build the trait implementation
    fakify_macro(&input)
}

fn fakify_macro(ast: &syn::DeriveInput) -> TokenStream {
    let static_map = quote_static_map(ast);
    let struct_definition = quote_struct(ast);

    let final_result = quote! {
        #static_map
        #struct_definition
    };
    final_result.into()
}

fn quote_struct(ast: &syn::DeriveInput) -> TokenStream2 {
    let struct_name = make_struct_name(&ast.ident);
    let map_name = make_map_name(&ast.ident);
    let fields = match &ast.data {
        syn::Data::Struct(struct_data) => {
            &struct_data.fields
        },
        _ => panic!("TODO - invalid for enums and union types") 
    };
    let struct_fields = fields.iter().map(quote_struct_field);
    let to_fake_ptr = quote_to_fake_ptr(ast);
    let recover = quote_recover(ast);
    let getters = fields.iter().enumerate().map(|(i, f)| {
        quote_get(&struct_name, &map_name, f, i)
    });
    let setters = fields.iter().enumerate().map(|(i, f)| {
        quote_set(&struct_name, &map_name, f, i)
    });
    quote! {
        #[repr(C)]
        #[derive(Debug)]
        pub struct #struct_name {
            #(#struct_fields)*
        }

        impl #struct_name {
            #to_fake_ptr
            #recover
        }

        #(#getters)*

        #(#setters)*
    }
}

fn make_struct_name(name: &syn::Ident) -> syn::Ident {
    format_ident!("{}", name)
}

fn make_map_name(name: &syn::Ident) -> syn::Ident {
    format_ident!("FFIStructMap{}", name)
}

fn quote_struct_field(field: &syn::Field) -> TokenStream2 {
    let ident = &field.ident;
    let ty = &field.ty;
    quote! {
        #ident: #ty,
    }
}

fn quote_static_map(ast: &syn::DeriveInput) -> TokenStream2 {
    let struct_name = make_struct_name(&ast.ident);
    let map_name = make_map_name(&ast.ident);
    quote! {
        // TODO - remove invocation of thread_local! macro
        thread_local! {
            #[allow(non_upper_case_globals)]
            static #map_name: fakeptr::FakePtrMap<#struct_name> = fakeptr::FakePtrMap::new();
        }
    }
}

fn quote_to_fake_ptr(ast: &syn::DeriveInput) -> TokenStream2 {
    let struct_name = make_struct_name(&ast.ident);
    let map_name = make_map_name(&ast.ident);
    quote! {
        fn to_fake_ptr(t: #struct_name) -> fakeptr::FakePtr<#struct_name> {
            #map_name.with(|x| x.to_fake_ptr(t))
        }
    }
}

fn quote_recover(ast: &syn::DeriveInput) -> TokenStream2 {
    let struct_name = make_struct_name(&ast.ident);
    let map_name = make_map_name(&ast.ident);
    quote! {
        fn recover(p: fakeptr::FakePtr<#struct_name>) -> #struct_name {
            #map_name.with(|x| x.recover(p))
        }
    }
}

fn make_getter_name(field_name: &syn::Ident, struct_name: &syn::Ident) -> syn::Ident {
    format_ident!("get_{}_in_{}", field_name, struct_name)
}

fn make_getter_name_index(index: usize, struct_name: &syn::Ident) -> syn::Ident {
    format_ident!("get_field_{}_in_{}", index, struct_name)
}

fn quote_inner_get(struct_name: &syn::Ident, map_name: &syn::Ident, field_name: &syn::Ident, field_type: &syn::Type) -> TokenStream2 {
    let func_name = make_getter_name(field_name, struct_name);
    quote! {
        fn #func_name(p: &fakeptr::FakePtr<#struct_name>) -> #field_type {
            #map_name.with(|x| {
                let inner_map = x.ptr_to_t.take();
                let result = inner_map.get(&p.id).expect("Incorrect FakePtr requested, erroring out").#field_name;
                x.ptr_to_t.replace(inner_map);
                result
            })
        }
    }
}

fn quote_extern_get(struct_name: &syn::Ident, field_name: &syn::Ident, field_type: &syn::Type, index: usize) -> TokenStream2 {
    let inner_func_name = make_getter_name(field_name, struct_name);
    let func_name = format_ident!("{}_ffi", inner_func_name);
    let index_name = format_ident!("{}_ffi", make_getter_name_index(index, struct_name));
    quote! {
        #[no_mangle]
        extern fn #func_name (p: fakeptr::FakePtr<#struct_name>) -> #field_type {
            #inner_func_name(&p)
        }

        #[no_mangle]
        extern fn #index_name (p: fakeptr::FakePtr<#struct_name>) -> #field_type {
            #inner_func_name(&p)
        }
    }
}

fn quote_get(struct_name: &syn::Ident, map_name: &syn::Ident, field: &syn::Field, index: usize) -> TokenStream2 {
    let field_name = field.ident.as_ref().expect("TODO - no nameless fields allowed");
    let field_type = &field.ty;
    let inner_get = quote_inner_get(struct_name, map_name, field_name, field_type);
    let extern_get = quote_extern_get(struct_name, field_name, field_type, index);
    quote! {
        #[allow(non_snake_case)]
        #inner_get

        #[allow(non_snake_case)]
        #extern_get
    }
}

fn make_setter_name(field_name: &syn::Ident, struct_name: &syn::Ident) -> syn::Ident {
    format_ident!("set_{}_in_{}", field_name, struct_name)
}

fn make_setter_name_index(index: usize, struct_name: &syn::Ident) -> syn::Ident {
    format_ident!("set_field_{}_in_{}", index, struct_name)
}

fn quote_inner_set(struct_name: &syn::Ident, map_name: &syn::Ident, field_name: &syn::Ident, field_type: &syn::Type) -> TokenStream2 {
    let func_name = make_setter_name(field_name, struct_name);
    quote! {
        fn #func_name(p: &fakeptr::FakePtr<#struct_name>, v: #field_type) {
            #map_name.with(|x| {
                let mut inner_map = x.ptr_to_t.take();
                let inner_elem = inner_map.get_mut(&p.id).expect("Incorrect FakePtr requested, erroring out");
                inner_elem.#field_name = v;
                x.ptr_to_t.replace(inner_map);
            })
        }
    }
}

fn quote_extern_set(struct_name: &syn::Ident, field_name: &syn::Ident, field_type: &syn::Type, index: usize) -> TokenStream2 {
    let inner_func_name = make_setter_name(field_name, struct_name);
    let func_name = format_ident!("{}_ffi", inner_func_name);
    let index_name = format_ident!("{}_ffi", make_setter_name_index(index, struct_name));
    quote! {
        #[no_mangle]
        extern fn #func_name (p: fakeptr::FakePtr<#struct_name>, v: #field_type) {
            #inner_func_name(&p, v);
        }

        #[no_mangle]
        extern fn #index_name (p: fakeptr::FakePtr<#struct_name>, v: #field_type) {
            #inner_func_name(&p, v)
        }
    }
}

fn quote_set(struct_name: &syn::Ident, map_name: &syn::Ident, field: &syn::Field, index: usize) -> TokenStream2 {
    let field_name = field.ident.as_ref().expect("TODO - no nameless fields allowed");
    let field_type = &field.ty;
    let inner_set = quote_inner_set(struct_name, map_name, field_name, field_type);
    let extern_set = quote_extern_set(struct_name, field_name, field_type, index);
    quote! {
        #[allow(non_snake_case)]
        #inner_set

        #[allow(non_snake_case)]
        #extern_set
    }
}
