use serde::{Deserialize, Serialize};

/// A custom correlation rule defined in pack manifests or by the user.
///
/// Example DSL:
///   WHEN nginx.status >= 500 WITHIN 2s CORRELATE php.error WHERE php.script = nginx.uri
///
/// Parsed into a structured rule:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationRule {
    pub name: String,
    pub source_pack: String,
    pub source_condition: Condition,
    pub target_pack: String,
    pub target_condition: Option<Condition>,
    pub match_fields: Vec<FieldMatch>,
    pub window_seconds: u32,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub field: String,
    pub operator: Operator,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    Contains,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMatch {
    pub source_field: String,
    pub target_field: String,
}

impl CorrelationRule {
    /// Evaluate the source condition against field values.
    pub fn matches_source(&self, fields: &serde_json::Map<String, serde_json::Value>) -> bool {
        evaluate_condition(&self.source_condition, fields)
    }

    /// Evaluate the target condition against field values.
    pub fn matches_target(&self, fields: &serde_json::Map<String, serde_json::Value>) -> bool {
        match &self.target_condition {
            Some(cond) => evaluate_condition(cond, fields),
            None => true, // No condition = always matches
        }
    }
}

fn evaluate_condition(
    condition: &Condition,
    fields: &serde_json::Map<String, serde_json::Value>,
) -> bool {
    let field_value = match fields.get(&condition.field) {
        Some(v) => v,
        None => return false,
    };

    let field_str = match field_value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        _ => return false,
    };

    match condition.operator {
        Operator::Eq => field_str == condition.value,
        Operator::Neq => field_str != condition.value,
        Operator::Contains => field_str.contains(&condition.value),
        Operator::Gt | Operator::Gte | Operator::Lt | Operator::Lte => {
            // Try numeric comparison
            if let (Ok(a), Ok(b)) = (field_str.parse::<f64>(), condition.value.parse::<f64>()) {
                match condition.operator {
                    Operator::Gt => a > b,
                    Operator::Gte => a >= b,
                    Operator::Lt => a < b,
                    Operator::Lte => a <= b,
                    _ => unreachable!(),
                }
            } else {
                // String comparison fallback
                match condition.operator {
                    Operator::Gt => field_str > condition.value,
                    Operator::Gte => field_str >= condition.value,
                    Operator::Lt => field_str < condition.value,
                    Operator::Lte => field_str <= condition.value,
                    _ => unreachable!(),
                }
            }
        }
    }
}

/// Parse a simple DSL rule string.
///
/// Format: `WHEN <pack>.<field> <op> <value> WITHIN <window> CORRELATE <pack>`
pub fn parse_rule(name: &str, input: &str) -> Option<CorrelationRule> {
    let parts: Vec<&str> = input.split_whitespace().collect();

    // Minimal: WHEN pack.field op value WITHIN Ns CORRELATE pack
    if parts.len() < 8 {
        return None;
    }

    if parts[0].to_uppercase() != "WHEN" {
        return None;
    }

    let source_ref: Vec<&str> = parts[1].splitn(2, '.').collect();
    if source_ref.len() != 2 {
        return None;
    }

    let operator = match parts[2].to_uppercase().as_str() {
        "=" | "==" | "EQ" => Operator::Eq,
        "!=" | "NEQ" => Operator::Neq,
        ">" | "GT" => Operator::Gt,
        ">=" | "GTE" => Operator::Gte,
        "<" | "LT" => Operator::Lt,
        "<=" | "LTE" => Operator::Lte,
        "CONTAINS" => Operator::Contains,
        _ => return None,
    };

    if parts[4].to_uppercase() != "WITHIN" {
        return None;
    }

    let window_str = parts[5].trim_end_matches('s');
    let window: u32 = window_str.parse().ok()?;

    if parts[6].to_uppercase() != "CORRELATE" {
        return None;
    }

    let target_pack = parts[7].to_string();

    Some(CorrelationRule {
        name: name.to_string(),
        source_pack: source_ref[0].to_string(),
        source_condition: Condition {
            field: source_ref[1].to_string(),
            operator,
            value: parts[3].to_string(),
        },
        target_pack,
        target_condition: None,
        match_fields: Vec::new(),
        window_seconds: window,
        description: Some(input.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rule_basic() {
        let rule = parse_rule(
            "5xx_correlation",
            "WHEN nginx.status >= 500 WITHIN 2s CORRELATE php-fpm",
        );
        assert!(rule.is_some());
        let rule = rule.unwrap();
        assert_eq!(rule.source_pack, "nginx");
        assert_eq!(rule.source_condition.field, "status");
        assert_eq!(rule.target_pack, "php-fpm");
        assert_eq!(rule.window_seconds, 2);
    }

    #[test]
    fn test_parse_rule_contains() {
        let rule = parse_rule(
            "deadlock_rule",
            "WHEN mysql.message CONTAINS deadlock WITHIN 5s CORRELATE php-fpm",
        );
        assert!(rule.is_some());
        let rule = rule.unwrap();
        assert!(matches!(rule.source_condition.operator, Operator::Contains));
    }

    #[test]
    fn test_evaluate_condition_numeric() {
        let cond = Condition {
            field: "status".into(),
            operator: Operator::Gte,
            value: "500".into(),
        };

        let mut fields = serde_json::Map::new();
        fields.insert("status".into(), serde_json::json!(502));
        assert!(evaluate_condition(&cond, &fields));

        fields.insert("status".into(), serde_json::json!(200));
        assert!(!evaluate_condition(&cond, &fields));
    }

    #[test]
    fn test_evaluate_condition_contains() {
        let cond = Condition {
            field: "message".into(),
            operator: Operator::Contains,
            value: "timeout".into(),
        };

        let mut fields = serde_json::Map::new();
        fields.insert(
            "message".into(),
            serde_json::json!("upstream timeout error"),
        );
        assert!(evaluate_condition(&cond, &fields));
    }

    #[test]
    fn test_rule_matches_source() {
        let rule = parse_rule(
            "test",
            "WHEN nginx.status >= 500 WITHIN 2s CORRELATE php-fpm",
        )
        .unwrap();

        let mut fields = serde_json::Map::new();
        fields.insert("status".into(), serde_json::json!(502));
        assert!(rule.matches_source(&fields));

        fields.insert("status".into(), serde_json::json!(200));
        assert!(!rule.matches_source(&fields));
    }
}
