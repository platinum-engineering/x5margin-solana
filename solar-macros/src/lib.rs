mod parse_accounts;

#[proc_macro]
pub fn parse_accounts(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    parse_accounts::parse_accounts(input)
}
