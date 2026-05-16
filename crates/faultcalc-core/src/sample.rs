use crate::{Branch, Impedance, Network, ProjectInfo, Result, Source};

pub fn sample_network() -> Network {
    let mut n = Network::new(100.0);
    n.project = ProjectInfo {
        name: "AES sample short-circuit study".to_string(),
        number: "SEED-001".to_string(),
        engineer: "Victor Lü".to_string(),
        revision: "A".to_string(),
        description: "Illustrative utility-transformer-MSB-MCC topology for validating the Rust WASM workstation.".to_string(),
    };
    n.add_bus("util_12k", "Utility 12.47 kV", 12.47, 150.0, 245.0).unwrap();
    n.add_bus("msb_600", "Main Switchboard", 0.6, 445.0, 245.0).unwrap();
    n.add_bus("mcc_600", "Mechanical MCC", 0.6, 745.0, 245.0).unwrap();
    n.add_utility_source("src_utility", "Utility source equivalent", "util_12k", 500.0, 10.0, 1.0).unwrap();
    n.add_transformer("tx_1", "2500 kVA transformer", "util_12k", "msb_600", 2500.0, 0.6, 5.75, 7.0, 5.75, true).unwrap();
    n.add_branch(Branch {
        id: "fdr_mcc".to_string(),
        kind: "feeder".to_string(),
        name: "MSB to MCC feeder".to_string(),
        from: "msb_600".to_string(),
        to: "mcc_600".to_string(),
        conductors: Default::default(),
        primary_connection: String::new(),
        secondary_connection: String::new(),
        vector_shift_deg: 0.0,
        z1_pu: Impedance::new(0.00080, 0.00230),
        z2_pu: Impedance::new(0.00080, 0.00230),
        z0_pu: Impedance::new(0.00240, 0.00690),
        has_z0: true,
        enabled: true,
        rating_a: 800.0,
        length_m: 55.0,
        notes: "Illustrative per-unit feeder impedance on 100 MVA base.".to_string(),
    }).unwrap();
    n.add_source(Source {
        id: "mot_mcc".to_string(),
        kind: "motor".to_string(),
        name: "Aggregate motor contribution".to_string(),
        bus: "mcc_600".to_string(),
        z1_pu: Impedance::new(18.0, 95.0),
        z2_pu: Impedance::new(18.0, 95.0),
        z0_pu: Impedance::new(25.0, 120.0),
        has_z0: false,
        enabled: true,
        rating: "aggregate motor source".to_string(),
        notes: "Replace with project-specific motor contribution data.".to_string(),
    }).unwrap();
    n.notes.push("This sample is for software validation and workflow demonstration only.".to_string());
    n
}

pub fn sample_json() -> Result<String> {
    Ok(serde_json::to_string_pretty(&sample_network())?)
}
