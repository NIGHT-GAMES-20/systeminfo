use std::fs;

pub fn read_sys_file(interface: &str, file: &str) -> String {
    let path = format!("/sys/class/net/{}/{}", interface, file);
    fs::read_to_string(path)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

pub fn clean_float(val: Option<f32>) -> Option<f32> {
    val.filter(|v| !v.is_nan())
}
