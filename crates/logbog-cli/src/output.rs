use colored::Colorize;

pub fn banner() {
    let version = env!("CARGO_PKG_VERSION");
    println!(
        r#"
  {}
  {}
  Version {}
"#,
        "╔══════════════════════════════╗".bright_cyan(),
        "║         LogBog               ║".bright_cyan(),
        version.bright_yellow()
    );
}

pub fn success(msg: &str) {
    println!("{} {}", "[OK]".bright_green().bold(), msg);
}

pub fn info(msg: &str) {
    println!("{} {}", "[*]".bright_blue().bold(), msg);
}

pub fn warn(msg: &str) {
    println!("{} {}", "[!]".bright_yellow().bold(), msg);
}

pub fn error(msg: &str) {
    eprintln!("{} {}", "[ERR]".bright_red().bold(), msg);
}

pub fn header(msg: &str) {
    println!("\n{}", msg.bold().underline());
}

pub fn item(key: &str, value: &str) {
    println!("  {} {}", format!("{key}:").dimmed(), value);
}
