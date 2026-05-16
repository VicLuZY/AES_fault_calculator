# FaultCalc Rust WASM Workstation

Offline portable short-circuit calculator seed package with a Rust calculation engine, native CLI, and browser-based WebAssembly workstation.

This is a professional seed implementation, not a sealed engineering product. It is intended to give Codex a strong starting point for a full IEEE-style fault-current calculator that can be used offline, versioned, tested, and extended.

## What is included

- Rust calculation core in `crates/faultcalc-core`
- Native CLI in `crates/faultcalc-cli`
- Direct Rust WebAssembly module in `crates/faultcalc-wasm`
- Professional one-line editor GUI in `web/index.template.html`
- External-WASM browser entry at `web/index.html`
- Standalone HTML generation workflow that embeds `faultcalc.wasm`
- Zoomable/pannable one-line canvas with snap-to-grid bus dragging, orthogonal branch routing, switch/breaker/tie symbols, equipment schedules, duty comparison, and print-friendly HTML reports
- Programmatic case model using JSON
- Sample case in `cases/sample.json`
- JSON and CSV report export
- Codex `/goal` instruction in `CODEX_GOAL.txt`

## Engineering method implemented

The core implements a transparent symmetrical-component network solution using per-unit impedances:

- positive, negative, and zero sequence networks
- source equivalents as shunts to the reference bus
- branches as series sequence impedances between buses
- driving-point Thevenin impedance solved at every bus
- 3-phase, single-line-ground, line-line, and double-line-ground faults
- base-current conversion to available symmetrical kA
- X/R from equivalent impedance
- formula-based peak and asymmetrical RMS estimates using the DC time constant

The implementation intentionally does not embed copyrighted IEEE or ANSI multiplying-factor tables. If the production calculator needs exact IEEE/ANSI/NEMA device-duty factors, those should be added as licensed external data or user-provided project factors.

## Build

Install Rust and the WASM target:

```bash
rustup target add wasm32-unknown-unknown
```

Run the full build:

```bash
make bundle
```

Create a portable release zip:

```bash
make release
```

Equivalent manual commands:

```bash
cargo test --workspace
cargo build -p faultcalc-cli --release
mkdir -p bin
cp target/release/faultcalc bin/faultcalc
cargo build -p faultcalc-wasm --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/faultcalc_wasm.wasm web/faultcalc.wasm
bin/faultcalc embed-wasm web/index.template.html web/faultcalc.wasm web/faultcalc_workstation.html
bin/faultcalc sample > cases/sample.json
```

Platform notes:

- macOS/Linux: install the stable Rust toolchain with `rustup`, then run the commands above from a terminal.
- Windows: install Rust through `rustup-init.exe`; run the same `cargo` commands from PowerShell. The `make` targets require GNU Make, so use the manual commands below if Make is not installed.
- The browser workstation has no runtime internet dependency. Development mode fetches the local `web/faultcalc.wasm`; standalone mode embeds that WASM directly in `web/faultcalc_workstation.html`.

## Use the CLI

```bash
cargo run -p faultcalc-cli -- sample > cases/sample.json
cargo run -p faultcalc-cli -- calc cases/sample.json --json out/report.json --csv out/report.csv
```

## Use the GUI

Development mode, served locally:

```bash
cargo run -p faultcalc-cli -- serve web 8080
```

Then open:

```text
http://127.0.0.1:8080
```

Standalone mode after `make standalone`:

```text
web/faultcalc_workstation.html
```

The standalone file embeds the Rust WASM module as base64, so it can be opened directly from disk without a server.

## GitHub Pages

This repo can publish the browser workstation as a static GitHub Pages site through `.github/workflows/pages.yml`.

On every push to `main`, the workflow:

- runs `cargo test --workspace`
- builds the Rust WASM module
- copies `faultcalc_wasm.wasm` to `web/faultcalc.wasm`
- generates `web/faultcalc_workstation.html`
- deploys the static `web/` directory to GitHub Pages

The hosted entry point is `index.html`, which loads `faultcalc.wasm` from the same static directory. The standalone `faultcalc_workstation.html` is also included in the Pages artifact.

## Case model

Top-level structure:

```json
{
  "project": { "name": "Project", "number": "", "engineer": "", "revision": "A" },
  "base_mva": 100.0,
  "frequency_hz": 60.0,
  "prefault_voltage_pu": 1.0,
  "options": { "fault_r_ohm": 0.0, "fault_x_ohm": 0.0, "duty_cycles": 0.5 },
  "buses": [],
  "branches": [],
  "sources": []
}
```

All impedances are stored in per-unit on the system `base_mva`. Each bus carries its own `kv_ll`, so the solver can report base current and convert fault impedance from ohms to per-unit at each bus.

## Programmatic use

```rust
use faultcalc_core::{calculate_all_buses, sample_network};

fn main() -> faultcalc_core::Result<()> {
    let net = sample_network();
    let report = calculate_all_buses(&net)?;
    println!("max 3PH kA = {:.2}", report.summary.max_3ph_ka);
    Ok(())
}
```

## Important validation notes

Before using this for issued work, Codex should extend the package with:

- utility source import workflow
- transformer connection and grounding templates
- conductor library and impedance base conversion
- motor and generator decrement modelling
- IEC 60909 module if required by project jurisdiction
- exact IEEE/ANSI device duty and momentary interrupting logic using licensed tables or verified formulae
- equipment-rating comparison and pass/fail schedules
- test cases against hand calculations and a known commercial tool

## Current limitations

- Equipment duty comparison uses available symmetrical current against user-entered ampere ratings; it does not replace project-specific interrupting, withstand, momentary, or series-rating checks.
- Conductor, transformer, motor, generator, and inverter builders are intentionally simple seed workflows unless project-licensed data is supplied.
- The HTML report includes the calculation method appendix and warnings, but PDF generation is performed through the browser print dialog.
