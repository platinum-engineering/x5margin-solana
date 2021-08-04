use proc_macro2::{Group, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    parse2, parse_macro_input, parse_quote,
    parse_quote::ParseQuote,
    punctuated::Punctuated,
    Expr, Ident, Result, Stmt, Token,
};

struct LoadStatement {
    is_writable: bool,
    ident: Ident,
    init_expr: Option<Expr>,
}

fn replace_this_ident(input: TokenStream, replacement: Ident) -> TokenStream {
    input
        .into_iter()
        .map(|tt| match tt {
            TokenTree::Group(group) => TokenTree::Group(Group::new(
                group.delimiter(),
                replace_this_ident(group.stream(), replacement.clone()),
            )),
            TokenTree::Ident(ident) => {
                if ident == "this" {
                    TokenTree::Ident(replacement.clone())
                } else {
                    TokenTree::Ident(ident)
                }
            }
            x => x,
        })
        .collect()
}

impl Parse for LoadStatement {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.parse::<Token!(&)>().is_ok() {
            let is_writable = input.parse::<Token!(mut)>().is_ok();
            let ident = input.parse::<Ident>()?;

            let init_expr = if input.parse::<Token!(=)>().is_ok() {
                Some(parse2::<Expr>(replace_this_ident(
                    input.parse::<syn::Expr>()?.into_token_stream(),
                    ident.clone(),
                ))?)
            } else if !input.is_empty() && !input.peek(Token!(,)) {
                return Err(syn::Error::new(
                    input.span(),
                    "expected `=` followed by an expression, or next statement",
                ));
            } else {
                None
            };

            Ok(Self {
                is_writable,
                ident,
                init_expr,
            })
        } else {
            Err(syn::Error::new(input.span(), "expected & or &mut here"))
        }
    }
}

struct LoadStatements {
    stmts: Vec<LoadStatement>,
}

impl Parse for LoadStatements {
    fn parse(input: ParseStream) -> Result<Self> {
        let list = Punctuated::<LoadStatement, Token!(,)>::parse(input)?;

        Ok(LoadStatements {
            stmts: list.into_iter().collect(),
        })
    }
}

pub fn parse_accounts(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let stmts = parse_macro_input!(input as LoadStatements).stmts;

    let mut new_stmts: Vec<Stmt> = vec![];
    for stmt in stmts {
        let LoadStatement {
            is_writable,
            ident,
            init_expr,
        } = stmt;

        new_stmts.push(parse_quote! {
            if input.is_empty() {
                solar::qlog!("cannot load `", stringify!(#ident), "` because there are not enough accounts (len = ", input.len(), ")");
                panic!("cannot load");
            }
        });

        new_stmts.push(parse_quote! {
            let #ident = input.next_account();
        });

        if is_writable {
            new_stmts.push(parse_quote! {
                if !solar::account::AccountFields::is_writable(solar::account::AccountBackend::backend(&#ident)) {
                    solar::qlog!("cannot load `", stringify!(#ident), "` because it is read-only, but expected writable (len = ", input.len(), ")");
                    panic!("cannot load");
                }
            })
        }

        if let Some(init_expr) = init_expr {
            new_stmts.push(parse_quote! {
                let mut #ident = #init_expr;
            });
        }
    }

    (quote! {#(#new_stmts)*}).into()
}
