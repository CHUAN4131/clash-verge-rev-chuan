use serde_yaml_ng::{Mapping, Value};
use std::sync::Arc;
use tristr::alias::String;

#[derive(Clone, Default)]
pub(crate) struct AppDirectRules {
    original: Option<Arc<Mapping>>,
}

impl AppDirectRules {
    pub(super) fn apply(&mut self, config: &mut Mapping, paths: &[String]) {
        const KEYS: [&str; 2] = ["rules", "find-process-mode"];

        if let Some(original) = self.original.take() {
            for key in KEYS {
                if let Some(value) = original.get(key) {
                    config.insert(key.into(), value.clone());
                } else {
                    config.remove(key);
                }
            }
        }
        if paths.is_empty() {
            return;
        }

        let original: Mapping = KEYS
            .into_iter()
            .filter_map(|key| config.get(key).map(|value| (key.into(), value.clone())))
            .collect();
        let rules = config.entry("rules".into()).or_insert(Value::Null);
        if rules.is_null() {
            *rules = Value::Sequence(Vec::new());
        }
        let Some(rules) = rules.as_sequence_mut() else {
            return;
        };
        drop(rules.splice(..0, paths.iter().map(|path| direct_rule(path.as_str()))));

        if config
            .get("find-process-mode")
            .and_then(Value::as_str)
            .is_some_and(|mode| mode.eq_ignore_ascii_case("off"))
        {
            config.insert("find-process-mode".into(), "strict".into());
        }
        self.original = Some(Arc::new(original));
    }
}

fn direct_rule(path: &str) -> Value {
    if !path.contains(',') {
        return Value::String(format!("PROCESS-PATH,{path},DIRECT"));
    }

    // Mihomo splits rule fields at commas. Encode literal commas inside an exact path regex.
    let mut pattern = std::string::String::from(r"PROCESS-PATH-REGEX,\A");
    for character in path.chars() {
        match character {
            ',' => pattern.push_str(r"\x2c"),
            '\\' | '.' | '^' | '$' | '|' | '?' | '*' | '+' | '(' | ')' | '[' | ']' | '{' | '}' => {
                pattern.push('\\');
                pattern.push(character);
            }
            _ => pattern.push(character),
        }
    }
    pattern.push_str(r"\z,DIRECT");
    Value::String(pattern)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changing_app_rules_preserves_subscription_rules_and_runtime_chain() -> Result<(), serde_yaml_ng::Error> {
        let mut config = Mapping::new();
        config.insert("find-process-mode".into(), "OFF".into());
        config.insert("rules".into(), Value::Sequence(vec!["MATCH,Proxy".into()]));
        let mut apps = AppDirectRules::default();

        apps.apply(&mut config, &[r"C:\Program Files\App\app.exe".into()]);
        assert_eq!(
            config.get("rules"),
            Some(&Value::Sequence(vec![
                r"PROCESS-PATH,C:\Program Files\App\app.exe,DIRECT".into(),
                "MATCH,Proxy".into(),
            ])),
        );

        let proxies: Value = serde_yaml_ng::from_str("- { name: Exit, dialer-proxy: Entry }")?;
        config.insert("proxies".into(), proxies.clone());
        apps.apply(&mut config, &[r"C:\Apps\Name, Inc. (x64)\app.exe".into()]);
        assert_eq!(config.get("find-process-mode").and_then(Value::as_str), Some("strict"));
        assert_eq!(
            config.get("rules"),
            Some(&Value::Sequence(vec![
                r"PROCESS-PATH-REGEX,\AC:\\Apps\\Name\x2c Inc\. \(x64\)\\app\.exe\z,DIRECT".into(),
                "MATCH,Proxy".into(),
            ])),
        );

        apps.apply(&mut config, &[]);
        assert_eq!(config.get("find-process-mode").and_then(Value::as_str), Some("OFF"));
        assert_eq!(config.get("rules"), Some(&Value::Sequence(vec!["MATCH,Proxy".into()])));
        assert_eq!(config.get("proxies"), Some(&proxies));
        Ok(())
    }
}
