use crate::complex::Complex;
use crate::model::{x_over_r_abs, Impedance, Network};
use crate::{FaultCalcError, Result, VERSION};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::f64::consts::PI;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub version: String,
    pub project: crate::ProjectInfo,
    pub base_mva: f64,
    pub frequency_hz: f64,
    pub prefault_voltage_pu: f64,
    pub options: crate::CalculationOptions,
    pub buses: Vec<BusFaultResult>,
    pub summary: SummaryMetrics,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SummaryMetrics {
    pub bus_count: usize,
    pub branch_count: usize,
    pub source_count: usize,
    pub max_3ph_ka: f64,
    pub max_3ph_bus: String,
    pub max_slg_ka: f64,
    pub max_slg_bus: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusFaultResult {
    pub bus_id: String,
    pub bus: String,
    pub kv_ll: f64,
    pub ibase_ka: f64,
    pub z1_pu: Option<Impedance>,
    pub z2_pu: Option<Impedance>,
    pub z0_pu: Option<Impedance>,
    pub faults: Vec<FaultResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultResult {
    pub kind: String,
    pub available_sym_ka: f64,
    pub available_pu: f64,
    pub ground_return_ka: Option<f64>,
    pub equivalent_z_pu: Option<Impedance>,
    pub x_over_r: Option<f64>,
    pub peak_asym_ka: Option<f64>,
    pub asym_rms_ka: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct SeqBranch {
    from: usize,
    to: usize,
    z: Complex,
}

#[derive(Debug, Clone)]
struct SeqNetwork {
    branches: Vec<SeqBranch>,
    shunts: Vec<Complex>,
    adjacency: Vec<Vec<usize>>,
}

pub fn calculate_all_buses(net: &Network) -> Result<Report> {
    net.validate()?;
    let seq1 = build_seq_network(net, 1);
    let seq2 = build_seq_network(net, 2);
    let seq0 = build_seq_network(net, 0);

    let mut report = Report {
        version: VERSION.to_string(),
        project: net.project.clone(),
        base_mva: net.base_mva,
        frequency_hz: net.frequency_hz,
        prefault_voltage_pu: net.prefault_voltage_pu,
        options: net.options.clone(),
        buses: Vec::new(),
        summary: SummaryMetrics {
            bus_count: net.buses.len(),
            branch_count: net.branches.len(),
            source_count: net.sources.len(),
            ..SummaryMetrics::default()
        },
        warnings: Vec::new(),
    };

    for (bus_index, bus) in net.buses.iter().enumerate() {
        let z1 = driving_point_impedance(&seq1, bus_index).map_err(|e| {
            FaultCalcError::new(format!(
                "positive sequence solve failed at {}: {}",
                bus.display_name(),
                e
            ))
        })?;
        let z2 = driving_point_impedance(&seq2, bus_index).map_err(|e| {
            FaultCalcError::new(format!(
                "negative sequence solve failed at {}: {}",
                bus.display_name(),
                e
            ))
        })?;
        let z0 = driving_point_impedance(&seq0, bus_index).map_err(|e| {
            FaultCalcError::new(format!(
                "zero sequence solve failed at {}: {}",
                bus.display_name(),
                e
            ))
        })?;

        if z1.is_none() {
            report.warnings.push(format!(
                "bus {} has no positive-sequence source path",
                bus.display_name()
            ));
        }
        if z0.is_none() {
            report.warnings.push(format!("bus {} has no zero-sequence return path; SLG/DLG ground current is zero in this model", bus.display_name()));
        }

        let ibase_ka = net.i_base_ka(bus_index);
        let zbase = net.z_base_ohm(bus_index);
        let zf = Complex::new(
            net.options.fault_r_ohm / zbase,
            net.options.fault_x_ohm / zbase,
        );
        let faults = fault_results_for_bus(net, z1, z2, z0, zf, ibase_ka);

        let result = BusFaultResult {
            bus_id: bus.id.clone(),
            bus: bus.display_name(),
            kv_ll: bus.kv_ll,
            ibase_ka,
            z1_pu: z1.map(Impedance::from_complex),
            z2_pu: z2.map(Impedance::from_complex),
            z0_pu: z0.map(Impedance::from_complex),
            faults,
        };

        for fault in &result.faults {
            match fault.kind.as_str() {
                "3PH" if fault.available_sym_ka > report.summary.max_3ph_ka => {
                    report.summary.max_3ph_ka = fault.available_sym_ka;
                    report.summary.max_3ph_bus = result.bus.clone();
                }
                "SLG" if fault.available_sym_ka > report.summary.max_slg_ka => {
                    report.summary.max_slg_ka = fault.available_sym_ka;
                    report.summary.max_slg_bus = result.bus.clone();
                }
                _ => {}
            }
        }
        report.buses.push(result);
    }

    Ok(report)
}

pub fn report_json_pretty(report: &Report) -> Result<String> {
    Ok(serde_json::to_string_pretty(report)?)
}

pub fn report_csv(report: &Report) -> String {
    let mut out = String::new();
    out.push_str("bus_id,bus,kv_ll,fault,sym_ka,peak_asym_ka,asym_rms_ka,x_over_r,ground_return_ka,z_eq_r_pu,z_eq_x_pu,notes\n");
    for bus in &report.buses {
        for fault in &bus.faults {
            let (zr, zx) = match fault.equivalent_z_pu {
                Some(z) => (fmt(z.r, 6), fmt(z.x, 6)),
                None => (String::new(), String::new()),
            };
            let row = vec![
                csv_escape(&bus.bus_id),
                csv_escape(&bus.bus),
                fmt(bus.kv_ll, 6),
                csv_escape(&fault.kind),
                fmt(fault.available_sym_ka, 6),
                fmt_opt(fault.peak_asym_ka, 6),
                fmt_opt(fault.asym_rms_ka, 6),
                fmt_opt(fault.x_over_r, 6),
                fmt_opt(fault.ground_return_ka, 6),
                zr,
                zx,
                csv_escape(&fault.notes.join("; ")),
            ];
            out.push_str(&row.join(","));
            out.push('\n');
        }
    }
    out
}

fn fmt(v: f64, digits: usize) -> String {
    if !v.is_finite() {
        String::new()
    } else {
        format!("{:.*}", digits, v)
    }
}

fn fmt_opt(v: Option<f64>, digits: usize) -> String {
    v.map(|x| fmt(x, digits)).unwrap_or_default()
}

fn csv_escape(v: &str) -> String {
    if v.contains(',') || v.contains('"') || v.contains('\n') || v.contains('\r') {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v.to_string()
    }
}

fn build_seq_network(net: &Network, seq: u8) -> SeqNetwork {
    let n = net.buses.len();
    let mut out = SeqNetwork {
        branches: Vec::new(),
        shunts: vec![Complex::ZERO; n],
        adjacency: vec![Vec::new(); n],
    };
    for branch in &net.branches {
        if !branch.enabled {
            continue;
        }
        let Some(from) = net.bus_index(&branch.from) else {
            continue;
        };
        let Some(to) = net.bus_index(&branch.to) else {
            continue;
        };
        let (ok, z) = match seq {
            1 => (true, branch.z1_pu.to_complex()),
            2 => (true, branch.z2_pu.to_complex()),
            0 => (branch.has_z0, branch.z0_pu.to_complex()),
            _ => (false, Complex::ZERO),
        };
        if ok && z.abs() > 1e-14 && z.finite() {
            let idx = out.branches.len();
            out.branches.push(SeqBranch { from, to, z });
            out.adjacency[from].push(idx);
            out.adjacency[to].push(idx);
        }
    }
    for source in &net.sources {
        if !source.enabled {
            continue;
        }
        let Some(bus) = net.bus_index(&source.bus) else {
            continue;
        };
        let (ok, z) = match seq {
            1 => (true, source.z1_pu.to_complex()),
            2 => (true, source.z2_pu.to_complex()),
            0 => (source.has_z0, source.z0_pu.to_complex()),
            _ => (false, Complex::ZERO),
        };
        if ok && z.abs() > 1e-14 && z.finite() {
            out.shunts[bus] += Complex::ONE / z;
        }
    }
    out
}

fn driving_point_impedance(seq: &SeqNetwork, bus_index: usize) -> Result<Option<Complex>> {
    let component = connected_component(seq, bus_index);
    let has_ground_path = component.iter().any(|&i| seq.shunts[i].abs() > 1e-14);
    if !has_ground_path {
        return Ok(None);
    }

    let mut pos = vec![usize::MAX; seq.shunts.len()];
    for (local, &global) in component.iter().enumerate() {
        pos[global] = local;
    }
    let n = component.len();
    let mut y = vec![vec![Complex::ZERO; n]; n];
    for (local, &global) in component.iter().enumerate() {
        y[local][local] += seq.shunts[global];
    }
    for branch in &seq.branches {
        let i = pos[branch.from];
        let j = pos[branch.to];
        if i != usize::MAX && j != usize::MAX {
            let adm = Complex::ONE / branch.z;
            y[i][i] += adm;
            y[j][j] += adm;
            y[i][j] -= adm;
            y[j][i] -= adm;
        }
    }
    let mut rhs = vec![Complex::ZERO; n];
    rhs[pos[bus_index]] = Complex::ONE;
    let v = solve_linear(y, rhs)?;
    Ok(Some(v[pos[bus_index]]))
}

fn connected_component(seq: &SeqNetwork, start: usize) -> Vec<usize> {
    let mut seen = vec![false; seq.shunts.len()];
    let mut q = std::collections::VecDeque::new();
    seen[start] = true;
    q.push_back(start);
    while let Some(node) = q.pop_front() {
        for &branch_index in &seq.adjacency[node] {
            let branch = seq.branches[branch_index];
            let next = if branch.to == node {
                branch.from
            } else {
                branch.to
            };
            if !seen[next] {
                seen[next] = true;
                q.push_back(next);
            }
        }
    }
    let mut out: Vec<usize> = seen
        .iter()
        .enumerate()
        .filter_map(|(i, v)| if *v { Some(i) } else { None })
        .collect();
    out.sort_unstable();
    out
}

fn solve_linear(mut a: Vec<Vec<Complex>>, mut b: Vec<Complex>) -> Result<Vec<Complex>> {
    let n = b.len();
    for i in 0..n {
        let (pivot, pivot_abs) = (i..n)
            .map(|r| (r, a[r][i].abs()))
            .max_by(|x, y| x.1.partial_cmp(&y.1).unwrap_or(Ordering::Equal))
            .unwrap_or((i, 0.0));
        if pivot_abs < 1e-18 {
            return Err(FaultCalcError::new("singular sequence admittance matrix"));
        }
        if pivot != i {
            a.swap(i, pivot);
            b.swap(i, pivot);
        }
        let piv = a[i][i];
        for c in i..n {
            a[i][c] /= piv;
        }
        b[i] /= piv;
        for r in 0..n {
            if r == i {
                continue;
            }
            let factor = a[r][i];
            if factor.abs() < 1e-18 {
                continue;
            }
            for c in i..n {
                a[r][c] = a[r][c] - factor * a[i][c];
            }
            b[r] = b[r] - factor * b[i];
        }
    }
    Ok(b)
}

fn fault_results_for_bus(
    net: &Network,
    z1: Option<Complex>,
    z2: Option<Complex>,
    z0: Option<Complex>,
    zf: Complex,
    ibase_ka: f64,
) -> Vec<FaultResult> {
    let v = Complex::new(net.prefault_voltage_pu, 0.0);
    let mut out = Vec::new();

    if let Some(z1) = z1 {
        let zeq = z1 + zf;
        let i = v / zeq;
        out.push(make_fault_result(
            "3PH",
            i.abs(),
            None,
            zeq,
            ibase_ka,
            net.frequency_hz,
            net.options.duty_cycles,
            Vec::new(),
        ));
    } else {
        out.push(zero_fault("3PH", "no positive-sequence source path"));
    }

    if let (Some(z1), Some(z2), Some(z0)) = (z1, z2, z0) {
        let zeq = z1 + z2 + z0 + zf.scale(3.0);
        let i1 = v / zeq;
        let ia = i1.scale(3.0);
        let ground_ka = ia.abs() * ibase_ka;
        out.push(make_fault_result(
            "SLG",
            ia.abs(),
            Some(ground_ka),
            zeq,
            ibase_ka,
            net.frequency_hz,
            net.options.duty_cycles,
            Vec::new(),
        ));
    } else {
        out.push(zero_fault(
            "SLG",
            "missing positive, negative, or zero sequence return path",
        ));
    }

    if let (Some(z1), Some(z2)) = (z1, z2) {
        let zeq = z1 + z2 + zf;
        let i1 = v / zeq;
        let line_current_pu = 3.0_f64.sqrt() * i1.abs();
        out.push(make_fault_result(
            "LL",
            line_current_pu,
            None,
            zeq,
            ibase_ka,
            net.frequency_hz,
            net.options.duty_cycles,
            Vec::new(),
        ));
    } else {
        out.push(zero_fault(
            "LL",
            "missing positive or negative sequence source path",
        ));
    }

    if let (Some(z1), Some(z2), Some(z0)) = (z1, z2, z0) {
        let z0g = z0 + zf.scale(3.0);
        let denom = z2 + z0g;
        if denom.abs() > 1e-18 {
            let zpar = (z2 * z0g) / denom;
            let zeq = z1 + zpar;
            let i1 = v / zeq;
            let i2 = -(i1 * (z0g / denom));
            let i0 = -(i1 * (z2 / denom));
            let a = Complex::from_polar(1.0, 2.0 * PI / 3.0);
            let a2 = a * a;
            let ib = i0 + a2 * i1 + a * i2;
            let ic = i0 + a * i1 + a2 * i2;
            let phase_current_pu = ib.abs().max(ic.abs());
            let ground_ka = i0.scale(3.0).abs() * ibase_ka;
            out.push(make_fault_result(
                "DLG",
                phase_current_pu,
                Some(ground_ka),
                zeq,
                ibase_ka,
                net.frequency_hz,
                net.options.duty_cycles,
                Vec::new(),
            ));
        } else {
            out.push(zero_fault(
                "DLG",
                "zero denominator in double-line-ground sequence combination",
            ));
        }
    } else {
        out.push(zero_fault(
            "DLG",
            "missing positive, negative, or zero sequence return path",
        ));
    }

    out
}

fn zero_fault(kind: &str, note: &str) -> FaultResult {
    FaultResult {
        kind: kind.to_string(),
        available_sym_ka: 0.0,
        available_pu: 0.0,
        ground_return_ka: None,
        equivalent_z_pu: None,
        x_over_r: None,
        peak_asym_ka: None,
        asym_rms_ka: None,
        notes: vec![note.to_string()],
    }
}

fn make_fault_result(
    kind: &str,
    available_pu: f64,
    ground_return_ka: Option<f64>,
    zeq: Complex,
    ibase_ka: f64,
    frequency_hz: f64,
    duty_cycles: f64,
    notes: Vec<String>,
) -> FaultResult {
    let available_sym_ka = available_pu * ibase_ka;
    let xr = x_over_r_abs(zeq).map(|v| v.min(1.0e9));
    let peak_asym_ka =
        xr.map(|v| peak_asym_current_ka(available_sym_ka, v, frequency_hz, 0.5 / frequency_hz));
    let asym_rms_ka = xr.map(|v| {
        asym_rms_current_ka(
            available_sym_ka,
            v,
            frequency_hz,
            duty_cycles / frequency_hz,
        )
    });
    FaultResult {
        kind: kind.to_string(),
        available_sym_ka: clean_f64(available_sym_ka),
        available_pu: clean_f64(available_pu),
        ground_return_ka: ground_return_ka.map(clean_f64),
        equivalent_z_pu: Some(Impedance::from_complex(zeq)),
        x_over_r: xr.map(clean_f64),
        peak_asym_ka: peak_asym_ka.map(clean_f64),
        asym_rms_ka: asym_rms_ka.map(clean_f64),
        notes,
    }
}

fn clean_f64(v: f64) -> f64 {
    if v.is_finite() {
        v
    } else {
        0.0
    }
}

fn peak_asym_current_ka(i_sym_ka: f64, xr: f64, frequency_hz: f64, t_seconds: f64) -> f64 {
    let tau = dc_time_constant_seconds(xr, frequency_hz);
    if tau == 0.0 {
        2.0_f64.sqrt() * i_sym_ka
    } else {
        2.0_f64.sqrt() * i_sym_ka * (1.0 + (-t_seconds / tau).exp())
    }
}

fn asym_rms_current_ka(i_sym_ka: f64, xr: f64, frequency_hz: f64, t_seconds: f64) -> f64 {
    let tau = dc_time_constant_seconds(xr, frequency_hz);
    if tau == 0.0 {
        i_sym_ka
    } else {
        i_sym_ka * (1.0 + 2.0 * (-2.0 * t_seconds / tau).exp()).sqrt()
    }
}

fn dc_time_constant_seconds(xr: f64, frequency_hz: f64) -> f64 {
    if xr <= 0.0 || frequency_hz <= 0.0 {
        0.0
    } else {
        xr / (2.0 * PI * frequency_hz)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{sample_network, Branch, Impedance, Network, Source};

    fn assert_close(actual: f64, expected: f64, tol: f64) {
        assert!(
            (actual - expected).abs() <= tol,
            "actual {actual} expected {expected} tol {tol}"
        );
    }

    fn assert_complex_close(actual: Complex, expected: Complex, tol: f64) {
        assert_close(actual.re, expected.re, tol);
        assert_close(actual.im, expected.im, tol);
    }

    fn bus<'a>(report: &'a Report, id: &str) -> &'a BusFaultResult {
        report.buses.iter().find(|b| b.bus_id == id).unwrap()
    }

    fn fault<'a>(bus: &'a BusFaultResult, kind: &str) -> &'a FaultResult {
        bus.faults.iter().find(|f| f.kind == kind).unwrap()
    }

    fn one_bus_network(z: Impedance, has_z0: bool) -> Network {
        let mut net = Network::new(100.0);
        net.add_bus("bus", "Hand bus", 10.0, 0.0, 0.0).unwrap();
        net.add_source(Source {
            id: "src".to_string(),
            kind: "utility".to_string(),
            name: "Utility".to_string(),
            bus: "bus".to_string(),
            state: "in_service".to_string(),
            z1_pu: z,
            z2_pu: z,
            z0_pu: z,
            has_z0,
            enabled: true,
            rating: String::new(),
            notes: String::new(),
        })
        .unwrap();
        net
    }

    #[test]
    fn sample_solves_all_buses() {
        let net = sample_network();
        let report = calculate_all_buses(&net).unwrap();
        assert_eq!(report.buses.len(), net.buses.len());
        assert!(report.summary.max_3ph_ka > 0.0);
        assert!(report.buses.iter().all(|b| b.faults.len() == 4));
    }

    #[test]
    fn csv_has_rows() {
        let net = sample_network();
        let report = calculate_all_buses(&net).unwrap();
        let csv = report_csv(&report);
        assert!(csv.contains("3PH"));
        assert!(csv.lines().count() > 4);
    }

    #[test]
    fn matrix_solver_solves_hand_checkable_system() {
        let a = vec![
            vec![Complex::new(2.0, 0.0), Complex::new(-1.0, 0.0)],
            vec![Complex::new(-1.0, 0.0), Complex::new(2.0, 0.0)],
        ];
        let b = vec![Complex::new(1.0, 0.0), Complex::ZERO];

        let x = solve_linear(a, b).unwrap();

        assert_complex_close(x[0], Complex::new(2.0 / 3.0, 0.0), 1e-12);
        assert_complex_close(x[1], Complex::new(1.0 / 3.0, 0.0), 1e-12);
    }

    #[test]
    fn sequence_driving_point_impedance_includes_source_and_branch() {
        let mut net = Network::new(100.0);
        net.add_bus("src_bus", "Source bus", 10.0, 0.0, 0.0)
            .unwrap();
        net.add_bus("load_bus", "Load bus", 10.0, 100.0, 0.0)
            .unwrap();
        net.add_source(Source {
            id: "src".to_string(),
            kind: "utility".to_string(),
            name: "Utility".to_string(),
            bus: "src_bus".to_string(),
            state: "in_service".to_string(),
            z1_pu: Impedance::new(0.0, 0.1),
            z2_pu: Impedance::new(0.0, 0.1),
            z0_pu: Impedance::new(0.0, 0.1),
            has_z0: true,
            enabled: true,
            rating: String::new(),
            notes: String::new(),
        })
        .unwrap();
        net.add_branch(Branch {
            id: "f1".to_string(),
            kind: "feeder".to_string(),
            name: "Feeder".to_string(),
            from: "src_bus".to_string(),
            to: "load_bus".to_string(),
            state: "normally_closed".to_string(),
            conductors: Default::default(),
            primary_connection: String::new(),
            secondary_connection: String::new(),
            vector_shift_deg: 0.0,
            z1_pu: Impedance::new(0.0, 0.2),
            z2_pu: Impedance::new(0.0, 0.2),
            z0_pu: Impedance::new(0.0, 0.2),
            has_z0: true,
            enabled: true,
            rating_a: 0.0,
            rating_kva: 0.0,
            impedance_percent: 0.0,
            xr_ratio: 0.0,
            length_m: 0.0,
            notes: String::new(),
        })
        .unwrap();

        let seq1 = build_seq_network(&net, 1);
        let z = driving_point_impedance(&seq1, net.bus_index("load_bus").unwrap())
            .unwrap()
            .unwrap();

        assert_complex_close(z, Complex::new(0.0, 0.3), 1e-12);
    }

    #[test]
    fn simple_one_source_one_branch_thevenin_matches_hand_check() {
        let mut net = Network::new(100.0);
        net.add_bus("source", "Source", 10.0, 0.0, 0.0).unwrap();
        net.add_bus("load", "Load", 10.0, 100.0, 0.0).unwrap();
        net.add_source(Source {
            id: "src".to_string(),
            kind: "utility".to_string(),
            name: "Utility".to_string(),
            bus: "source".to_string(),
            state: "in_service".to_string(),
            z1_pu: Impedance::new(0.0, 0.1),
            z2_pu: Impedance::new(0.0, 0.1),
            z0_pu: Impedance::new(0.0, 0.1),
            has_z0: true,
            enabled: true,
            rating: String::new(),
            notes: String::new(),
        })
        .unwrap();
        net.add_branch(Branch {
            id: "branch".to_string(),
            kind: "feeder".to_string(),
            name: "Feeder".to_string(),
            from: "source".to_string(),
            to: "load".to_string(),
            state: "normally_closed".to_string(),
            conductors: Default::default(),
            primary_connection: String::new(),
            secondary_connection: String::new(),
            vector_shift_deg: 0.0,
            z1_pu: Impedance::new(0.0, 0.2),
            z2_pu: Impedance::new(0.0, 0.2),
            z0_pu: Impedance::new(0.0, 0.2),
            has_z0: true,
            enabled: true,
            rating_a: 0.0,
            rating_kva: 0.0,
            impedance_percent: 0.0,
            xr_ratio: 0.0,
            length_m: 0.0,
            notes: String::new(),
        })
        .unwrap();

        let report = calculate_all_buses(&net).unwrap();
        let load = bus(&report, "load");
        let z1 = load.z1_pu.unwrap();
        let three_phase = fault(load, "3PH");

        assert_close(z1.r, 0.0, 1e-12);
        assert_close(z1.x, 0.3, 1e-12);
        assert_close(three_phase.available_pu, 1.0 / 0.3, 1e-12);
        assert_close(three_phase.available_sym_ka, 19.245008972987527, 1e-12);
    }

    #[test]
    fn fault_formulas_match_balanced_hand_calculation() {
        let net = one_bus_network(Impedance::new(0.0, 0.2), true);
        let report = calculate_all_buses(&net).unwrap();
        let bus = bus(&report, "bus");

        assert_close(fault(bus, "3PH").available_pu, 5.0, 1e-12);
        assert_close(fault(bus, "3PH").available_sym_ka, 28.86751345948129, 1e-12);
        assert_close(fault(bus, "SLG").available_pu, 5.0, 1e-12);
        assert_close(
            fault(bus, "SLG").ground_return_ka.unwrap(),
            28.86751345948129,
            1e-12,
        );
        assert_close(fault(bus, "LL").available_pu, 4.330127018922193, 1e-12);
        assert_close(fault(bus, "LL").available_sym_ka, 25.0, 1e-12);
        assert_close(fault(bus, "DLG").available_pu, 5.0, 1e-12);
        assert_close(
            fault(bus, "DLG").ground_return_ka.unwrap(),
            28.86751345948129,
            1e-12,
        );
    }

    #[test]
    fn sample_case_fault_snapshot_covers_all_faults_at_all_buses() {
        let net = sample_network();
        let report = calculate_all_buses(&net).unwrap();
        let expected = [(
            "util_bus",
            [
                ("3PH", 962202.3397350826),
                ("SLG", 962202.3397350826),
                ("LL", 833291.6697914065),
                ("DLG", 962202.3397350826),
            ],
        )];

        for (bus_id, faults) in expected {
            let bus = bus(&report, bus_id);
            assert_eq!(bus.faults.len(), 4);
            for (kind, expected_ka) in faults {
                assert_close(fault(bus, kind).available_sym_ka, expected_ka, 1e-9);
            }
        }
    }

    #[test]
    fn disabled_sources_do_not_contribute_to_driving_point_impedance() {
        let mut net = one_bus_network(Impedance::new(0.0, 0.1), true);
        net.add_source(Source {
            id: "disabled".to_string(),
            kind: "utility".to_string(),
            name: "Disabled source".to_string(),
            bus: "bus".to_string(),
            state: "out_of_service".to_string(),
            z1_pu: Impedance::new(0.0, 0.1),
            z2_pu: Impedance::new(0.0, 0.1),
            z0_pu: Impedance::new(0.0, 0.1),
            has_z0: true,
            enabled: false,
            rating: String::new(),
            notes: String::new(),
        })
        .unwrap();

        let report = calculate_all_buses(&net).unwrap();
        let three_phase = fault(bus(&report, "bus"), "3PH");

        assert_close(three_phase.available_pu, 10.0, 1e-12);
    }

    #[test]
    fn disabled_branch_leaves_downstream_bus_unsourced() {
        let mut net = Network::new(100.0);
        net.add_bus("source", "Source", 10.0, 0.0, 0.0).unwrap();
        net.add_bus("load", "Load", 10.0, 100.0, 0.0).unwrap();
        net.add_source(Source {
            id: "src".to_string(),
            kind: "utility".to_string(),
            name: "Utility".to_string(),
            bus: "source".to_string(),
            state: "in_service".to_string(),
            z1_pu: Impedance::new(0.0, 0.1),
            z2_pu: Impedance::new(0.0, 0.1),
            z0_pu: Impedance::new(0.0, 0.1),
            has_z0: true,
            enabled: true,
            rating: String::new(),
            notes: String::new(),
        })
        .unwrap();
        net.add_branch(Branch {
            id: "open".to_string(),
            kind: "feeder".to_string(),
            name: "Open feeder".to_string(),
            from: "source".to_string(),
            to: "load".to_string(),
            state: "normally_open".to_string(),
            conductors: Default::default(),
            primary_connection: String::new(),
            secondary_connection: String::new(),
            vector_shift_deg: 0.0,
            z1_pu: Impedance::new(0.0, 0.2),
            z2_pu: Impedance::new(0.0, 0.2),
            z0_pu: Impedance::new(0.0, 0.2),
            has_z0: true,
            enabled: false,
            rating_a: 0.0,
            rating_kva: 0.0,
            impedance_percent: 0.0,
            xr_ratio: 0.0,
            length_m: 0.0,
            notes: String::new(),
        })
        .unwrap();

        let report = calculate_all_buses(&net).unwrap();
        let load = bus(&report, "load");

        assert!(load.z1_pu.is_none());
        assert_close(fault(load, "3PH").available_sym_ka, 0.0, 1e-12);
        assert!(report
            .warnings
            .iter()
            .any(|w| w.contains("Load") && w.contains("positive-sequence")));
    }

    #[test]
    fn zero_sequence_open_generates_ground_fault_warnings() {
        let net = one_bus_network(Impedance::new(0.0, 0.2), false);
        let report = calculate_all_buses(&net).unwrap();
        let bus = bus(&report, "bus");

        assert!(report
            .warnings
            .iter()
            .any(|w| w.contains("zero-sequence return path")));
        assert_close(fault(bus, "SLG").available_sym_ka, 0.0, 1e-12);
        assert_close(fault(bus, "DLG").available_sym_ka, 0.0, 1e-12);
        assert!(fault(bus, "SLG")
            .notes
            .iter()
            .any(|n| n.contains("zero sequence return path")));
        assert!(fault(bus, "DLG")
            .notes
            .iter()
            .any(|n| n.contains("zero sequence return path")));
    }

    #[test]
    fn nonzero_fault_impedance_is_converted_from_ohms_at_faulted_bus() {
        let mut net = one_bus_network(Impedance::new(0.0, 0.2), true);
        net.options.fault_x_ohm = 0.1;

        let report = calculate_all_buses(&net).unwrap();
        let three_phase = fault(bus(&report, "bus"), "3PH");
        let zeq = three_phase.equivalent_z_pu.unwrap();

        assert_close(zeq.x, 0.3, 1e-12);
        assert_close(three_phase.available_pu, 1.0 / 0.3, 1e-12);
        assert_close(three_phase.available_sym_ka, 19.245008972987527, 1e-12);
    }
}
