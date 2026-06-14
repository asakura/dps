# Changelog

All notable changes to this project will be documented in this file.

## [0.1.1] - 2026-06-14

### [ADD] Features

- Add file-based tracing with color-eyre

- Add Catppuccin theme system with full palette and styling guide

- Add surface0 background to status bar

- Colour focused column with lavender

- Colour table block title with lavender

- Add tab bar with active/inactive styling

- Add which-key popup for context-sensitive key binding discovery

- *(cli)* Add CLI arg parsing with clap and vergen build metadata

- Implement Widget for &mut ModTab and &mut PpO2Tab

- Implement Widget for &mut App, align Component::render with ratatui API

- *(errors)* Add init() with panic hook, trace_dbg! macro, and panic deps

- *(config)* Add serde, serde_json, config, json5 deps and wire up config loading

- Wire up keybinding system, movement dispatch, and config overrides

- *(ppo2_tab)* Handle Action::Select, store depth/mix selection

- *(app)* Drive event loop with tokio and async Tui::next_event

- *(tui)* Validate tick_rate and frame_rate in builder and start()

- *(tui)* Implement Default for Tui delegating to new()

- *(theme)* Replace global THEME static with runtime-configured theme

- *(gas)* Introduce Mod value type for maximum operating depth

- *(config)* Replace String errors with ParseKeyError enum

- *(gas)* Add EANxDetail<M> newtype for verbose gas display

- *(gas)* Add PPO2, EAD, END, MND newtypes for EANx depth calculations

- *(units)* Add GramsPerLitre newtype; use it in gas_density_at

- *(units)* Add OTUPerMinute newtype; use it in otu_rate_at

- *(units)* Add CnsRatePerMinute newtype; use it in cns_rate_at

- *(environment)* Add absolute_pressure/depth helpers; use them in gas

- *(action)* Expand Action enum with infrastructure and error variants

- *(app)* Handle SIGTERM, SIGINT, and Suspend/Resume in the event loop

- *(components)* Add ComponentNew trait with lifecycle methods

- *(components)* Add FpsCounter overlay component

- *(components)* Add Home placeholder component

- *(app)* Implement AppNew event-loop coordinator

- *(keymap)* Add keymap module with modes, sequences, maps, and chord engine

- *(action)* Add Confirm and Cancel actions for confirmation prompts

- *(config)* Rename Home mode to Normal, add Confirm mode bindings

- *(app)* Replace AppNew tick-buffer chord logic with SequenceEngine

- *(app)* Handle SIGTERM/SIGINT via tokio::select in the event loop

- *(config)* Complete Normal-mode keybindings and stub planned bindings

- *(movement)* Add LineUp/LineDown variants for single-line scroll

- *(keymap)* Implement count prefix for SequenceEngine

- *(action)* Gate count-prefix repeat on Action::accepts_count

- *(deps)* Add arboard for clipboard access, sort dependencies

- *(config)* Bind yy/dd/p/P to Edit actions in Normal mode

- *(registers)* Implement Vim-style register store

- *(units)* Add ParseError type and FromStr impls for all unit newtypes

- *(gas)* Add FromStr impl for EANx and ParseEANxError type

- *(registers)* Introduce ParseError type for RegisterValue::from_str

- *(dive_environment)* Add FromStr impl and ParseDiveEnvironmentError

- *(action)* Add EditOp, register-prefix handling, and RegisterStore wiring

- *(errors)* Wire ActionError, UnitError, KeyMapError, and RegisterError into top-level Error

- *(components)* Add YankRow to ModTab via push_yank

- *(components)* Wire Delete routing in ModTab with named-register bypass

- *(registers)* Add yank ring and restrict YankRow count semantics

- *(edit)* Add CyclePaste op and wire Paste, PasteAbove, CyclePaste in ModTab

- *(units)* Change Percent::new to return Result; add Percent::literal

- *(registers)* Add RegisterName type

- *(registers)* Add typed error variants for RegisterName validation

- *(app)* Introduce AppError; migrate event-loop methods off color_eyre

- *(action)* Add Tab(TabDir) action and leader-key substitution

- *(components)* Introduce TabPane; migrate tabs to ComponentNew

- *(config)* Introduce ThemeMap, ThemeConfigMap, and PaletteConfigMap newtypes

- *(units)* Add optional serde Serialize/Deserialize derives to all unit types

- *(units)* Add Default, Sum, Add/Sub impls and deprecated as_f64 to unit types

- *(units)* Add to_clipboard_string for lossless roundtrip serialisation

- *(units)* Re-export ParseError from crate root

- *(units)* Make approx an optional feature

- *(environment)* Named Display/FromStr for Ocean and Lake presets

- *(environment)* Mark Ocean and Lake non_exhaustive; export physics validation limits

- *(environment)* Add to_clipboard_string() for lossless DiveEnvironment serialisation

- *(environment)* Add MIN_ALTITUDE and MIN_SALINITY_PPT constants; use in validation

- *(environment)* Tag preset constructors to fix Display/clipboard identity

- *(gas)* Add serde Serialize/Deserialize to gas blend types

- *(gas)* Publish and document gas composition constants

- *(gas)* Implement Default for Membrane delegating to typical()

- *(gas)* Implement Default for EANx returning air

- *(git-stats)* Add dps-git-stats CLI for per-day commit statistics


### [FIX] Bug Fixes

- Make abs_pressure_bar() return Bar instead of f64

- Make Bar API consistent with Meters

- Move col_window_size doc to the correct function

- Implement Default for App, ModTab, and PpO2Tab to satisfy clippy

- Restore terminal before color-eyre panic hook fires

- *(cli)* Use git describe as version when available, omit stray dash

- *(cli)* Append short commit hash to version when no git tag exists

- *(cli)* Use short git SHA in version string

- *(main)* Wire tick_rate as input-poll interval, decouple from frame rate

- *(config)* Serialize env-var tests with a mutex to prevent race

- *(tests)* Import SCROLL_DELTA/PAGE_DELTA explicitly in action_dispatch tests

- *(tui)* Use block_in_place in stop() to avoid blocking the executor

- *(app)* Exhaust event match and log crossterm errors

- *(app)* Warn on config load failure instead of silently falling back

- *(cli,tui)* Promote debug_assert to assert and validate rates at CLI boundary

- *(tui)* Emit Event::Closed when event stream closes naturally

- *(tui)* Fix stop() doc comment and replace magic numbers with named constants

- *(lints)* Replace expect() with propagation or unreachable! to satisfy expect_used

- *(lints)* Replace unwrap() with unreachable! to satisfy unwrap_used

- *(lints)* Add #[must_use] to satisfy must_use_candidate warnings

- *(lints)* Suppress cast_precision_loss for bounded usize→f64 casts

- *(lints)* Replace redundant closures with method references

- *(lints)* Add #[must_use] to max method in unit_newtype! macro

- *(lints)* Resolve 18 clippy warnings across six files

- *(lints)* Add strict lint config and fix elided lifetime in Frame

- *(lints)* Add crate-level doc comment to build.rs

- *(lints)* Implement Debug for all public structs

- *(lints)* Add missing documentation across public API

- *(lints)* Derive Copy for five types that satisfy its requirements

- *(lints)* Add crate-level doc comment to main.rs

- *(lints)* Replace clone with copy for Action type

- *(lints)* Add missing Cargo.toml package metadata

- *(lints)* Replace Action:: with Self:: in impl blocks

- *(lints)* Add reason to expect attribute in gas.rs

- *(lints)* Add const to functions identified by missing_const_for_fn

- *(lints)* Use map_or_else instead of if let/else in config dir functions

- *(lints)* Add missing doc comment to test_utils module

- *(lints)* Add missing doc comment to widget_text function

- *(lints)* Remove unused peekable() call on chars iterator in parse_key_sequence

- *(lints)* Use mul_add for float multiply-add expressions in mod_tab

- *(lints)* Use Self instead of explicit type names in From and Deserialize impls

- *(lints)* Make ppo2 method const in mod_tab

- *(lints)* Require Send on Component trait objects to satisfy future_not_send

- *(lints)* Allow clippy::exit in panic hook where process::exit is intentional

- *(lints)* Replace unsafe env::set_var with temp-env in config tests

- *(tui)* Ensure stop() always waits for task to finish

- *(lints)* Collapse nested if blocks in stop() per clippy::collapsible_if

- *(theme)* Replace #[allow] with #[expect] + reason per clippy config

- *(main)* Make App::new error conversion to color_eyre::Report explicit

- *(gas)* Use ISO-standard surface pressure and seawater constants

- *(environment)* Resolve Clippy pedantic lints

- *(environment)* Correct Red Sea and Baltic salinity; rewrite module docs

- *(environment)* Merge identical PersianGulf/RedSea salinity arms

- *(app)* Emit SIGTSTP on suspend so the OS actually pauses the process

- *(clippy)* Resolve pedantic lint warnings

- *(gas)* Re-export InvalidEANxError and ParseEANxError from crate::gas

- *(config)* Update KeyResolutionError re-export to use KeyMapError

- *(tab)* Add module doc example and simplify FromStr via combinators

- *(environment)* Resolve broken intra-doc links in dive_environment

- *(dps)* Resolve unresolved imports by using dps_gas prelude

- *(registers)* Update EANx import path to dps_gas::prelude and fix use ordering

- *(environment)* Correct fabricated water_density in to_clipboard_string doctest

- *(gas)* Use AR_NARCOTIC_POTENCY constant in narcotic fraction test

- *(gas)* Correct OTU rate formula and fix use-statement ordering in EANx modules

- Update build.rs for vergen 10 API


### [MOD] Refactor

- Move seawater pressure conversion out of Meters

- Eliminate newtype boilerplate with unit_newtype! macro

- Move max into unit_newtype! macro

- Absorb arithmetic ops and docs into unit_newtype! macro

- Move label() from free function to Ean method

- Rename Ean private field fo2 -> fraction

- Replace debug_assert in from_percent with Result

- Use RAII Tui guard for guaranteed terminal cleanup

- Extract trailing_constraints helper to deduplicate column layout logic

- Extract col_window_size helper to deduplicate window sizing formula

- Extract styled_table and build_header_row to deduplicate table builders

- Extract window_start free function to deduplicate window logic

- Extract cursor_next/cursor_prev to deduplicate TableState movement

- Extract idx_next/idx_prev to deduplicate index increment/decrement

- Introduce Component trait and Action enum for scalable tab architecture

- Move window_start to ui.rs to eliminate duplication

- Restrict pub constants to private now that rendering is co-located

- Derive PPO2_TABLE_MIX_COUNT from slice length to prevent drift

- Name fixed column count in ppo2_tab for consistency with mod_tab

- Convert which_key from free function to Widget impl

- *(which_key)* Replace manual padding with Layout-based dynamic columns

- *(which_key)* Use Style::from() tuple constructors instead of builder chain

- *(theme)* Centralise all Style construction as semantic Theme methods

- *(which_key)* Convert render_entry to Entry widget; split and improve tests

- *(mod_tab)* Promote build_rows to method, extract move_row helper

- *(app)* Use Layout::vertical() shorthand, drop Direction import

- *(ui)* Clarify trailing_constraints by separating uniform and fill columns

- *(app)* Consolidate handle_key into a single match

- *(ppo2_tab)* Extract move_row helper, collapse handle_key Down/Up arms

- *(ppo2_tab)* Move build_rows into impl block as associated function

- Extract ModRow and PpO2Row row-builder types

- *(main)* Drop calls restore_terminal() instead of repeating cleanup

- Extract ModTabStatus and PpO2TabStatus widgets, replace status_bar() with render_status()

- *(app)* Extract HintBar widget

- *(errors)* Extract InvalidO2Percent into errors.rs

- *(logging)* Replace lazy_static with std::sync::LazyLock and modernise init

- Drop Component::handle_key, route all input through handle_action

- *(tests)* Drop redundant _fn suffix from test module names

- *(tests)* Move shared widget_text helper to components::test_utils

- Hoist SCROLL_DELTA/PAGE_DELTA to components/mod.rs

- *(app)* Drop redundant Action::None arm in dispatch

- *(components)* Hoist move_row to components/mod.rs

- *(tests)* Use shared widget_text from components::test_utils in app.rs

- *(action)* Group movement actions into Movement sub-enum

- *(tui)* Extract Tui RAII guard into tui.rs

- *(tui)* Make terminal field private

- *(tui)* Remove redundant Ok(()) in resume()

- *(tui)* Defer tokio::spawn to start(), enabling sync builder tests

- *(tui)* Replace pub event_tx field with event_tx() getter

- *(logging)* Avoid unnecessary clone() on LazyLock<String>

- *(action)* Derive Copy for Movement, drop redundant clone()

- *(config)* Use or_insert_with to defer cmd clone

- *(config)* Prefer to_owned() over to_string() on string literal

- *(theme)* Enforce semantic API for safety colours and static/method hygiene

- *(theme)* Make all palette fields private, remove accents()

- *(theme)* Split Palette from Theme (Option B)

- *(gas)* Group tests into submodules by method

- *(components)* Use method reference in widget_text helper

- Prefer .as_slice() over &[..] for static slice coercions

- *(theme)* Promote Theme fields from Color to Style, add modifier support

- *(config/theme)* Move theme tests to theme.rs, clean up internals

- *(units)* Introduce Percent newtype, migrate Ean to typed O₂ fractions

- *(units)* Expose approx traits unconditionally

- *(errors)* Move InvalidO2Percent into the gas module

- *(mod_tab)* Adopt EANx/MOD names and flattened API

- *(ppo2_tab)* Rename Ean to EANx

- *(gas)* Extract gas constants into gas/constants.rs

- *(gas)* Extract GasComponents into gas/components.rs

- *(gas)* Extract EANx error types into gas/eanx/errors.rs

- *(gas)* Extract MOD logic into gas/eanx/minimum_operating_depth.rs

- *(gas)* Extract operating depth logic into gas/eanx/operating_depth.rs

- *(gas)* Introduce gas/eanx submodule, re-exporting EANx and MOD

- *(gas)* Extract membrane blending into gas/blend/membrane.rs

- *(gas)* Extract partial pressure blending into gas/blend/partial_pressure.rs

- *(gas)* Extract PSA blending into gas/blend/psa.rs

- *(gas)* Introduce gas/blend submodule

- *(gas)* Introduce gas/mod.rs as the module root

- *(gas)* Delete monolithic gas.rs, superseded by gas/ module tree

- *(errors)* Adopt thiserror, extract error modules, propagate config failures

- *(gas)* Mark InvalidEANx and InvalidMembraneFractions as #[non_exhaustive]

- *(gas)* Replace fmt_gas_name free fn with gas_name() -> impl Display

- *(gas)* Align MODSummary and MiniMODSummary with EANxDetail pattern

- *(gas)* Eliminate .value() at EAN_MIN_O2 comparisons and best_mix

- *(units)* Split monolithic units.rs into per-type submodules

- *(units)* Eliminate .value() in favour of From<T> for f64

- *(gas)* Type air-composition constants as Percent

- *(environment)* Extract magic numbers into named constants

- *(environment)* Split mod.rs into focused submodules; add module docs

- *(units)* Add Celsius and PartsPerThousand newtypes; migrate environment API to typed units

- Localize constants, migrate errors to thiserror, rename InvalidEANx

- *(action)* Change Move serialization to Move(...), add FromStr

- *(action)* Split into action/mod.rs + action/movement.rs

- *(chord)* Extract ChordEngine trait and SequenceEngine from app

- *(components)* Extract HintBar into its own module

- *(chord)* Remove src/chord.rs, moved to keymap::chord

- *(mode)* Remove src/mode.rs, moved to keymap::mode

- *(lib)* Replace chord/mode modules with keymap module

- *(config)* Re-export ParseError from keymap instead of duplicating

- *(config)* Remove key parsing impl, re-export from keymap::keys

- *(config)* Use KeyBindingsBuilder from keymap, remove inline KeyBindings type

- *(app)* Update imports and mode refs to use keymap module

- *(config)* Remove config::keys re-export shim

- *(dive_environment)* Split module into sub-files, add Display and ParseError

- *(environment)* Inline error module into dive_environment

- *(units)* Introduce UnitError as the stable public error boundary

- *(registers)* Introduce RegisterError as the stable public error boundary

- *(keymap)* Introduce KeyMapError as the stable public error boundary

- *(action)* Introduce ActionError as the stable public error boundary

- *(gas)* Migrate Percent callers to new Result API

- *(components)* Update remaining Percent callers to new Result API

- *(registers)* Re-export RegisterName and updated error types

- *(registers)* Use RegisterName in RegisterStore read/write API

- *(gas)* Remove Copy from Error derive

- *(gas/eanx)* Remove Copy, add Unit variant, align tests to style guide

- *(gas/eanx)* Change best_mix to return Result instead of Option

- *(keymap)* Use RegisterName instead of char for pending_register

- *(action)* Remove redundant Ok() wrapper in movement test helper

- *(action)* Use RegisterName instead of char in EditOp variants

- *(action)* Update Action callers to RegisterName; simplify accepts_count

- *(components)* Simplify Percent::new calls in ppo2_tab tests

- *(components)* Use RegisterName throughout ModTab register operations

- *(tui)* Return std::io::Result from fallible methods

- *(cli)* Replace color_eyre::Result and Box<dyn Error> in tests

- *(ui)* Replace color_eyre::Result and Box<dyn Error> in tests

- *(tests)* Replace color_eyre::Result across remaining modules

- *(tests)* Simplify helper returns and reformat lookup calls

- *(tests)* Migrate component tests to rstest and drop color_eyre

- *(tests)* Replace color_eyre::Result with concrete error types in remaining modules

- *(imports)* Enforce use-statement ordering and compaction codebase-wide

- *(action)* Introduce Prompt/Ui wrappers, rename TabDir, drop Movement::None

- *(components)* Rename ComponentNew to Component

- *(logging)* Extract LoggingError and convert module to subdir layout

- *(cli)* Move dir resolution into Cli, make dirs non-optional, export PROJECT_NAME

- *(config)* Remove dir-resolution helpers; from_dirs takes concrete paths

- *(app)* Accept &Args in App::new, init logging inside constructor

- *(main)* Simplify startup to Cli::parse().try_into() + App::new(&args)

- *(config)* Extract RawConfigContext and TryFrom for Config assembly

- *(config)* Hoist config_files array to module-level const

- *(config)* Replace config crate with direct json5/yaml/toml parsing

- *(app)* Rewrap doc comment line

- *(app)* Mark doc example as no_run to full UI example

- *(tests)* Replace assert!(matches!()) with assert_matches!

- *(config)* Remove unused Styles placeholder struct

- *(keymap)* Split bindings module into builder and registry submodules

- *(app)* Use ThemeMap type alias instead of bare HashMap in tests

- *(app)* Use ThemeMap::default() in test config fixture

- *(units)* Extract units module into standalone dps-units crate

- *(environment)* Extract environment module into standalone dps-environment crate

- *(gas)* Extract gas module into standalone dps-gas crate

- *(libs)* Expose physics constants and gas constants/helpers as pub

- *(units)* Extract macros into dedicated macroses/ submodules

- *(units)* Split From/TryFrom<f64> between unbounded and bounded newtypes

- *(units)* Replace assert!(is_ok/is_err) with assert_matches!

- *(environment)* Expand ParseDiveEnvironmentError into a rich enum

- *(config)* Extract RawConfig into its own module

- *(gas)* Rename EAD.ead field and accessor to depth

- *(gas)* Rename END.end field and accessor to depth

- *(gas)* Rename MND.mnd field and accessor to depth

- *(gas)* Update call sites and docs to use .depth() accessors

- *(gas)* Introduce prelude and consolidate public API

- *(gas)* Rename Ppo2Summary to PPO2Summary


### [DOC] Documentation

- Add missing documentation across units, app, and ui

- Add missing doc comments across all public and pub(crate) items

- *(theme)* Fix unassigned slot inventory and subtext1 guidance

- Add missing docs and doc tests across all modules

- Fix private Entry link, add KeyBinding field docs, document HintBar

- *(config)* Add module and function docs; test env-var overrides

- *(cli)* Add module, struct, field, and version() docs; add CLI and version tests

- *(tui)* Document event_tx as the synthetic event injection point

- *(tui)* Add doc comments and examples to all public functions

- *(tui)* Add # Panics section to Default impl

- *(tui)* Document that stop() currently never errors

- *(lints)* Add missing # Errors and # Panics sections to satisfy missing_errors_doc / missing_panics_doc warnings

- *(lints)* Wrap missing backtick items in doc comments

- *(theme)* Steer callers toward semantic methods, away from raw fields

- *(app)* Replace color_eyre::Result with Box<dyn Error> in doc examples

- *(theme)* Add TODO placeholder for module documentation

- *(theme)* Remove stale TODO placeholder

- *(app)* Document App::default as test/harness constructor, not production path

- *(units)* Complete OTUPerMinute examples; fix otu_rate_at doc types

- Fix all broken intra-doc links

- Replace katexit with KaTeX auto-render; convert ocean/lake docs to LaTeX

- Commit leftover KaTeX migration files

- *(environment)* Fix doc examples — ignore → no_run, update API calls

- *(environment)* Add missing use imports to ignored doc examples

- *(components)* Expand ComponentNew trait docs and re-export FpsCounter/Home

- Update stale references after AppNew→App rename

- *(claude)* Add CLAUDE.md with code quality standards

- *(physics)* Add module doc comment with runnable example

- *(units)* Clarify Percent::literal contract and suppress clippy::panic

- *(keymap)* Simplify ParseError variant links in parse_keys doc comment

- *(keymap)* Add Examples sections to KeyBindings and KeyBindingsBuilder methods

- Add README and update CLAUDE.md path reference after crate extraction

- *(units)* Make all doc examples runnable and expand crate-level docs

- *(units)* Enrich doc comments with KaTeX-formatted physical quantities

- *(claude)* Add KaTeX backslash-escaping guide to CLAUDE.md

- *(environment)* Add CLAUDE.md with API contract and standards for dps-environment

- *(units)* Add CLAUDE.md with API contract and standards for dps-units

- *(environment)* Improve KaTeX formatting in dive_environment doc comments

- *(gas)* Add CLAUDE.md with API contract and standards for dps-gas

- *(environment)* Convert inline math to KaTeX syntax in physics constants

- *(units)* Convert O2 subscripts to KaTeX \ce{} chemistry notation

- *(src)* Convert ppO2 to KaTeX notation in doc comments

- *(gas)* Convert chemical formulas and abbreviations to KaTeX notation

- *(gas)* Convert remaining Ar and percentage literals to KaTeX notation

- *(environment)* Convert density formula to KaTeX notation

- *(registers)* Add module-level doc example for RegisterStore

- *(gas)* Document why BlendMethod is sealed

- *(gas)* Promote no_run doc examples to runnable examples

- *(gas)* Wrap bare Ar and g/mol in KaTeX chemical/unit macros

- *(gas)* Promote remaining no_run doc examples to runnable

- *(gas)* Render GasComponents sum invariant as KaTeX formula

- *(readme)* Add safety disclaimer and dual-license notice


### [PWR] Performance

- *(app)* Skip render when state has not changed


### [UI_] Styling

- Replace format!("{}") with to_string() in mod_tab

- *(app)* Apply rustfmt formatting and move Widget impl before tests

- *(ui)* Apply rustfmt formatting and move build_header_row before tests

- Apply rustfmt formatting across all remaining files

- Apply rustfmt formatting

- *(lints)* Inline variables directly into format! strings

- *(units)* Add blank lines between impl blocks for readability

- Apply rustfmt formatting; make Theme::new const

- *(ui)* Prefer .as_slice() over &[..] in test call sites

- *(units)* Rustfmt alignment in tests

- *(lake)* Wrap long test assertion to fit line length

- Format function signature and import in tab/registers

- Enforce CLAUDE.md import group order in dps-environment and dps-gas


### [CHK] Testing

- Add unit tests for gas.rs using approx and cargo-nextest

- Add doc tests for gas.rs and units.rs; split into lib+bin

- Add unit tests for units.rs macro operators to close coverage gaps

- Cover all label() match arms in gas.rs

- *(which_key)* Add unit tests for bottom_rect and widget rendering

- Add navigation, color, and status bar tests for ModTab and PpO2Tab

- Add tests for ui helpers, App::handle_key, and HintBar

- Add missing boundary and inverse-direction tests

- *(tui)* Add compile-check test for resume()

- *(tui)* Add Key and Mouse round-trip serde tests

- *(units)* Switch float assertions to assert_relative_eq

- *(gas)* Migrate tests to use rstest and ? operator

- *(keys)* Group tests by unit and migrate to rstest

- *(cli)* Migrate tests to use Result and rstest conventions

- *(app)* Migrate tests to use Result and rstest conventions

- *(action)* Migrate tests to use Result and rstest conventions

- *(mod_tab)* Migrate tests to use Result and rstest conventions

- *(config)* Migrate tests to use Result and rstest conventions

- *(tui)* Migrate tests to use Result and rstest conventions

- *(which_key)* Migrate tests to use Result and .as_slice() conventions

- *(ppo2_tab)* Migrate tests to Result, reorganize imports, apply formatting

- Add mutation-targeted tests and cargo-mutants infrastructure

- *(app)* Update test return types from String to Box<dyn Error>

- *(registers)* Migrate value tests to rstest and remove unwrap

- *(lake)* Restructure tests with rstest and nested submodules

- *(ocean)* Restructure tests with rstest and nested submodules

- *(environment)* Drop trivial display-string tests from error.rs

- *(environment)* Add public API accessibility tests for constants and error variants

- *(environment)* Add physical correctness tests against ISO/ICAO reference values

- *(environment)* Add exhaustive preset coverage tests for Ocean and Lake variants

- *(environment)* Add serde roundtrip tests for DiveEnvironment, Ocean, and Lake

- *(environment)* Add end-to-end workflow tests for consumer usage patterns

- *(gas)* Add serde roundtrip tests for all gas blend types

- *(gas)* Update serde tests to use PPO2Summary


### [SYS] Miscellaneous Tasks

- Add authors field to Cargo.toml for clap::crate_authors!()

- *(build)* Add release profile with LTO, size opt, and strip

- *(cargo)* Silence test output with quiet = true

- *(cargo)* Maximize release optimizations and strip all symbols

- *(lints)* Enable pedantic clippy lints and deny panic-prone patterns

- Normalize comment style in Cargo.toml to sentence case

- Add clippy.toml with complexity and doc-valid-idents config

- Add cargo-deny config with full coverage

- *(config)* Add theme and palette stubs to config.json5

- *(config)* Restructure theme config with defaultTheme, themes, palettes

- *(config)* Add latte, macchiato, mocha themes and palettes

- *(lint)* Suppress clippy::panic_in_result_fn in test builds

- Normalize version pinning and add cargo-audit config

- *(docs)* Mark doc examples no_run, skip doctests in mutants

- *(build)* Improve dev/test/mutants compile speed

- *(test)* Configure nextest to print totals only

- *(test)* Disable doctests by default, run with --doc to opt in

- Allow rustdoc::private_doc_tests lint; add cargo docs alias

- Bump Cargo.lock dependencies

- *(git)* Ignore Claude Code worktrees directory

- Ignore rustc ICE dump files

- Update Cargo.lock

- Update Cargo.lock

- *(units)* Suppress clippy::panic_in_result_fn in test cfg

- *(workspace)* Integrate dps-units, dps-environment, and dps-gas as workspace members

- *(libs)* Disable crate-level doctests in dps-units, dps-environment, dps-gas

- Add git-stats-per-day.sh for per-day commit activity analysis

- Update Cargo.lock for serde_json and ryu dependencies

- Bump tokio lock to 0.4.32

- Suppress false doc_markdown lints for KaTeX expressions

- *(gas)* Mark cns_limit_minutes as #[must_use]

- *(keymap)* Remove unused import of KeyBindingsBuilder

- *(gas)* Remove now-redundant lint suppressions

- *(git-stats)* Bump gix to 0.84, polars to 0.54, anstream to 1.0

- *(git-stats)* Adapt to gix 0.84 and polars 0.54 API changes

- *(cargo)* Drop mold linker and sccache build config

- *(cargo)* Update Cargo.lock dependency versions

- Add git-cliff configuration for changelog generation

- *(deny)* Move cargo-deny config to repo root and expand rules

- Add dual MIT/Apache-2.0 license files

- Relicense workspace crates as MIT OR Apache-2.0

- *(nix)* Add flake for reproducible dev shell and builds

- *(cargo)* Unify dependency versions via workspace.dependencies

- *(cargo)* Regenerate Cargo.lock for vergen 10 and workspace deps

- Add dependabot configuration

- Add ci workflow

- Add coverage workflow

- Add flake-lock-update workflow

- Add release-binaries workflow

- Add release-plz workflow

- Add release-plz config

- Add rust-toolchain config


### Deps

- Add katexit for KaTeX math rendering in rustdoc

- Add merge crate for config merging support


### Todo

- *(components)* Mark LineUp/LineDown fallback as needing proper scroll


<!-- generated by git-cliff -->
