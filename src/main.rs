mod error;

fn main() -> error::Result<()> {
    color_eyre::install()?;
    println!("distill");
    Ok(())
}
