use fsmgen::FSMGenerator;
use syn::{parse::Parser as _, File};

fn main() -> anyhow::Result<()> {
    let input = std::fs::read_to_string("/dev/stdin")?;
    let output = syn::parse2::<File>(FSMGenerator::parse_dot.parse_str(&input)?.codegen())?;
    println!("{}", prettyplease::unparse(&output));
    Ok(())
}
