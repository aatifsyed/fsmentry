use fsmgen::FSMGenerator;
use syn::parse::Parser as _;

fn main() -> anyhow::Result<()> {
    let input = std::fs::read_to_string("/dev/stdin")?;
    let output = FSMGenerator::parse_dot.parse_str(&input)?.codegen();
    println!("{}", prettyplease::unparse(&output));
    Ok(())
}
