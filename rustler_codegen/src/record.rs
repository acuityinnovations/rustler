use proc_macro2::TokenStream;

use syn::{self, Data, Field};

use super::{Context, RustlerAttr};

pub fn transcoder_decorator(ast: &syn::DeriveInput) -> TokenStream {
    let ctx = Context::from_ast(ast);

    let record_tag = get_tag(&ctx);

    let struct_fields = match ast.data {
        Data::Struct(ref data_struct) => &data_struct.fields,
        Data::Enum(_) => panic!("NifRecord can only be used with structs"),
        Data::Union(_) => panic!("NifRecord can only be used with enums"),
    };

    let atom_defs = quote! {
        rustler::atoms! {
            atom_tag = #record_tag,
        }
    };

    let struct_fields: Vec<_> = struct_fields.iter().collect();

    let decoder = if ctx.decode() {
        gen_decoder(&ctx, &atom_defs, &struct_fields)
    } else {
        quote! {}
    };

    let encoder = if ctx.encode() {
        gen_encoder(&ctx, &atom_defs, &struct_fields)
    } else {
        quote! {}
    };

    let gen = quote! {
        #decoder
        #encoder
    };

    gen
}

fn gen_decoder(ctx: &Context, atom_defs: &TokenStream, fields: &[&Field]) -> TokenStream {
    let struct_type = &ctx.ident_with_lifetime;
    let struct_name = ctx.ident;

    // Make a decoder for each of the fields in the struct.
    let field_defs: Vec<TokenStream> = fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let ident = field.ident.as_ref().unwrap();
            let error_message = format!(
                "Could not decode field :{} on Record {}",
                ident.to_string(),
                struct_name.to_string()
            );
            let decoder = quote! {
                match ::rustler::Decoder::decode(terms[#index + 1]) {
                    Err(_) => return Err(::rustler::Error::RaiseTerm(Box::new(#error_message))),
                    Ok(value) => value
                }
            };

            quote! { #ident: #decoder }
        })
        .collect();

    let field_num = field_defs.len();
    let struct_name_str = struct_name.to_string();

    // The implementation itself
    let gen = quote! {
        impl<'a> ::rustler::Decoder<'a> for #struct_type {
            fn decode(term: ::rustler::Term<'a>) -> Result<Self, ::rustler::Error> {
                #atom_defs

                let terms = match ::rustler::types::tuple::get_tuple(term) {
                    Err(_) => return Err(::rustler::Error::RaiseTerm(Box::new(format!("Invalid Record structure for {}", #struct_name_str)))),
                    Ok(value) => value,
                };

                if terms.len() != #field_num + 1 {
                    return Err(::rustler::Error::Atom("invalid_record"));
                }

                let tag : ::rustler::types::atom::Atom = terms[0].decode()?;

                if tag != atom_tag() {
                    return Err(::rustler::Error::Atom("invalid_record"));
                }

                Ok(
                    #struct_name {
                        #(#field_defs),*
                    }
                )
            }
        }
    };

    gen
}

fn gen_encoder(ctx: &Context, atom_defs: &TokenStream, fields: &[&Field]) -> TokenStream {
    let struct_type = &ctx.ident_with_lifetime;

    // Make a field encoder expression for each of the items in the struct.
    let field_encoders: Vec<TokenStream> = fields
        .iter()
        .map(|field| {
            let field_ident = field.ident.as_ref().unwrap();
            let field_source = quote! { self.#field_ident };
            quote! { #field_source.encode(env) }
        })
        .collect();

    let tag_encoder = quote! { atom_tag().encode(env) };

    // Build a slice ast from the field_encoders

    let field_list_ast = quote! {
        [#tag_encoder, #(#field_encoders),*]
    };

    // The implementation itself
    let gen = quote! {
        impl<'b> ::rustler::Encoder for #struct_type {
            fn encode<'a>(&self, env: ::rustler::Env<'a>) -> ::rustler::Term<'a> {
                #atom_defs
                let arr = #field_list_ast;
                ::rustler::types::tuple::make_tuple(env, &arr)
            }
        }
    };

    gen
}

fn get_tag(ctx: &Context) -> String {
    ctx.attrs
        .iter()
        .find_map(|attr| match attr {
            RustlerAttr::Tag(ref tag) => Some(tag.clone()),
            _ => None,
        })
        .expect("NifStruct requires a 'tag' attribute")
}
