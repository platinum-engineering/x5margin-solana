use quote::quote;
use syn::{parse_macro_input, LitStr};

mod account_schema;
mod parse_accounts;

#[proc_macro]
pub fn parse_accounts(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    parse_accounts::parse_accounts(input)
}

#[proc_macro]
pub fn account_schema(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    account_schema::account_schema(input)
}

#[proc_macro]
pub fn parse_base58(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as LitStr);

    let data = bs58::decode(input.value())
        .into_vec()
        .expect("invalid bs58 format");

    (quote! {
        [#(#data),*]
    })
    .into()
}

#[proc_macro]
pub fn parse_pubkey(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as LitStr);

    let data = bs58::decode(input.value())
        .into_vec()
        .expect("invalid bs58 format");

    if data.len() != 32 {
        panic!("pubkey must be 32 bytes");
    }

    (quote! {
        solana_api_types::Pubkey::new([#(#data),*])
    })
    .into()
}
