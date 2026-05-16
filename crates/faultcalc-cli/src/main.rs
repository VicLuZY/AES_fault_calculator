use faultcalc_core::{calculate_all_buses, report_csv, report_json_pretty, sample_json, Network, Result, VERSION};
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;

fn main() {
    if let Err(err) = run(std::env::args().skip(1).collect()) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<()> {
    if args.is_empty() || args[0] == "-h" || args[0] == "--help" {
        print_help();
        return Ok(());
    }
    match args[0].as_str() {
        "sample" => {
            println!("{}", sample_json()?);
            Ok(())
        }
        "calc" => run_calc(&args[1..]),
        "serve" => run_serve(&args[1..]),
        "embed-wasm" => run_embed_wasm(&args[1..]),
        "version" => {
            println!("{VERSION}");
            Ok(())
        }
        other => Err(faultcalc_core::FaultCalcError::new(format!("unknown command {other:?}"))),
    }
}

fn run_calc(args: &[String]) -> Result<()> {
    if args.is_empty() {
        return Err(faultcalc_core::FaultCalcError::new("calc requires a case JSON file"));
    }
    let input = &args[0];
    let mut json_out: Option<String> = None;
    let mut csv_out: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => {
                i += 1;
                if i >= args.len() { return Err(faultcalc_core::FaultCalcError::new("--json requires a path")); }
                json_out = Some(args[i].clone());
            }
            "--csv" => {
                i += 1;
                if i >= args.len() { return Err(faultcalc_core::FaultCalcError::new("--csv requires a path")); }
                csv_out = Some(args[i].clone());
            }
            other => return Err(faultcalc_core::FaultCalcError::new(format!("unknown calc option {other:?}"))),
        }
        i += 1;
    }

    let text = fs::read_to_string(input)?;
    let mut net = Network::from_json(&text)?;
    net.normalise_defaults();
    let report = calculate_all_buses(&net)?;
    let json = report_json_pretty(&report)?;
    if let Some(path) = json_out {
        write_file(path, json.as_bytes())?;
    } else {
        println!("{json}");
    }
    if let Some(path) = csv_out {
        write_file(path, report_csv(&report).as_bytes())?;
    }
    Ok(())
}

fn run_serve(args: &[String]) -> Result<()> {
    let dir = args.get(0).cloned().unwrap_or_else(|| "web".to_string());
    let port = args.get(1).and_then(|p| p.parse::<u16>().ok()).unwrap_or(8080);
    if !Path::new(&dir).exists() {
        return Err(faultcalc_core::FaultCalcError::new(format!("{dir} does not exist")));
    }
    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr)?;
    println!("serving {dir} at http://{addr}");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(err) = handle_http(stream, Path::new(&dir)) {
                    eprintln!("request error: {err}");
                }
            }
            Err(err) => eprintln!("connection error: {err}"),
        }
    }
    Ok(())
}

fn run_embed_wasm(args: &[String]) -> Result<()> {
    if args.len() != 3 {
        return Err(faultcalc_core::FaultCalcError::new("embed-wasm requires: <template.html> <faultcalc.wasm> <out.html>"));
    }
    let template = fs::read_to_string(&args[0])?;
    let wasm = fs::read(&args[1])?;
    let b64 = base64_encode(&wasm);
    let html = template
        .replace("__WASM_BASE64__", &b64)
        .replace("__BUILD_NOTE__", "standalone embedded Rust WASM");
    write_file(&args[2], html.as_bytes())?;
    println!("wrote {}", args[2]);
    Ok(())
}

fn write_file(path: impl AsRef<Path>, data: &[u8]) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(path, data)?;
    Ok(())
}

fn handle_http(mut stream: TcpStream, root: &Path) -> Result<()> {
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf)?;
    let req = String::from_utf8_lossy(&buf[..n]);
    let first = req.lines().next().unwrap_or("");
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("/");
    if method != "GET" {
        return write_response(&mut stream, "405 Method Not Allowed", "text/plain", b"method not allowed");
    }
    let mut rel = target.split('?').next().unwrap_or("/").trim_start_matches('/').to_string();
    if rel.is_empty() {
        rel = if root.join("faultcalc_workstation.html").exists() {
            "faultcalc_workstation.html".to_string()
        } else {
            "index.html".to_string()
        };
    }
    if rel.contains("..") || rel.starts_with('/') || rel.contains('\\') {
        return write_response(&mut stream, "400 Bad Request", "text/plain", b"bad path");
    }
    let path = root.join(rel);
    if !path.exists() || !path.is_file() {
        return write_response(&mut stream, "404 Not Found", "text/plain", b"not found");
    }
    let body = fs::read(&path)?;
    write_response(&mut stream, "200 OK", mime_for(&path), &body)
}

fn write_response(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8]) -> Result<()> {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    Ok(())
}

fn mime_for(path: &Path) -> &'static str {
    match path.extension().and_then(|x| x.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "wasm" => "application/wasm",
        "json" => "application/json; charset=utf-8",
        "csv" => "text/csv; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i];
        let b1 = if i + 1 < data.len() { data[i + 1] } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] } else { 0 };
        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        if i + 1 < data.len() {
            out.push(TABLE[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if i + 2 < data.len() {
            out.push(TABLE[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
        i += 3;
    }
    out
}

fn print_help() {
    println!(r#"faultcalc - offline Rust IEEE-style short-circuit workstation

Usage:
  faultcalc sample > cases/sample.json
  faultcalc calc cases/sample.json [--json out/report.json] [--csv out/report.csv]
  faultcalc serve web 8080
  faultcalc embed-wasm web/index.template.html web/faultcalc.wasm web/faultcalc_workstation.html
  faultcalc version

Build flow:
  cargo test --workspace
  cargo build -p faultcalc-cli --release
  rustup target add wasm32-unknown-unknown
  cargo build -p faultcalc-wasm --release --target wasm32-unknown-unknown
  cp target/wasm32-unknown-unknown/release/faultcalc_wasm.wasm web/faultcalc.wasm
  cargo run -p faultcalc-cli -- embed-wasm web/index.template.html web/faultcalc.wasm web/faultcalc_workstation.html
"#);
}
