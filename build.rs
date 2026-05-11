use vergen::{BuildBuilder, Emitter};
use vergen_gix::GixBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let build = BuildBuilder::default().build_date(true).build()?;
    let gix = GixBuilder::default().describe(true, true, None).build()?;

    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&gix)?
        .emit()?;

    Ok(())
}
