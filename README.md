# DPS — Dive Planning System

[![CI](https://github.com/asakura/inceptool/actions/workflows/ci.yml/badge.svg)](https://github.com/asakura/inceptool/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://asakura.github.io/inceptool/coverage.json)](https://asakura.github.io/inceptool/)
[![cargo-deny](https://img.shields.io/endpoint?url=https://asakura.github.io/inceptool/cargo-deny.json)](https://asakura.github.io/inceptool/)

A terminal UI application for scuba dive planning.

## ⚠️ Safety Disclaimer

Scuba diving is an inherently dangerous activity. Errors in dive planning can
result in decompression sickness, arterial gas embolism, oxygen toxicity, or
death. This software is provided for educational and planning purposes only
and is **not** a substitute for proper dive training, a certified dive
computer, or a qualified instructor.

Independently verify every number this tool produces, dive within the limits
of your training and certification, and always have a second method of
computing your dive plan. This software is provided "as is", without
warranty of any kind — see [LICENSE-MIT](LICENSE-MIT) and
[LICENSE-APACHE](LICENSE-APACHE). You use it entirely at your own risk; the
authors accept no liability for any injury, death, or other damages arising
from its use.

## Features

- Interactive MOD and ppO₂ tables for nitrox dive planning.
- Decoupled dive-physics library crates.
- Vim-style key bindings and registers.
- Support for various dive environments (ocean, lake, altitude).

## Crates

- `dps`: The main TUI application.
- `dps-units`: Type-safe physical units.
- `dps-environment`: Dive environment models (altitude, salinity).
- `dps-gas`: Gas mix and blending calculations.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
