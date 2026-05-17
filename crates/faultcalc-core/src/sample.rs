use crate::{Impedance, Network, ProjectInfo, Result, Source, DEFAULT_UTILITY_KV_LL};

pub fn sample_network() -> Network {
    let mut n = Network::new(100.0);
    n.project = ProjectInfo {
        name: "AES simple SLD study".to_string(),
        number: "SEED-001".to_string(),
        engineer: "Victor Lü".to_string(),
        revision: "A".to_string(),
        description: "Single infinite utility starter model for the browser SLD workstation."
            .to_string(),
    };
    n.add_bus(
        "util_bus",
        "Utility bus",
        DEFAULT_UTILITY_KV_LL,
        360.0,
        320.0,
    )
    .unwrap();
    n.add_source(Source {
        id: "src_inf".to_string(),
        kind: "utility".to_string(),
        name: "Infinite utility".to_string(),
        bus: "util_bus".to_string(),
        kv_ll: DEFAULT_UTILITY_KV_LL,
        state: "in_service".to_string(),
        z1_pu: Impedance::new(0.000001, 0.0001),
        z2_pu: Impedance::new(0.000001, 0.0001),
        z0_pu: Impedance::new(0.000001, 0.0001),
        has_z0: true,
        enabled: true,
        rating: "infinite utility".to_string(),
        notes: "Replace with project-specific utility source impedance before issued work."
            .to_string(),
    })
    .unwrap();
    n.notes
        .push("Starter case for a simple single-line diagram.".to_string());
    n
}

pub fn sample_json() -> Result<String> {
    Ok(serde_json::to_string_pretty(&sample_network())?)
}
