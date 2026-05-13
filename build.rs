use anyhow::Result;
use vergen::{BuildBuilder, Emitter};
use vergen_gix::GixBuilder;

fn main() -> Result<()> {
    let build = BuildBuilder::default().build_date(true).build()?;
    let gix = GixBuilder::default()
        .describe(true, true, None)
        .sha(true)
        .build()?;
    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&gix)?
        .emit()?;

    Ok(())
}
