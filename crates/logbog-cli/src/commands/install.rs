use crate::output;
use logbog_core::Config;
use logbog_packs::PackRegistry;
use std::path::Path;

/// Packs disponibles dans le registre intégré.
const BUILTIN_PACKS: &[&str] = &["nginx", "php-fpm", "mysql", "systemd", "syslog"];

pub fn run(config_path: &str, packs: &[String]) -> anyhow::Result<()> {
    if packs.is_empty() {
        output::error("No pack names specified. Usage: logbog install <pack> [pack...]");
        output::info("Available packs:");
        for pack in BUILTIN_PACKS {
            println!("  - {pack}");
        }
        return Ok(());
    }

    let path = Path::new(config_path);
    if !path.exists() {
        output::error("LogBog is not initialized. Run 'logbog init' first.");
        return Ok(());
    }

    let config = Config::load(path)?;
    let packs_dir = config.server.data_dir.join("packs");
    let mut registry = PackRegistry::new(&packs_dir);
    registry.scan()?;

    for pack_name in packs {
        if registry.is_installed(pack_name) {
            output::warn(&format!("Pack '{pack_name}' is already installed."));
            continue;
        }

        if !BUILTIN_PACKS.contains(&pack_name.as_str()) {
            output::warn(&format!(
                "Pack '{pack_name}' is not in the builtin registry. Available: {}",
                BUILTIN_PACKS.join(", ")
            ));
            continue;
        }

        output::info(&format!("Installing pack '{pack_name}'..."));

        // Créer le répertoire du pack avec un pack.toml minimal
        let pack_dir = packs_dir.join(pack_name);
        std::fs::create_dir_all(&pack_dir)?;

        let manifest = generate_builtin_manifest(pack_name);
        std::fs::write(pack_dir.join("pack.toml"), manifest)?;

        // Créer le répertoire testdata
        std::fs::create_dir_all(pack_dir.join("testdata"))?;

        output::success(&format!("Pack '{pack_name}' installed successfully."));
    }

    // Re-scanner pour confirmer
    registry.scan()?;
    output::info(&format!("Total packs installed: {}", registry.count()));

    Ok(())
}

fn generate_builtin_manifest(name: &str) -> String {
    match name {
        "nginx" => r#"[pack]
name = "nginx"
version = "1.0.0"
description = "Nginx access and error log parser"
author = "LogBog Team"
tags = ["web", "reverse-proxy", "http"]

[[sources]]
name = "access"
paths = ["/var/log/nginx/access.log", "/var/log/nginx/*-access.log"]
format = "regex"
pattern = '(?P<client_ip>\S+) - (?P<remote_user>\S+) \[(?P<timestamp>[^\]]+)\] "(?P<method>\S+) (?P<uri>\S+) HTTP/(?P<http_version>\S+)" (?P<status>\d+) (?P<body_bytes>\d+) "(?P<referrer>[^"]*)" "(?P<user_agent>[^"]*)"'
timestamp_format = "%d/%b/%Y:%H:%M:%S %z"

[[sources]]
name = "error"
paths = ["/var/log/nginx/error.log", "/var/log/nginx/*-error.log"]
format = "regex"
pattern = '(?P<timestamp>\d{4}/\d{2}/\d{2} \d{2}:\d{2}:\d{2}) \[(?P<level>\w+)\] (?P<pid>\d+)#(?P<tid>\d+): \*(?P<cid>\d+) (?P<message>.*)'
timestamp_format = "%Y/%m/%d %H:%M:%S"

[schema]
fields = [
    { name = "client_ip", type = "ip", indexed = true },
    { name = "method", type = "string", indexed = true },
    { name = "uri", type = "string", indexed = true },
    { name = "status", type = "int", indexed = true },
    { name = "body_bytes", type = "int", indexed = false },
    { name = "user_agent", type = "string", indexed = false },
]

[[alerts]]
name = "5xx_spike"
description = "Spike of 5xx errors"
condition = "count(status >= 500) > 10 in 1m"
severity = "critical"
cooldown = "5m"

[[alerts]]
name = "high_latency"
description = "High request latency"
condition = "avg(request_time) > 5.0 in 5m"
severity = "warning"

[[correlations]]
target_pack = "php-fpm"
match_fields = ["timestamp", "uri:script"]
window = "2s"
"#
        .to_string(),

        "php-fpm" => r#"[pack]
name = "php-fpm"
version = "1.0.0"
description = "PHP-FPM error and slow log parser"
author = "LogBog Team"
tags = ["php", "web", "backend"]

[[sources]]
name = "error"
paths = ["/var/log/php-fpm/error.log", "/var/log/php*/php*-fpm.log"]
format = "regex"
pattern = '\[(?P<timestamp>[^\]]+)\] (?P<level>\w+): (?P<message>.*)'
multiline = true
multiline_start = '^\['

[[sources]]
name = "slow"
paths = ["/var/log/php-fpm/slow.log", "/var/log/php*/slow.log"]
format = "regex"
pattern = '\[(?P<timestamp>[^\]]+)\]\s+\[pool (?P<pool>\w+)\] pid (?P<pid>\d+)'
multiline = true
multiline_start = '^\['

[schema]
fields = [
    { name = "pool", type = "string", indexed = true },
    { name = "pid", type = "int", indexed = true },
    { name = "script", type = "string", indexed = true },
    { name = "error_type", type = "string", indexed = true },
]

[[alerts]]
name = "fatal_errors"
description = "PHP Fatal errors detected"
condition = "count(level = 'Fatal') > 0 in 1m"
severity = "critical"
"#
        .to_string(),

        "mysql" => r#"[pack]
name = "mysql"
version = "1.0.0"
description = "MySQL/MariaDB error and slow query log parser"
author = "LogBog Team"
tags = ["database", "mysql", "mariadb"]

[[sources]]
name = "error"
paths = ["/var/log/mysql/error.log", "/var/log/mysqld.log"]
format = "regex"
pattern = '(?P<timestamp>\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}.\d+Z)\s+(?P<tid>\d+)\s+\[(?P<level>\w+)\]\s+(?P<message>.*)'

[[sources]]
name = "slow"
paths = ["/var/log/mysql/slow.log", "/var/log/mysql/mariadb-slow.log"]
format = "regex"
pattern = '# Time: (?P<timestamp>\S+)\s+# User@Host: (?P<user>\S+)\s*@\s*(?P<host>\S+)'
multiline = true
multiline_start = '^# Time:'

[schema]
fields = [
    { name = "query_time", type = "float", indexed = false },
    { name = "lock_time", type = "float", indexed = false },
    { name = "rows_examined", type = "int", indexed = false },
    { name = "user", type = "string", indexed = true },
    { name = "db", type = "string", indexed = true },
]

[[alerts]]
name = "slow_queries"
description = "Slow queries detected"
condition = "count(query_time > 5.0) > 5 in 5m"
severity = "warning"

[[alerts]]
name = "deadlocks"
description = "Deadlock detected"
condition = "count(message contains 'deadlock') > 0 in 1m"
severity = "critical"
"#
        .to_string(),

        "systemd" => r#"[pack]
name = "systemd"
version = "1.0.0"
description = "Systemd journal reader"
author = "LogBog Team"
tags = ["system", "linux", "journal"]

[[sources]]
name = "journal"
paths = ["journalctl"]
format = "json"

[schema]
fields = [
    { name = "unit", type = "string", indexed = true },
    { name = "priority", type = "int", indexed = true },
    { name = "pid", type = "int", indexed = true },
    { name = "uid", type = "int", indexed = false },
]

[[alerts]]
name = "service_crash"
description = "Service restart loop detected"
condition = "count(message contains 'Started' AND unit = same) > 3 in 5m"
severity = "critical"

[[alerts]]
name = "oom_killer"
description = "OOM Killer invoked"
condition = "count(message contains 'Out of memory') > 0 in 1m"
severity = "critical"
"#
        .to_string(),

        "syslog" => r#"[pack]
name = "syslog"
version = "1.0.0"
description = "Generic syslog and auth log parser (RFC 3164 & 5424)"
author = "LogBog Team"
tags = ["system", "security", "auth"]

[[sources]]
name = "syslog"
paths = ["/var/log/syslog", "/var/log/messages"]
format = "syslog"

[[sources]]
name = "auth"
paths = ["/var/log/auth.log", "/var/log/secure"]
format = "syslog"

[schema]
fields = [
    { name = "hostname", type = "string", indexed = true },
    { name = "app", type = "string", indexed = true },
    { name = "pid", type = "int", indexed = true },
    { name = "facility", type = "int", indexed = false },
]

[[alerts]]
name = "ssh_bruteforce"
description = "SSH brute force attempt"
condition = "count(message contains 'Failed password') > 10 in 5m"
severity = "critical"

[[alerts]]
name = "privilege_escalation"
description = "Privilege escalation detected"
condition = "count(message contains 'sudo' AND message contains 'COMMAND') > 0 in 1m"
severity = "warning"
"#
        .to_string(),

        _ => format!(
            r#"[pack]
name = "{name}"
version = "0.1.0"
description = "Custom pack for {name}"

[[sources]]
name = "main"
paths = ["/var/log/{name}/*.log"]
format = "regex"
pattern = '(?P<timestamp>\S+) (?P<level>\w+) (?P<message>.*)'
"#
        ),
    }
}
