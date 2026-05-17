use crate::complex::Complex;
use crate::{FaultCalcError, Result};
use serde::{Deserialize, Serialize};

fn default_frequency() -> f64 {
    60.0
}
fn default_prefault() -> f64 {
    1.0
}
fn default_duty_cycles() -> f64 {
    0.5
}
fn default_enabled() -> bool {
    true
}

pub const DEFAULT_SECONDARY_KV_LL: f64 = 0.6;
pub const DEFAULT_UTILITY_KV_LL: f64 = 12.470_765_814_495_916;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Network {
    #[serde(default)]
    pub project: ProjectInfo,
    pub base_mva: f64,
    #[serde(default = "default_frequency")]
    pub frequency_hz: f64,
    #[serde(default = "default_prefault")]
    pub prefault_voltage_pu: f64,
    #[serde(default)]
    pub options: CalculationOptions,
    #[serde(default)]
    pub buses: Vec<Bus>,
    #[serde(default)]
    pub branches: Vec<Branch>,
    #[serde(default)]
    pub sources: Vec<Source>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectInfo {
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub number: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub engineer: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub revision: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculationOptions {
    #[serde(default)]
    pub fault_r_ohm: f64,
    #[serde(default)]
    pub fault_x_ohm: f64,
    #[serde(default = "default_duty_cycles")]
    pub duty_cycles: f64,
}

impl Default for CalculationOptions {
    fn default() -> Self {
        Self {
            fault_r_ohm: 0.0,
            fault_x_ohm: 0.0,
            duty_cycles: 0.5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bus {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub kv_ll: f64,
    #[serde(default, skip_serializing)]
    pub x: f64,
    #[serde(default, skip_serializing)]
    pub y: f64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub id: String,
    #[serde(default = "default_branch_kind")]
    pub kind: String,
    #[serde(default)]
    pub name: String,
    pub from: String,
    pub to: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub state: String,
    #[serde(default, skip_serializing_if = "ConductorSet::is_empty")]
    pub conductors: ConductorSet,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub primary_connection: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub secondary_connection: String,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub vector_shift_deg: f64,
    pub z1_pu: Impedance,
    pub z2_pu: Impedance,
    #[serde(default)]
    pub z0_pu: Impedance,
    #[serde(default)]
    pub has_z0: bool,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rating_a: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rating_kva: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub impedance_percent: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub xr_ratio: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub length_m: f64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

fn default_branch_kind() -> String {
    "impedance".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    #[serde(default = "default_source_kind")]
    pub kind: String,
    #[serde(default)]
    pub name: String,
    pub bus: String,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub kv_ll: f64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub state: String,
    pub z1_pu: Impedance,
    pub z2_pu: Impedance,
    #[serde(default)]
    pub z0_pu: Impedance,
    #[serde(default)]
    pub has_z0: bool,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub rating: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

fn default_source_kind() -> String {
    "source".to_string()
}
fn is_zero(v: &f64) -> bool {
    v.abs() < 1e-15
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Impedance {
    pub r: f64,
    pub x: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConductorSet {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub phases: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub neutral: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ground: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bond: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub other: Vec<String>,
}

impl ConductorSet {
    pub fn is_empty(&self) -> bool {
        self.phases.is_empty()
            && self.neutral.is_empty()
            && self.ground.is_empty()
            && self.bond.is_empty()
            && self.other.is_empty()
    }
}

impl Impedance {
    pub fn new(r: f64, x: f64) -> Self {
        Self { r, x }
    }
    pub fn to_complex(self) -> Complex {
        Complex::new(self.r, self.x)
    }
    pub fn from_complex(z: Complex) -> Self {
        Self { r: z.re, x: z.im }
    }
}

impl Network {
    pub fn new(base_mva: f64) -> Self {
        Self {
            project: ProjectInfo {
                name: "Untitled short-circuit study".to_string(),
                revision: "A".to_string(),
                ..ProjectInfo::default()
            },
            base_mva,
            frequency_hz: 60.0,
            prefault_voltage_pu: 1.0,
            options: CalculationOptions::default(),
            buses: Vec::new(),
            branches: Vec::new(),
            sources: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn from_json(data: &str) -> Result<Self> {
        let mut net: Self = serde_json::from_str(data)?;
        net.normalise_defaults();
        Ok(net)
    }

    pub fn to_json_pretty(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn normalise_defaults(&mut self) {
        if self.frequency_hz == 0.0 {
            self.frequency_hz = 60.0;
        }
        if self.prefault_voltage_pu == 0.0 {
            self.prefault_voltage_pu = 1.0;
        }
        if self.options.duty_cycles == 0.0 {
            self.options.duty_cycles = 0.5;
        }
        for b in &mut self.branches {
            if b.kind.is_empty() {
                b.kind = default_branch_kind();
            }
            b.normalise_metadata_defaults();
            if b.state.is_empty() {
                b.state = if b.enabled {
                    "normally_closed"
                } else {
                    "normally_open"
                }
                .to_string();
            } else {
                b.enabled = !b.state.contains("open") && b.state != "out_of_service";
            }
        }
        let bus_voltage = self
            .buses
            .iter()
            .map(|bus| (bus.id.clone(), bus.kv_ll))
            .collect::<Vec<_>>();
        for s in &mut self.sources {
            if s.kind.is_empty() {
                s.kind = default_source_kind();
            }
            if s.kv_ll <= 0.0 {
                s.kv_ll = bus_voltage
                    .iter()
                    .find(|(id, _)| id == &s.bus)
                    .map(|(_, kv_ll)| *kv_ll)
                    .unwrap_or_else(|| default_kv_ll_for_source_kind(&s.kind));
            }
            if s.state.is_empty() {
                s.state = if s.enabled {
                    "in_service"
                } else {
                    "out_of_service"
                }
                .to_string();
            } else {
                s.enabled = !s.state.contains("open") && s.state != "out_of_service";
            }
        }
    }

    pub fn add_bus(&mut self, id: &str, name: &str, kv_ll: f64, x: f64, y: f64) -> Result<()> {
        if id.trim().is_empty() {
            return Err(FaultCalcError::new("bus id cannot be blank"));
        }
        if kv_ll <= 0.0 {
            return Err(FaultCalcError::new(format!("bus {id} has non-positive kV")));
        }
        if self.bus_index(id).is_some() {
            return Err(FaultCalcError::new(format!("duplicate bus id {id}")));
        }
        self.buses.push(Bus {
            id: id.to_string(),
            name: name.to_string(),
            kv_ll,
            x,
            y,
            notes: String::new(),
        });
        Ok(())
    }

    pub fn add_branch(&mut self, mut branch: Branch) -> Result<()> {
        if branch.id.trim().is_empty() {
            return Err(FaultCalcError::new("branch id cannot be blank"));
        }
        if self.branch_index(&branch.id).is_some() {
            return Err(FaultCalcError::new(format!(
                "duplicate branch id {}",
                branch.id
            )));
        }
        if self.bus_index(&branch.from).is_none() {
            return Err(FaultCalcError::new(format!(
                "branch {} references missing from bus {}",
                branch.id, branch.from
            )));
        }
        if self.bus_index(&branch.to).is_none() {
            return Err(FaultCalcError::new(format!(
                "branch {} references missing to bus {}",
                branch.id, branch.to
            )));
        }
        if branch.from == branch.to {
            return Err(FaultCalcError::new(format!(
                "branch {} connects a bus to itself",
                branch.id
            )));
        }
        if branch.kind.is_empty() {
            branch.kind = default_branch_kind();
        }
        branch.normalise_metadata_defaults();
        self.branches.push(branch);
        Ok(())
    }

    pub fn add_source(&mut self, mut source: Source) -> Result<()> {
        if source.id.trim().is_empty() {
            return Err(FaultCalcError::new("source id cannot be blank"));
        }
        if self.source_index(&source.id).is_some() {
            return Err(FaultCalcError::new(format!(
                "duplicate source id {}",
                source.id
            )));
        }
        if self.bus_index(&source.bus).is_none() {
            return Err(FaultCalcError::new(format!(
                "source {} references missing bus {}",
                source.id, source.bus
            )));
        }
        if source.kind.is_empty() {
            source.kind = default_source_kind();
        }
        if source.kv_ll <= 0.0 {
            source.kv_ll = self
                .buses
                .iter()
                .find(|bus| bus.id == source.bus)
                .map(|bus| bus.kv_ll)
                .unwrap_or_else(|| default_kv_ll_for_source_kind(&source.kind));
        }
        self.sources.push(source);
        Ok(())
    }

    pub fn add_utility_source(
        &mut self,
        id: &str,
        name: &str,
        bus_id: &str,
        short_circuit_mva: f64,
        xr: f64,
        z0_multiplier: f64,
    ) -> Result<()> {
        if short_circuit_mva <= 0.0 {
            return Err(FaultCalcError::new(format!(
                "utility source {id} short-circuit MVA must be positive"
            )));
        }
        let xr = if xr <= 0.0 { 10.0 } else { xr };
        let mag = self.base_mva / short_circuit_mva;
        let z1 = z_from_magnitude_xr(mag, xr);
        let z0 = if z0_multiplier <= 0.0 {
            z1
        } else {
            z1.scale(z0_multiplier)
        };
        self.add_source(Source {
            id: id.to_string(),
            kind: "utility".to_string(),
            name: name.to_string(),
            bus: bus_id.to_string(),
            kv_ll: self
                .buses
                .iter()
                .find(|bus| bus.id == bus_id)
                .map(|bus| bus.kv_ll)
                .unwrap_or(DEFAULT_UTILITY_KV_LL),
            state: "in_service".to_string(),
            z1_pu: Impedance::from_complex(z1),
            z2_pu: Impedance::from_complex(z1),
            z0_pu: Impedance::from_complex(z0),
            has_z0: true,
            enabled: true,
            rating: format!("{short_circuit_mva:.0} MVA, X/R {xr:.1}"),
            notes: String::new(),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_transformer(
        &mut self,
        id: &str,
        name: &str,
        from: &str,
        to: &str,
        kva: f64,
        kv_ll: f64,
        impedance_percent: f64,
        xr: f64,
        z0_percent: f64,
        grounded: bool,
    ) -> Result<()> {
        if kva <= 0.0 {
            return Err(FaultCalcError::new(format!(
                "transformer {id} kVA must be positive"
            )));
        }
        if impedance_percent <= 0.0 {
            return Err(FaultCalcError::new(format!(
                "transformer {id} impedance percent must be positive"
            )));
        }
        let xr = if xr <= 0.0 { 7.0 } else { xr };
        let base_on_own_mva = kva / 1000.0;
        let z_pu = (impedance_percent / 100.0) * (self.base_mva / base_on_own_mva);
        let z1 = z_from_magnitude_xr(z_pu, xr);
        let z0 = if grounded && z0_percent > 0.0 {
            z_from_magnitude_xr((z0_percent / 100.0) * (self.base_mva / base_on_own_mva), xr)
        } else {
            z1
        };
        let rating_a = kva * 1000.0 / (3.0_f64.sqrt() * kv_ll * 1000.0);
        self.add_branch(Branch {
            id: id.to_string(),
            kind: "transformer".to_string(),
            name: name.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            state: "normally_closed".to_string(),
            conductors: ConductorSet::default(),
            primary_connection: "delta".to_string(),
            secondary_connection: "grounded_wye".to_string(),
            vector_shift_deg: -30.0,
            z1_pu: Impedance::from_complex(z1),
            z2_pu: Impedance::from_complex(z1),
            z0_pu: Impedance::from_complex(z0),
            has_z0: grounded,
            enabled: true,
            rating_a,
            rating_kva: kva,
            impedance_percent,
            xr_ratio: xr,
            length_m: 0.0,
            notes: format!("{kva:.0} kVA, {impedance_percent:.2}%Z"),
        })
    }

    pub fn bus_index(&self, id: &str) -> Option<usize> {
        self.buses.iter().position(|b| b.id == id)
    }
    pub fn branch_index(&self, id: &str) -> Option<usize> {
        self.branches.iter().position(|b| b.id == id)
    }
    pub fn source_index(&self, id: &str) -> Option<usize> {
        self.sources.iter().position(|s| s.id == id)
    }
    pub fn bus_name(&self, id: &str) -> String {
        self.bus_index(id)
            .map(|i| self.buses[i].display_name())
            .unwrap_or_else(|| id.to_string())
    }

    pub fn z_base_ohm(&self, bus_index: usize) -> f64 {
        let kv = self.buses[bus_index].kv_ll;
        kv * kv / self.base_mva
    }

    pub fn i_base_ka(&self, bus_index: usize) -> f64 {
        self.base_mva / (3.0_f64.sqrt() * self.buses[bus_index].kv_ll)
    }

    pub fn validate(&self) -> Result<()> {
        if self.base_mva <= 0.0 {
            return Err(FaultCalcError::new("base_mva must be positive"));
        }
        if self.frequency_hz <= 0.0 {
            return Err(FaultCalcError::new("frequency_hz must be positive"));
        }
        if self.prefault_voltage_pu <= 0.0 {
            return Err(FaultCalcError::new("prefault_voltage_pu must be positive"));
        }
        if self.buses.is_empty() {
            return Err(FaultCalcError::new("network has no buses"));
        }
        if self.sources.iter().filter(|s| s.enabled).count() == 0 {
            return Err(FaultCalcError::new("network has no enabled sources"));
        }
        let mut seen = std::collections::HashSet::new();
        for b in &self.buses {
            if b.id.trim().is_empty() {
                return Err(FaultCalcError::new("bus id cannot be blank"));
            }
            if !seen.insert(b.id.as_str()) {
                return Err(FaultCalcError::new(format!("duplicate bus id {}", b.id)));
            }
            if b.kv_ll <= 0.0 {
                return Err(FaultCalcError::new(format!(
                    "bus {} has non-positive kv_ll",
                    b.id
                )));
            }
        }
        for br in &self.branches {
            if !br.enabled {
                continue;
            }
            if self.bus_index(&br.from).is_none() || self.bus_index(&br.to).is_none() {
                return Err(FaultCalcError::new(format!(
                    "branch {} references a missing bus",
                    br.id
                )));
            }
            if br.from == br.to {
                return Err(FaultCalcError::new(format!(
                    "branch {} connects a bus to itself",
                    br.id
                )));
            }
            if br.z1_pu.to_complex().abs() < 1e-14 || br.z2_pu.to_complex().abs() < 1e-14 {
                return Err(FaultCalcError::new(format!(
                    "branch {} has near-zero positive or negative sequence impedance",
                    br.id
                )));
            }
        }
        for source in &self.sources {
            if !source.enabled {
                continue;
            }
            let bus_index = match self.bus_index(&source.bus) {
                Some(index) => index,
                None => {
                    return Err(FaultCalcError::new(format!(
                        "source {} references a missing bus",
                        source.id
                    )));
                }
            };
            if source.kv_ll <= 0.0 {
                return Err(FaultCalcError::new(format!(
                    "source {} has non-positive kv_ll",
                    source.id
                )));
            }
            if (self.buses[bus_index].kv_ll - source.kv_ll).abs() > 1e-9 {
                return Err(FaultCalcError::new(format!(
                    "source {} voltage rating does not match bus {}",
                    source.id, source.bus
                )));
            }
            if source.z1_pu.to_complex().abs() < 1e-14 || source.z2_pu.to_complex().abs() < 1e-14 {
                return Err(FaultCalcError::new(format!(
                    "source {} has near-zero positive or negative sequence impedance",
                    source.id
                )));
            }
        }
        Ok(())
    }
}

impl Bus {
    pub fn display_name(&self) -> String {
        if self.name.is_empty() {
            self.id.clone()
        } else {
            self.name.clone()
        }
    }
}

impl Branch {
    pub fn normalise_metadata_defaults(&mut self) {
        if self.kind == "transformer" {
            if self.primary_connection.is_empty() {
                self.primary_connection = "delta".to_string();
            }
            if self.secondary_connection.is_empty() {
                self.secondary_connection = "grounded_wye".to_string();
            }
            if self.vector_shift_deg == 0.0 {
                self.vector_shift_deg = default_transformer_vector_shift(
                    &self.primary_connection,
                    &self.secondary_connection,
                );
            }
        }
        if self.conductors.is_empty() {
            self.conductors = default_conductors_for_branch(&self.kind, &self.secondary_connection);
        }
    }
}

fn default_transformer_vector_shift(primary: &str, secondary: &str) -> f64 {
    let p_delta = primary == "delta";
    let s_delta = secondary == "delta";
    match (p_delta, s_delta) {
        (true, false) => -30.0,
        (false, true) => 30.0,
        _ => 0.0,
    }
}

fn default_conductors_for_branch(kind: &str, secondary_connection: &str) -> ConductorSet {
    let neutral = if kind == "transformer" && secondary_connection == "delta" {
        Vec::new()
    } else if kind == "transformer" {
        vec!["X0/N".to_string()]
    } else {
        vec!["N".to_string()]
    };
    ConductorSet {
        phases: vec!["A".to_string(), "B".to_string(), "C".to_string()],
        neutral,
        ground: vec!["EGC".to_string()],
        bond: vec!["BOND".to_string()],
        other: Vec::new(),
    }
}

fn default_kv_ll_for_source_kind(kind: &str) -> f64 {
    if kind == "load" {
        DEFAULT_SECONDARY_KV_LL
    } else {
        DEFAULT_UTILITY_KV_LL
    }
}

pub fn z_from_magnitude_xr(magnitude: f64, xr: f64) -> Complex {
    if magnitude == 0.0 {
        return Complex::ZERO;
    }
    if xr.is_infinite() {
        return Complex::new(0.0, magnitude);
    }
    let r = magnitude / (1.0 + xr * xr).sqrt();
    Complex::new(r, xr * r)
}

pub fn x_over_r_abs(z: Complex) -> Option<f64> {
    let r = z.re;
    let x = z.im;
    if x.abs() < 1e-18 {
        Some(0.0)
    } else if r.abs() < 1e-18 {
        Some(1.0e9)
    } else {
        Some((x / r).abs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample_network;

    fn assert_close(actual: f64, expected: f64, tol: f64) {
        assert!(
            (actual - expected).abs() <= tol,
            "actual {actual} expected {expected} tol {tol}"
        );
    }

    #[test]
    fn per_unit_base_conversions_use_bus_voltage_and_system_mva() {
        let mut net = Network::new(100.0);
        net.add_bus("b1", "13.8 kV bus", 13.8, 0.0, 0.0).unwrap();
        net.add_bus("b2", "600 V bus", 0.6, 0.0, 0.0).unwrap();

        assert_close(net.z_base_ohm(0), 1.9044, 1e-12);
        assert_close(net.i_base_ka(0), 4.183697602823375, 1e-12);
        assert_close(net.z_base_ohm(1), 0.0036, 1e-15);
        assert_close(net.i_base_ka(1), 96.22504486493763, 1e-12);
    }

    #[test]
    fn utility_source_builder_converts_short_circuit_mva_and_xr() {
        let mut net = Network::new(100.0);
        net.add_bus("util", "Utility", 12.47, 0.0, 0.0).unwrap();
        net.add_utility_source("u1", "Utility", "util", 500.0, 10.0, 1.5)
            .unwrap();

        let source = &net.sources[0];
        assert_eq!(source.kind, "utility");
        assert!(source.enabled);
        assert!(source.has_z0);
        assert_close(source.z1_pu.r, 0.019900743804199785, 1e-15);
        assert_close(source.z1_pu.x, 0.19900743804199786, 1e-15);
        assert_close(source.z0_pu.r, 0.029851115706299677, 1e-15);
        assert_close(source.z0_pu.x, 0.2985111570629968, 1e-15);
    }

    #[test]
    fn transformer_builder_converts_percent_impedance_to_system_base() {
        let mut net = Network::new(100.0);
        net.add_bus("pri", "Primary", 12.47, 0.0, 0.0).unwrap();
        net.add_bus("sec", "Secondary", 0.6, 0.0, 0.0).unwrap();
        net.add_transformer(
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

        let branch = &net.branches[0];
        assert_eq!(branch.kind, "transformer");
        assert!(branch.has_z0);
        assert_eq!(branch.primary_connection, "delta");
        assert_eq!(branch.secondary_connection, "grounded_wye");
        assert_close(branch.vector_shift_deg, -30.0, 1e-12);
        assert_eq!(
            branch.conductors.phases,
            vec!["A".to_string(), "B".to_string(), "C".to_string()]
        );
        assert_eq!(branch.conductors.neutral, vec!["X0/N".to_string()]);
        assert_close(branch.z1_pu.r, 0.3252691193458119, 1e-15);
        assert_close(branch.z1_pu.x, 2.276883835420683, 1e-15);
        assert_close(branch.rating_a, 2405.626121623441, 1e-12);
    }

    #[test]
    fn json_import_export_round_trip_preserves_case_model() {
        let net = sample_network();
        let json = net.to_json_pretty().unwrap();
        let case_value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(case_value["buses"]
            .as_array()
            .unwrap()
            .iter()
            .all(|b| b.get("x").is_none() && b.get("y").is_none()));
        assert_close(
            case_value["buses"][0]["kv_ll"].as_f64().unwrap(),
            DEFAULT_UTILITY_KV_LL,
            1e-12,
        );
        assert_eq!(case_value["branches"].as_array().unwrap().len(), 0);
        assert_close(
            case_value["sources"][0]["kv_ll"].as_f64().unwrap(),
            DEFAULT_UTILITY_KV_LL,
            1e-12,
        );
        assert_eq!(case_value["sources"][0]["rating"], "infinite utility");
        let parsed = Network::from_json(&json).unwrap();

        assert_eq!(parsed.project.name, net.project.name);
        assert_eq!(parsed.buses.len(), net.buses.len());
        assert_eq!(parsed.branches.len(), net.branches.len());
        assert_eq!(parsed.sources.len(), net.sources.len());
        assert_close(parsed.base_mva, net.base_mva, 1e-12);
        assert_close(parsed.sources[0].z1_pu.r, net.sources[0].z1_pu.r, 1e-15);
        assert_close(parsed.sources[0].z1_pu.x, net.sources[0].z1_pu.x, 1e-15);
        assert_eq!(
            Network::from_json(&parsed.to_json_pretty().unwrap())
                .unwrap()
                .buses
                .len(),
            net.buses.len()
        );
    }

    #[test]
    fn x_over_r_handles_resistive_reactive_and_signed_values() {
        assert_eq!(x_over_r_abs(Complex::new(4.0, 0.0)), Some(0.0));
        assert_eq!(x_over_r_abs(Complex::new(0.0, 4.0)), Some(1.0e9));
        assert_eq!(x_over_r_abs(Complex::new(-2.0, 8.0)), Some(4.0));
    }
}
