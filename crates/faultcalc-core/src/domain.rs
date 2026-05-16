use crate::{Branch, Impedance, Network, Result};
use serde::Serialize;
use std::collections::{HashSet, VecDeque};

const SQRT3: f64 = 1.732_050_807_568_877_2;
const DEFAULT_SECONDARY_KV: f64 = 0.6;

#[derive(Debug, Clone, Serialize)]
pub struct VoltageOption {
    pub key: &'static str,
    pub label: &'static str,
    pub display: &'static str,
    pub volts: f64,
    pub kv_ll: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BusDisplay {
    pub id: String,
    pub name: String,
    pub voltage_key: String,
    pub voltage_label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransformerDisplay {
    pub kva: f64,
    pub impedance_percent: f64,
    pub primary_rated_a: f64,
    pub secondary_rated_a: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BranchDisplay {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub connection: String,
    pub rating: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transformer: Option<TransformerDisplay>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScheduleRow {
    pub item_type: String,
    pub name: String,
    pub connection: String,
    pub rating: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DomainSummary {
    pub closed_branch_count: usize,
    pub enabled_source_count: usize,
    pub warning_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseDomainView {
    pub network: Network,
    pub voltage_options: Vec<VoltageOption>,
    pub buses: Vec<BusDisplay>,
    pub branches: Vec<BranchDisplay>,
    pub schedule: Vec<ScheduleRow>,
    pub warnings: Vec<String>,
    pub summary: DomainSummary,
}

pub fn case_domain_json(text: &str) -> Result<String> {
    let mut network = Network::from_json(text)?;
    normalise_case(&mut network);
    let view = case_domain_view(network);
    Ok(serde_json::to_string(&view)?)
}

pub fn voltage_options() -> Vec<VoltageOption> {
    voltage_specs()
        .iter()
        .map(|spec| VoltageOption {
            key: spec.key,
            label: spec.label,
            display: spec.display,
            volts: spec.volts(),
            kv_ll: spec.volts() / 1000.0,
        })
        .collect()
}

pub fn voltage_key(kv_ll: f64) -> String {
    let volts = kv_ll * 1000.0;
    voltage_specs()
        .iter()
        .min_by(|a, b| {
            let da = (a.volts() - volts).abs();
            let db = (b.volts() - volts).abs();
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|spec| spec.key.to_string())
        .unwrap_or_else(|| "600".to_string())
}

pub fn voltage_kv(key: &str) -> f64 {
    voltage_specs()
        .iter()
        .find(|spec| spec.key == key)
        .or_else(|| voltage_specs().iter().find(|spec| spec.key == "600"))
        .map(|spec| spec.volts() / 1000.0)
        .unwrap_or(DEFAULT_SECONDARY_KV)
}

pub fn normalize_voltage_kv(kv_ll: f64) -> f64 {
    round12(voltage_kv(&voltage_key(kv_ll)))
}

pub fn voltage_label(kv_ll: f64) -> String {
    let key = voltage_key(kv_ll);
    voltage_specs()
        .iter()
        .find(|spec| spec.key == key)
        .map(|spec| spec.display.to_string())
        .unwrap_or_else(|| format!("{} V", fmt0(kv_ll * 1000.0)))
}

pub fn normalise_case(network: &mut Network) {
    network.normalise_defaults();
    for bus in &mut network.buses {
        bus.kv_ll = normalize_voltage_kv(bus.kv_ll);
    }
    sync_conductor_voltage_groups(network);
    for idx in 0..network.branches.len() {
        sync_transformer_derived_values(network, idx);
    }
}

pub fn transformer_kva(network: &Network, branch: &Branch) -> f64 {
    if branch.rating_kva > 0.0 {
        return branch.rating_kva;
    }
    let sec_kv = bus_voltage(network, &branch.to);
    if branch.rating_a > 0.0 && sec_kv > 0.0 {
        branch.rating_a * SQRT3 * sec_kv
    } else {
        0.0
    }
}

pub fn transformer_percent_z(network: &Network, branch: &Branch) -> f64 {
    if branch.impedance_percent > 0.0 {
        return branch.impedance_percent;
    }
    let kva = transformer_kva(network, branch);
    let z = (branch.z1_pu.r * branch.z1_pu.r + branch.z1_pu.x * branch.z1_pu.x).sqrt();
    if kva > 0.0 && network.base_mva > 0.0 && z > 0.0 {
        z * (kva / 1000.0) / network.base_mva * 100.0
    } else {
        0.0
    }
}

pub fn transformer_rated_current(network: &Network, branch: &Branch, bus_id: &str) -> f64 {
    let kva = transformer_kva(network, branch);
    let kv = bus_voltage(network, bus_id);
    if kva > 0.0 && kv > 0.0 {
        kva / (SQRT3 * kv)
    } else {
        0.0
    }
}

pub fn impedance_from_percent(base_mva: f64, kva: f64, percent: f64, xr: f64) -> Impedance {
    let own_mva = kva / 1000.0;
    let magnitude = if own_mva > 0.0 {
        (percent / 100.0) * base_mva / own_mva
    } else {
        0.0
    };
    let ratio = if xr > 0.0 { xr } else { 7.0 };
    let r = magnitude / (1.0 + ratio * ratio).sqrt();
    Impedance { r, x: r * ratio }
}

fn case_domain_view(network: Network) -> CaseDomainView {
    let buses = network
        .buses
        .iter()
        .map(|bus| BusDisplay {
            id: bus.id.clone(),
            name: bus.display_name(),
            voltage_key: voltage_key(bus.kv_ll),
            voltage_label: voltage_label(bus.kv_ll),
        })
        .collect::<Vec<_>>();
    let branches = network
        .branches
        .iter()
        .map(|branch| branch_display(&network, branch))
        .collect::<Vec<_>>();
    let mut schedule = Vec::new();
    for bus in &network.buses {
        schedule.push(ScheduleRow {
            item_type: "Bus".to_string(),
            name: bus.display_name(),
            connection: voltage_label(bus.kv_ll),
            rating: String::new(),
        });
    }
    for branch in &branches {
        schedule.push(ScheduleRow {
            item_type: branch.kind.clone(),
            name: branch.name.clone(),
            connection: branch.connection.clone(),
            rating: branch.rating.clone(),
        });
    }
    for source in &network.sources {
        schedule.push(ScheduleRow {
            item_type: source.kind.clone(),
            name: if source.name.is_empty() {
                source.id.clone()
            } else {
                source.name.clone()
            },
            connection: bus_label(&network, &source.bus),
            rating: source.rating.clone(),
        });
    }
    let warnings = model_warnings(&network);
    CaseDomainView {
        summary: DomainSummary {
            closed_branch_count: network
                .branches
                .iter()
                .filter(|branch| !is_branch_open(branch))
                .count(),
            enabled_source_count: network
                .sources
                .iter()
                .filter(|source| source.kind != "load" && !is_source_open(source))
                .count(),
            warning_count: warnings.len(),
        },
        network,
        voltage_options: voltage_options(),
        buses,
        branches,
        schedule,
        warnings,
    }
}

fn branch_display(network: &Network, branch: &Branch) -> BranchDisplay {
    let name = if branch.name.is_empty() {
        branch.id.clone()
    } else {
        branch.name.clone()
    };
    let connection = if is_voltage_changing_branch(branch) {
        format!(
            "{} ({}) to {} ({})",
            bus_label(network, &branch.from),
            voltage_label(bus_voltage(network, &branch.from)),
            bus_label(network, &branch.to),
            voltage_label(bus_voltage(network, &branch.to))
        )
    } else {
        format!(
            "{} to {}",
            bus_label(network, &branch.from),
            bus_label(network, &branch.to)
        )
    };
    let transformer = if is_voltage_changing_branch(branch) {
        Some(TransformerDisplay {
            kva: transformer_kva(network, branch),
            impedance_percent: transformer_percent_z(network, branch),
            primary_rated_a: transformer_rated_current(network, branch, &branch.from),
            secondary_rated_a: transformer_rated_current(network, branch, &branch.to),
        })
    } else {
        None
    };
    let rating = if let Some(transformer) = &transformer {
        format!(
            "{} kVA / {}%Z / Pri {} A / Sec {} A",
            fmt0(transformer.kva),
            fmt2(transformer.impedance_percent),
            fmt_a(transformer.primary_rated_a),
            fmt_a(transformer.secondary_rated_a)
        )
    } else if branch.rating_a > 0.0 {
        format!("{} A", fmt0(branch.rating_a))
    } else {
        String::new()
    };
    BranchDisplay {
        id: branch.id.clone(),
        name,
        kind: branch.kind.clone(),
        connection,
        rating,
        transformer,
    }
}

fn model_warnings(network: &Network) -> Vec<String> {
    let mut out = Vec::new();
    if !network
        .sources
        .iter()
        .any(|source| source.kind != "load" && !is_source_open(source))
    {
        out.push("No enabled source equivalent is defined.".to_string());
    }
    let bus_ids = network
        .buses
        .iter()
        .map(|bus| bus.id.as_str())
        .collect::<HashSet<_>>();
    for branch in &network.branches {
        let branch_name = if branch.name.is_empty() {
            branch.id.as_str()
        } else {
            branch.name.as_str()
        };
        let from_exists = bus_ids.contains(branch.from.as_str());
        let to_exists = bus_ids.contains(branch.to.as_str());
        if !from_exists || !to_exists {
            out.push(format!("Branch {branch_name} references a missing bus."));
            continue;
        }
        if !is_branch_open(branch)
            && branch.rating_a <= 0.0
            && ["breaker", "switch", "tie", "feeder", "transformer"].contains(&branch.kind.as_str())
        {
            out.push(format!(
                "Branch {branch_name} has no equipment rating for duty comparison."
            ));
        }
        if !is_branch_open(branch) && !branch.has_z0 {
            out.push(format!("Branch {branch_name} has no zero-sequence path."));
        }
        if !is_voltage_changing_branch(branch)
            && voltage_key(bus_voltage(network, &branch.from))
                != voltage_key(bus_voltage(network, &branch.to))
        {
            out.push(format!(
                "Branch {branch_name} connects buses with different voltage ratings."
            ));
        }
        if is_voltage_changing_branch(branch) {
            let primary_cabled = has_cable_branch_at(network, &branch.from, &branch.id);
            let secondary_cabled = has_cable_branch_at(network, &branch.to, &branch.id);
            if !primary_cabled || !secondary_cabled {
                out.push(format!("Transformer {branch_name} shall connect to other elements through cable branches."));
            }
        }
    }
    for source in &network.sources {
        let name = if source.name.is_empty() {
            source.id.as_str()
        } else {
            source.name.as_str()
        };
        let label = if source.kind == "load" {
            "Load"
        } else {
            "Source"
        };
        if !bus_ids.contains(source.bus.as_str()) {
            out.push(format!("{label} {name} references a missing bus."));
        }
        if source.kind != "load" && !is_source_open(source) && source.rating.is_empty() {
            out.push(format!("Source {name} has no rating metadata."));
        }
        if source.kind != "load"
            && !is_source_open(source)
            && !source.has_z0
            && ["utility", "source"].contains(&source.kind.as_str())
        {
            out.push(format!("Source {name} has no zero-sequence data."));
        }
    }
    out
}

fn sync_transformer_derived_values(network: &mut Network, idx: usize) {
    if !is_voltage_changing_branch(&network.branches[idx]) {
        return;
    }
    let kva = transformer_kva(network, &network.branches[idx]);
    let percent = transformer_percent_z(network, &network.branches[idx]);
    let xr = if network.branches[idx].xr_ratio > 0.0 {
        network.branches[idx].xr_ratio
    } else {
        7.0
    };
    let secondary_a =
        transformer_rated_current(network, &network.branches[idx], &network.branches[idx].to);
    let z = impedance_from_percent(network.base_mva, kva, percent, xr);
    let branch = &mut network.branches[idx];
    if kva > 0.0 {
        branch.rating_kva = kva;
    }
    if secondary_a > 0.0 {
        branch.rating_a = secondary_a;
    }
    if percent > 0.0 && kva > 0.0 {
        branch.impedance_percent = percent;
        branch.xr_ratio = xr;
        branch.z1_pu = z;
        branch.z2_pu = z;
        if branch.has_z0 {
            branch.z0_pu = z;
        }
    }
}

fn sync_conductor_voltage_groups(network: &mut Network) {
    let mut visited = HashSet::new();
    let bus_ids = network
        .buses
        .iter()
        .map(|bus| bus.id.clone())
        .collect::<Vec<_>>();
    for start in bus_ids {
        if visited.contains(&start) {
            continue;
        }
        let mut members = Vec::new();
        let mut queue = VecDeque::from([start.clone()]);
        while let Some(id) = queue.pop_front() {
            if !visited.insert(id.clone()) {
                continue;
            }
            members.push(id.clone());
            for branch in &network.branches {
                if is_voltage_changing_branch(branch) {
                    continue;
                }
                if branch.from == id && !visited.contains(&branch.to) {
                    queue.push_back(branch.to.clone());
                }
                if branch.to == id && !visited.contains(&branch.from) {
                    queue.push_back(branch.from.clone());
                }
            }
        }
        let anchor = members
            .first()
            .map(|id| bus_voltage(network, id))
            .unwrap_or(DEFAULT_SECONDARY_KV);
        for id in members {
            if let Some(bus) = network.buses.iter_mut().find(|bus| bus.id == id) {
                bus.kv_ll = normalize_voltage_kv(anchor);
            }
        }
    }
}

fn bus_voltage(network: &Network, id: &str) -> f64 {
    network
        .buses
        .iter()
        .find(|bus| bus.id == id)
        .map(|bus| bus.kv_ll)
        .unwrap_or(DEFAULT_SECONDARY_KV)
}

fn bus_label(network: &Network, id: &str) -> String {
    network
        .buses
        .iter()
        .find(|bus| bus.id == id)
        .map(|bus| bus.display_name())
        .unwrap_or_else(|| id.to_string())
}

fn is_voltage_changing_branch(branch: &Branch) -> bool {
    branch.kind == "transformer"
}

fn is_branch_open(branch: &Branch) -> bool {
    !branch.enabled || branch.state.contains("open") || branch.state == "out_of_service"
}

fn is_source_open(source: &crate::Source) -> bool {
    !source.enabled || source.state.contains("open") || source.state == "out_of_service"
}

fn has_cable_branch_at(network: &Network, bus_id: &str, transformer_id: &str) -> bool {
    network.branches.iter().any(|branch| {
        branch.id != transformer_id
            && branch.kind != "transformer"
            && (branch.from == bus_id || branch.to == bus_id)
    })
}

fn fmt0(value: f64) -> String {
    format!("{value:.0}")
}

fn fmt2(value: f64) -> String {
    format!("{value:.2}")
}

fn fmt_a(value: f64) -> String {
    if value <= 0.0 {
        String::new()
    } else if value >= 100.0 {
        format!("{value:.0}")
    } else if value >= 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    }
}

fn round12(value: f64) -> f64 {
    (value * 1_000_000_000_000.0).round() / 1_000_000_000_000.0
}

#[derive(Debug, Clone, Copy)]
struct VoltageSpec {
    key: &'static str,
    label: &'static str,
    display: &'static str,
    base_volts: f64,
    sqrt3: bool,
}

impl VoltageSpec {
    fn volts(self) -> f64 {
        if self.sqrt3 {
            self.base_volts * SQRT3
        } else {
            self.base_volts
        }
    }
}

fn voltage_specs() -> &'static [VoltageSpec] {
    &[
        VoltageSpec {
            key: "120",
            label: "120 V",
            display: "120 V",
            base_volts: 120.0,
            sqrt3: false,
        },
        VoltageSpec {
            key: "120r3",
            label: "120*sqrt(3) V (208 V)",
            display: "208 V",
            base_volts: 120.0,
            sqrt3: true,
        },
        VoltageSpec {
            key: "240",
            label: "240 V",
            display: "240 V",
            base_volts: 240.0,
            sqrt3: false,
        },
        VoltageSpec {
            key: "160r3",
            label: "160*sqrt(3) V (277 V)",
            display: "277 V",
            base_volts: 160.0,
            sqrt3: true,
        },
        VoltageSpec {
            key: "200r3",
            label: "200*sqrt(3) V (347 V)",
            display: "347 V",
            base_volts: 200.0,
            sqrt3: true,
        },
        VoltageSpec {
            key: "480",
            label: "480 V",
            display: "480 V",
            base_volts: 480.0,
            sqrt3: false,
        },
        VoltageSpec {
            key: "600",
            label: "600 V",
            display: "600 V",
            base_volts: 600.0,
            sqrt3: false,
        },
        VoltageSpec {
            key: "2400",
            label: "2400 V",
            display: "2.4 kV",
            base_volts: 2400.0,
            sqrt3: false,
        },
        VoltageSpec {
            key: "2400r3",
            label: "2400*sqrt(3) V (4.16 kV)",
            display: "4.16 kV",
            base_volts: 2400.0,
            sqrt3: true,
        },
        VoltageSpec {
            key: "7200r3",
            label: "7200*sqrt(3) V (12.5 kV)",
            display: "12.5 kV",
            base_volts: 7200.0,
            sqrt3: true,
        },
        VoltageSpec {
            key: "8000r3",
            label: "8000*sqrt(3) V (13.8 kV)",
            display: "13.8 kV",
            base_volts: 8000.0,
            sqrt3: true,
        },
        VoltageSpec {
            key: "14400r3",
            label: "14400*sqrt(3) V (25 kV)",
            display: "25 kV",
            base_volts: 14400.0,
            sqrt3: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample_network;

    #[test]
    fn voltage_options_carry_sqrt3_values() {
        assert_eq!(voltage_label(12.47), "12.5 kV");
        assert_eq!(voltage_key(25.0), "14400r3");
        assert!((voltage_kv("160r3") - 0.277_128_129_211).abs() < 1e-12);
    }

    #[test]
    fn domain_view_normalises_transformer_and_warnings_in_rust() {
        let mut network = sample_network();
        network.add_bus("pri", "Primary", 0.6, 0.0, 0.0).unwrap();
        network.add_bus("sec", "Secondary", 0.6, 0.0, 0.0).unwrap();
        network
            .add_transformer(
                "tx",
                "Transformer",
                "pri",
                "sec",
                2500.0,
                0.6,
                5.75,
                7.0,
                5.75,
                true,
            )
            .unwrap();
        normalise_case(&mut network);
        let view = case_domain_view(network);
        let tx = view
            .branches
            .iter()
            .find(|branch| branch.id == "tx")
            .unwrap();
        assert!(tx.rating.contains("2500 kVA"));
        assert!(view
            .warnings
            .iter()
            .any(|warning| warning.contains("cable branches")));
    }

    #[test]
    fn case_domain_json_round_trips_sample() {
        let json = crate::sample_json().unwrap();
        let view: serde_json::Value =
            serde_json::from_str(&case_domain_json(&json).unwrap()).unwrap();
        assert_eq!(view["network"]["buses"][0]["kv_ll"], 0.6);
        assert!(view["voltage_options"].as_array().unwrap().len() >= 10);
    }
}
