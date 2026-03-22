use crate::config::{config_path, load_config_file, save_config_file};
use owo_colors::OwoColorize;
use std::io::IsTerminal;

pub fn handle(args: &[String]) {
    match args.first().map(|s| s.as_str()) {
        None => show(),
        Some("set") => set(&args[1..]),
        Some("path") => println!("{}", config_path().display()),
        Some(other) => {
            eprintln!("unknown config command: {other}");
            eprintln!("usage: distill config [set <key> <value> | path]");
            std::process::exit(1);
        }
    }
}

fn mask_key(key: &str) -> String {
    if key.len() <= 7 {
        "***".to_string()
    } else {
        format!("{}...{}", &key[..3], &key[key.len() - 4..])
    }
}

fn resolve_with_source(env_var: &str, file_value: &Option<String>) -> (Option<String>, Option<&'static str>) {
    if let Ok(val) = std::env::var(env_var) {
        return (Some(val), Some("env"));
    }
    if let Some(val) = file_value {
        return (Some(val.clone()), Some("config file"));
    }
    (None, None)
}

fn show() {
    let color = std::io::stderr().is_terminal();
    let file = load_config_file();

    let settings: &[(&str, &str, &Option<String>, bool)] = &[
        ("api_key", "DISTILL_API_KEY", &file.api_key, true),
        ("api_base", "DISTILL_API_BASE", &file.api_base, false),
        ("model", "DISTILL_MODEL", &file.model, false),
    ];

    for &(key, env_var, file_val, is_secret) in settings {
        let (value, source) = resolve_with_source(env_var, file_val);
        print_setting(key, value.as_deref(), source, is_secret, color);
    }

    // Bool/numeric settings from config file only
    let parallel_str = file.parallel.map(|v| v.to_string());
    let jobs_str = file.jobs.map(|v| v.to_string());
    let level_str = file.level.clone();

    let file_settings: &[(&str, &Option<String>)] = &[
        ("level", &level_str),
        ("parallel", &parallel_str),
        ("jobs", &jobs_str),
    ];

    for &(key, value) in file_settings {
        let source = value.as_ref().map(|_| "config file");
        print_setting(key, value.as_deref(), source, false, color);
    }

    eprintln!();
    if color {
        eprintln!(
            "  {} {}",
            "config file:".dimmed(),
            config_path().display()
        );
    } else {
        eprintln!("  config file: {}", config_path().display());
    }
}

fn print_setting(key: &str, value: Option<&str>, source: Option<&str>, is_secret: bool, color: bool) {
    let display_val = match value {
        Some(v) if is_secret => mask_key(v),
        Some(v) => v.to_string(),
        None => "not set".to_string(),
    };

    let source_label = source.unwrap_or("not set");

    if color {
        match value {
            Some(_) => {
                eprintln!(
                    "  {:<12} {} {}",
                    key.bold(),
                    display_val,
                    format!("({source_label})").dimmed(),
                );
            }
            None => {
                eprintln!(
                    "  {:<12} {}",
                    key.bold(),
                    display_val.dimmed(),
                );
            }
        }
    } else {
        match value {
            Some(_) => eprintln!("  {key:<12} {display_val} ({source_label})"),
            None => eprintln!("  {key:<12} {display_val}"),
        }
    }
}

fn set(args: &[String]) {
    if args.len() != 2 {
        eprintln!("usage: distill config set <key> <value>");
        std::process::exit(1);
    }

    let key = &args[0];
    let value = &args[1];

    let mut file = load_config_file();

    match key.as_str() {
        "api_key" => file.api_key = Some(value.clone()),
        "api_base" => file.api_base = Some(value.clone()),
        "model" => file.model = Some(value.clone()),
        "level" => file.level = Some(value.clone()),
        "parallel" => {
            file.parallel = Some(value.parse::<bool>().unwrap_or_else(|_| {
                eprintln!("invalid value for parallel: expected true or false");
                std::process::exit(1);
            }));
        }
        "jobs" => {
            file.jobs = Some(value.parse::<usize>().unwrap_or_else(|_| {
                eprintln!("invalid value for jobs: expected a positive integer");
                std::process::exit(1);
            }));
        }
        other => {
            eprintln!("unknown config key: {other}");
            eprintln!("valid keys: api_key, api_base, model, level, parallel, jobs");
            std::process::exit(1);
        }
    }

    if let Err(e) = save_config_file(&file) {
        eprintln!("failed to save config: {e}");
        std::process::exit(1);
    }

    let display_val = if key == "api_key" {
        mask_key(value)
    } else {
        value.clone()
    };

    eprintln!("set {key} = {display_val}");
}
