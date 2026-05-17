#![allow(static_mut_refs)]

use faultcalc_core::{
    calculate_all_buses, case_domain_json, normalise_case, report_json_pretty, sample_json,
    Network, VERSION,
};
use serde_json::json;

static mut LAST_OUTPUT: Vec<u8> = Vec::new();

#[no_mangle]
pub extern "C" fn faultcalc_alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn faultcalc_free(ptr: *mut u8, len: usize) {
    if ptr.is_null() {
        return;
    }
    let _ = Vec::from_raw_parts(ptr, 0, len);
}

#[no_mangle]
pub extern "C" fn faultcalc_output_ptr() -> *const u8 {
    unsafe { LAST_OUTPUT.as_ptr() }
}

#[no_mangle]
pub extern "C" fn faultcalc_output_len() -> usize {
    unsafe { LAST_OUTPUT.len() }
}

#[no_mangle]
pub unsafe extern "C" fn faultcalc_calculate(input_ptr: *const u8, input_len: usize) -> i32 {
    let bytes = std::slice::from_raw_parts(input_ptr, input_len);
    match std::str::from_utf8(bytes) {
        Ok(text) => match calculate_response(text) {
            Ok(output) => {
                set_output(output);
                1
            }
            Err(err) => {
                set_output(error_response(err));
                1
            }
        },
        Err(err) => {
            set_output(error_response(err.to_string()));
            1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn faultcalc_model(input_ptr: *const u8, input_len: usize) -> i32 {
    let bytes = std::slice::from_raw_parts(input_ptr, input_len);
    match std::str::from_utf8(bytes) {
        Ok(text) => match model_response(text) {
            Ok(output) => {
                set_output(output);
                1
            }
            Err(err) => {
                set_output(error_response(err));
                1
            }
        },
        Err(err) => {
            set_output(error_response(err.to_string()));
            1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn faultcalc_sample() -> i32 {
    match sample_json() {
        Ok(output) => {
            set_output(output);
            1
        }
        Err(err) => {
            set_output(error_response(err.to_string()));
            1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn faultcalc_version() -> i32 {
    set_output(VERSION.to_string());
    1
}

fn calculate_response(text: &str) -> Result<String, String> {
    let mut net = Network::from_json(text).map_err(|e| e.to_string())?;
    normalise_case(&mut net);
    let report = calculate_all_buses(&net).map_err(|e| e.to_string())?;
    let report_value = serde_json::to_value(&report).map_err(|e| e.to_string())?;
    Ok(json!({
        "ok": true,
        "version": VERSION,
        "report": report_value
    })
    .to_string())
}

fn model_response(text: &str) -> Result<String, String> {
    let domain = case_domain_json(text).map_err(|e| e.to_string())?;
    let domain_value: serde_json::Value =
        serde_json::from_str(&domain).map_err(|e| e.to_string())?;
    let network_value = domain_value["network"].clone();
    Ok(json!({
        "ok": true,
        "version": VERSION,
        "domain": domain_value,
        "network": network_value
    })
    .to_string())
}

fn error_response(message: String) -> String {
    json!({
        "ok": false,
        "version": VERSION,
        "error": message
    })
    .to_string()
}

fn set_output(output: String) {
    unsafe {
        LAST_OUTPUT = output.into_bytes();
    }
}

#[allow(dead_code)]
fn _round_trip_report_json_for_linker() -> Option<String> {
    let sample = sample_json().ok()?;
    let net = Network::from_json(&sample).ok()?;
    let report = calculate_all_buses(&net).ok()?;
    report_json_pretty(&report).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    unsafe fn output_string() -> String {
        let ptr = faultcalc_output_ptr();
        let len = faultcalc_output_len();
        String::from_utf8(std::slice::from_raw_parts(ptr, len).to_vec()).unwrap()
    }

    unsafe fn calculate(input: &str) -> Value {
        let bytes = input.as_bytes();
        let ptr = faultcalc_alloc(bytes.len());
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
        let ok = faultcalc_calculate(ptr, bytes.len());
        faultcalc_free(ptr, bytes.len());
        assert_eq!(ok, 1);
        serde_json::from_str(&output_string()).unwrap()
    }

    unsafe fn model(input: &str) -> Value {
        let bytes = input.as_bytes();
        let ptr = faultcalc_alloc(bytes.len());
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
        let ok = faultcalc_model(ptr, bytes.len());
        faultcalc_free(ptr, bytes.len());
        assert_eq!(ok, 1);
        serde_json::from_str(&output_string()).unwrap()
    }

    #[test]
    fn direct_abi_exports_version_sample_and_calculate() {
        unsafe {
            assert_eq!(faultcalc_version(), 1);
            assert_eq!(output_string(), VERSION);

            assert_eq!(faultcalc_sample(), 1);
            let sample = output_string();
            let sample_value: Value = serde_json::from_str(&sample).unwrap();
            assert_eq!(sample_value["project"]["name"], "AES simple SLD study");

            let response = calculate(&sample);
            assert_eq!(response["ok"], true);
            assert_eq!(response["report"]["summary"]["bus_count"], 1);
            assert_eq!(
                response["report"]["buses"][0]["faults"]
                    .as_array()
                    .unwrap()
                    .len(),
                4
            );

            let model_response = model(&sample);
            assert_eq!(model_response["ok"], true);
            assert_eq!(model_response["domain"]["voltage_options"][0]["key"], "120");
            let utility_kv = model_response["network"]["buses"][0]["kv_ll"]
                .as_f64()
                .unwrap();
            assert!((utility_kv - faultcalc_core::DEFAULT_UTILITY_KV_LL).abs() < 1e-12);
            assert_eq!(
                model_response["domain"]["sources"][0]["voltage_label"],
                "12.5 kV"
            );
        }
    }
}
