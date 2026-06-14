//! Build script that embeds build-time metadata (date, git SHA) via vergen.

use anyhow::Result;
use vergen::{Build, Emitter};
use vergen_gix::Gix;

fn main() -> Result<()> {
    let build = Build::builder().build_date(true).build();
    let gix = Gix::builder().describe(true, true, None).sha(true).build();

    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&gix)?
        .emit()?;

    Ok(())
}
