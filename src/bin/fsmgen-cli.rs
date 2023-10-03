use std::{
    io::{Read, Write as _},
    path::{Path, PathBuf},
    process::Stdio,
};

use anyhow::{bail, Context as _};
use clap::{Parser, ValueEnum};
use fsmgen::FSMGenerator;
use quote::ToTokens as _;
use syn::{parse::Parser as _, parse_quote};

/// Read a file in a DSL or DOT, and generate rust code for a state machine.
#[derive(Parser)]
struct Args {
    /// Input file to generate from.
    /// If `-` or not supplied, read from stdin.
    file: Option<PathBuf>,
    /// Whether to shell out to `dot` to render an SVG to include in the diagram documentation.
    #[arg(long, name = "INCLUDE_SVG", default_value = "auto")]
    svg: IncludeSvg,
}

#[derive(ValueEnum, Clone)]
enum IncludeSvg {
    Force,
    Omit,
    Auto,
}

fn main() -> anyhow::Result<()> {
    let Args { file, svg } = Args::parse();
    let input = match file {
        Some(path) if path == Path::new("-") => get_stdin()?,
        Some(path) => std::fs::read_to_string(path).context("error reading file")?,
        None => get_stdin()?,
    };
    let generator = FSMGenerator::parse_dsl.parse_str(&input)?;
    let mut codegen = generator.codegen();
    let dot = generator.dot();
    let svg = match svg {
        IncludeSvg::Force => Some(get_svg(dot)?),
        IncludeSvg::Omit => None,
        IncludeSvg::Auto => get_svg(dot).ok(),
    };
    let Some(syn::Item::Struct(syn::ItemStruct { attrs, .. })) = codegen.items.first_mut() else {
        unreachable!("the struct is the first item generated")
    };
    if let Some(svg) = svg {
        let svg = format!("<div>{}</div>", svg);
        if !attrs.is_empty() {
            attrs.push(parse_quote!(#[doc = ""]))
        }
        attrs.push(parse_quote!(#[doc = #svg]))
    }

    println!("{}", prettyplease::unparse(&codegen));
    Ok(())
}

fn get_stdin() -> anyhow::Result<String> {
    let mut s = String::new();
    std::io::stdin()
        .read_to_string(&mut s)
        .context("error reading from stdin")?;
    Ok(s)
}

fn get_svg(dot: syn_graphs::dot::Graph) -> anyhow::Result<String> {
    let mut child = std::process::Command::new("dot")
        .arg("-Tsvg")
        .stdin(Stdio::piped())
        .stderr(Stdio::inherit())
        .stdout(Stdio::piped())
        .spawn()
        .context("could not exec `dot` - is it installed and on the PATH?")?;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(dot.into_token_stream().to_string().as_bytes())
        .context("couldn't pipe to `dot`")?;
    let output = child.wait_with_output().context("couldn't join `dot`")?;
    match output.status.code() {
        Some(0) => String::from_utf8(output.stdout).context("`dot` returned a non-utf8 svg"),
        Some(n) => bail!("`dot` exited with code {}", n),
        None => bail!("`dot` exited abnormally"),
    }
}
