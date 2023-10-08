use quote::ToTokens as _;
use std::{
    io::Write as _,
    process::{Command, Stdio},
};
use syn::parse_quote;

pub fn attach(mut file: syn::File, generator: &fsmentry_core::FSMGenerator) -> syn::File {
    let Some(syn::Item::Mod(syn::ItemMod { attrs, .. })) = file.items.first_mut() else {
        unreachable!("the code generates a module")
    };
    if let Some(svg) = call_dot(generator) {
        let svg = format!("<div>{}</div>", svg);
        if !attrs.is_empty() {
            attrs.push(parse_quote!(#[doc = ""]))
        }
        attrs.push(parse_quote!(#[doc = #svg]))
    }
    file
}

fn call_dot(generator: &fsmentry_core::FSMGenerator) -> Option<String> {
    let mut child = Command::new("dot")
        .arg("-Tsvg")
        .stdin(Stdio::piped())
        .stderr(Stdio::inherit())
        .stdout(Stdio::piped())
        .spawn()
        .ok()?;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(generator.dot().to_token_stream().to_string().as_bytes())
        .ok()?;
    let output = child.wait_with_output().ok()?;
    match output.status.success() {
        true => String::from_utf8(output.stdout).ok(),
        false => None,
    }
}
