use anyhow::Result;
use vergen::{BuildBuilder, CargoBuilder, Emitter};
use vergen_gix::GixBuilder;

fn main() -> Result<()> {
    let build = BuildBuilder::default().build_date(true).build()?;
    let gix = GixBuilder::default()
        .describe(true, true, None)
        .sha(true)
        .build()?;
    let cargo = CargoBuilder::all_cargo()?;

    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&gix)?
        .add_instructions(&cargo)?
        .emit()?;

    Ok(())
}
