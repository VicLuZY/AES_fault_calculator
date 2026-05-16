use faultcalc_core::{calculate_all_buses, report_json_pretty, sample_network, Result};

fn main() -> Result<()> {
    let net = sample_network();
    let report = calculate_all_buses(&net)?;
    println!("{}", report_json_pretty(&report)?);
    Ok(())
}
