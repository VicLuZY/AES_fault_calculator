use serde_json::Value;
use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn unique_temp_dir() -> std::path::PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("faultcalc-cli-test-{}-{nanos}", std::process::id()))
}

unsafe fn wasm_calculate(input: &str) -> Value {
    let bytes = input.as_bytes();
    let ptr = faultcalc_wasm::faultcalc_alloc(bytes.len());
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
    let ok = faultcalc_wasm::faultcalc_calculate(ptr, bytes.len());
    faultcalc_wasm::faultcalc_free(ptr, bytes.len());
    assert_eq!(ok, 1);

    let out_ptr = faultcalc_wasm::faultcalc_output_ptr();
    let out_len = faultcalc_wasm::faultcalc_output_len();
    let out = std::slice::from_raw_parts(out_ptr, out_len);
    serde_json::from_slice(out).unwrap()
}

#[test]
fn cli_json_csv_and_wasm_report_match_for_sample_case() {
    let root = workspace_root();
    let sample = root.join("cases/sample.json");
    let case_text = fs::read_to_string(&sample).unwrap();
    let temp = unique_temp_dir();
    fs::create_dir_all(&temp).unwrap();
    let json_out = temp.join("report.json");
    let csv_out = temp.join("report.csv");

    let status = Command::new(env!("CARGO_BIN_EXE_faultcalc"))
        .arg("calc")
        .arg(&sample)
        .arg("--json")
        .arg(&json_out)
        .arg("--csv")
        .arg(&csv_out)
        .status()
        .unwrap();
    assert!(status.success());

    let cli_report: Value = serde_json::from_str(&fs::read_to_string(&json_out).unwrap()).unwrap();
    let wasm_response = unsafe { wasm_calculate(&case_text) };
    assert_eq!(wasm_response["ok"], true);
    assert_eq!(cli_report, wasm_response["report"]);

    let csv = fs::read_to_string(&csv_out).unwrap();
    assert!(csv.starts_with("bus_id,bus,kv_ll,fault"));
    assert!(csv.contains("3PH"));
    assert!(csv.contains("DLG"));

    fs::remove_dir_all(temp).unwrap();
}
