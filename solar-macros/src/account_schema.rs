use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{bracketed, parse::Parse, parse_macro_input, Expr, Token, Type};

mod kw {
    syn::custom_keyword!(name);
    syn::custom_keyword!(accounts);
    syn::custom_keyword!(s);
}

#[derive(Debug, PartialEq, Eq)]
pub enum AccessKind {
    Read,
    Write,
}

pub struct AccountDef {
    name: Ident,
    access: AccessKind,
    ty: Type,
    init: Expr,
    is_sign: bool,
}

pub struct AccountSchema {
    name: Ident,
    defs: Vec<AccountDef>,
}

impl Parse for AccountDef {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name = input.parse::<Ident>()?;

        let is_sign = if input.peek(Token![#]) && input.peek2(kw::s) {
            input.parse::<Token![#]>()?;
            input.parse::<kw::s>()?;
            true
        } else {
            false
        };

        input.parse::<Token![:]>()?;

        let access = if input.peek(Token![&]) {
            input.parse::<Token![&]>()?;
            if input.peek(Token![mut]) {
                input.parse::<Token![mut]>()?;
                AccessKind::Write
            } else {
                AccessKind::Read
            }
        } else {
            return Err(input.error("expected `&`"));
        };

        let ty = input.parse::<Type>()?;
        input.parse::<Token![=]>()?;

        let init = input.parse::<Expr>()?;
        input.parse::<Token![;]>()?;

        Ok(Self {
            name,
            access,
            ty,
            init,
            is_sign,
        })
    }
}

impl Parse for AccountSchema {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        input.parse::<kw::name>()?;
        input.parse::<Token![=]>()?;
        let name = input.parse::<Ident>()?;
        input.parse::<Token![,]>()?;
        input.parse::<kw::accounts>()?;
        input.parse::<Token![=]>()?;

        let defs_content;
        bracketed!(defs_content in input);
        let mut defs = vec![];
        while !defs_content.is_empty() {
            defs.push(defs_content.parse::<AccountDef>()?);
        }

        Ok(AccountSchema { name, defs })
    }
}

fn generate_schema_struct(schema: &AccountSchema) -> TokenStream {
    let mut fields = vec![];
    let name = &schema.name;

    for def in &schema.defs {
        let name = &def.name;
        let ty = &def.ty;
        fields.push(quote! {
            pub #name: #ty,
        });
    }

    quote! {
        pub struct #name<B: solar::account::AccountBackend> {
            #(#fields)*
        }
    }
}

fn generate_parsed_struct(schema: &AccountSchema) -> (Ident, TokenStream) {
    let name = Ident::new(&format!("{}Parsed", schema.name), schema.name.span());
    let mut fields = vec![];

    for def in &schema.defs {
        let name = &def.name;
        let ty = &def.ty;
        let ty = match def.access {
            AccessKind::Read => quote! {
                &'scope #ty
            },
            AccessKind::Write => quote! {
                &'scope mut #ty
            },
        };
        fields.push(quote! {
            pub #name: #ty,
        });
    }

    (
        name.clone(),
        quote! {
            pub struct #name<'scope, B: solar::account::AccountBackend> {
                #(#fields)*
            }
        },
    )
}

fn generate_impls(schema: &AccountSchema, parsed_struct_name: &Ident) -> TokenStream {
    let name = &schema.name;
    let mut blocks = vec![];
    let mut funcs = vec![];

    // Parsing
    {
        let mut stmts = vec![quote! {
            let program_id = *input.program_id();
        }];

        for def in &schema.defs {
            let name = &def.name;
            let init = &def.init;

            stmts.push(quote! {
                if input.is_empty() {
                    solar::util::log_not_enough_accounts(stringify!(#name), input.len());
                    panic!("cannot load");
                }
            });

            stmts.push(quote! {
                let this = input.next_account();
            });

            if def.access == AccessKind::Write {
                stmts.push(quote! {
                    if !solar::account::AccountFields::is_writable(solar::account::AccountBackend::backend(&this)) {
                        solar::util::log_not_writable_account(stringify!(#name), input.len());
                        panic!("cannot load");
                    }
                })
            }

            stmts.push(quote! {
                let #name = #init;
            });
        }

        let idents = schema.defs.iter().map(|d| &d.name).collect::<Vec<_>>();
        stmts.push(quote!(
            Ok(#name {
                #(#idents),*
            })
        ));

        funcs.push(quote! {
            pub fn from_program_input<T: solar::input::AccountSource<B>>(input: &mut T) -> Result<Self, Error> {
                #(#stmts)*
            }
        });
    }

    // Meta generation
    {
        let metas = schema.defs.iter().map(|def| {
            let name = &def.name;
            let is_write = def.access == AccessKind::Write;
            let is_sign = def.is_sign;
            quote! {
                solana_api_types::AccountMeta {
                    pubkey: *self.#name.key(),
                    is_signer: #is_sign,
                    is_writable: #is_write,
                }
            }
        });

        let stmt = quote! {
            vec![
                #(#metas),*
            ]
        };

        funcs.push(quote! {
            pub fn metas(&self) -> Vec<solana_api_types::AccountMeta> {
                #stmt
            }
        })
    }

    // Borrowing
    {
        let fields_borrow = schema.defs.iter().map(|def| {
            let name = &def.name;
            if def.access == AccessKind::Write {
                quote! {
                    &mut self.#name
                }
            } else {
                quote! {
                    &self.#name
                }
            }
        });

        let fields = schema.defs.iter().map(|def| &def.name);

        funcs.push(quote! {
            pub fn borrow(&mut self) -> #parsed_struct_name<'_, B> {
                #parsed_struct_name {
                    #(
                        #fields: #fields_borrow
                    ),*
                }
            }
        })
    }

    // Constructor
    {
        let fields = schema.defs.iter().map(|def| &def.name).collect::<Vec<_>>();

        funcs.push(quote! {
            pub fn new(#(#fields: &solana_api_types::Pubkey),*) -> #name<solar::account::pubkey::PubkeyAccount> {
                #name {
                    #(
                        #fields: (*#fields).into()
                    ),*
                }
            }
        })
    }

    blocks.push(quote! {
        #[allow(clippy::all)]
        impl<B: solar::account::AccountBackend> #name<B> {
            #(#funcs)*
        }
    });

    quote! { #(#blocks)* }
}

pub fn account_schema(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let schema = parse_macro_input!(input as AccountSchema);

    let mut output = vec![generate_schema_struct(&schema)];
    let (parsed_struct_name, parsed_struct_def) = generate_parsed_struct(&schema);
    output.push(parsed_struct_def);
    output.push(generate_impls(&schema, &parsed_struct_name));

    (quote! {
        #(#output)*
    })
    .into()
}
