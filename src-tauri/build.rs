fn main() {
    emit_env_from_dotenv("../.env", "GOOGLE_CLIENT_ID");
    emit_env_from_dotenv("../.env", "MICROSOFT_CLIENT_ID");
    tauri_build::build()
}

fn emit_env_from_dotenv(path: &str, key: &str) {
    println!("cargo:rerun-if-changed={path}");
    println!("cargo:rerun-if-env-changed={key}");
    if std::env::var_os(key).is_some() {
        return;
    }

    let Ok(contents) = std::fs::read_to_string(path) else {
        return;
    };
    let Some(value) = dotenv_lookup_from_str(&contents, key) else {
        return;
    };
    println!("cargo:rustc-env={key}={value}");
}

fn dotenv_lookup_from_str(contents: &str, key: &str) -> Option<String> {
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        if name.trim() != key {
            continue;
        }
        let value = value.trim();
        let unquoted = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
            .unwrap_or(value);
        return Some(unquoted.to_string());
    }
    None
}
