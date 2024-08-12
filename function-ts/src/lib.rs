use proc_macro::TokenStream;
use ts_rs::TS;

#[proc_macro_attribute]
pub fn ts(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let function: syn::ItemFn = syn::parse(item).unwrap();
    let sig = &function.sig;
    let inputs = &sig.inputs;
    let output = &sig.output;
    let name = &sig.ident;
    quote::quote! {
        impl ts_rs::TS for #name {

        }
        #function
    }
    .into()
}

impl TS for ts {
    type WithoutGenerics = Self;

    fn decl() -> String {}

    fn decl_concrete() -> String {
        todo!()
    }

    fn name() -> String {
        todo!()
    }

    fn inline() -> String {
        todo!()
    }

    fn inline_flattened() -> String {
        todo!()
    }
}
