//! Auswertung der `rules` aus Version-JSONs (Betriebssystem, Architektur, Features).

use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    Allow,
    Disallow,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub action: RuleAction,
    #[serde(default)]
    pub os: Option<OsRule>,
    #[serde(default)]
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct OsRule {
    pub name: Option<String>,
    pub arch: Option<String>,
    /// Regulärer Ausdruck auf die OS-Version, z. B. `^10\\.`.
    pub version: Option<String>,
}

/// Die Umgebung, gegen die Rules ausgewertet werden.
#[derive(Debug, Clone)]
pub struct Environment {
    /// `windows`, `linux` oder `osx` (Mojang-Schreibweise).
    pub os_name: String,
    /// `x86`, `x86_64` oder `arm64`.
    pub arch: String,
    pub os_version: String,
    pub features: HashMap<String, bool>,
}

impl Environment {
    pub fn current() -> Self {
        let os_name = match std::env::consts::OS {
            "macos" => "osx",
            other => other,
        };
        let arch = match std::env::consts::ARCH {
            "aarch64" => "arm64",
            "x86" => "x86",
            other => other,
        };
        Self {
            os_name: os_name.to_string(),
            arch: arch.to_string(),
            os_version: String::new(),
            features: HashMap::new(),
        }
    }

    /// Bitbreite für `${arch}` in alten Natives-Klassifizierern (`natives-windows-${arch}`).
    pub fn arch_bits(&self) -> &'static str {
        if self.arch == "x86" { "32" } else { "64" }
    }
}

impl Rule {
    fn matches(&self, env: &Environment) -> bool {
        if let Some(os) = &self.os {
            if os.name.as_deref().is_some_and(|n| n != env.os_name) {
                return false;
            }
            if os.arch.as_deref().is_some_and(|a| a != env.arch) {
                return false;
            }
            if let Some(pattern) = &os.version {
                // Ungültige Muster gelten als nicht erfüllt, statt den Start zu blockieren.
                let ok = regex::Regex::new(pattern)
                    .map(|re| re.is_match(&env.os_version))
                    .unwrap_or(false);
                if !ok {
                    return false;
                }
            }
        }
        if let Some(features) = &self.features {
            for (name, wanted) in features {
                if env.features.get(name).copied().unwrap_or(false) != *wanted {
                    return false;
                }
            }
        }
        true
    }
}

/// Wertet eine Rule-Liste aus wie der offizielle Launcher: ohne Rules erlaubt,
/// sonst entscheidet die letzte passende Rule (Standard: verboten).
pub fn allowed(rules: &[Rule], env: &Environment) -> bool {
    if rules.is_empty() {
        return true;
    }
    let mut result = false;
    for rule in rules {
        if rule.matches(env) {
            result = rule.action == RuleAction::Allow;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(os: &str, arch: &str) -> Environment {
        Environment {
            os_name: os.into(),
            arch: arch.into(),
            os_version: "10.0".into(),
            features: HashMap::new(),
        }
    }

    fn rules(json: &str) -> Vec<Rule> {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn empty_rules_allow() {
        assert!(allowed(&[], &env("linux", "x86_64")));
    }

    #[test]
    fn allow_all_except_osx() {
        let r = rules(r#"[{"action":"allow"},{"action":"disallow","os":{"name":"osx"}}]"#);
        assert!(allowed(&r, &env("linux", "x86_64")));
        assert!(allowed(&r, &env("windows", "x86_64")));
        assert!(!allowed(&r, &env("osx", "arm64")));
    }

    #[test]
    fn only_specific_os() {
        let r = rules(r#"[{"action":"allow","os":{"name":"windows"}}]"#);
        assert!(allowed(&r, &env("windows", "x86_64")));
        assert!(!allowed(&r, &env("linux", "x86_64")));
    }

    #[test]
    fn arch_and_version() {
        let r = rules(r#"[{"action":"allow","os":{"arch":"x86"}}]"#);
        assert!(allowed(&r, &env("windows", "x86")));
        assert!(!allowed(&r, &env("windows", "x86_64")));

        let r = rules(r#"[{"action":"allow","os":{"name":"windows","version":"^10\\."}}]"#);
        assert!(allowed(&r, &env("windows", "x86_64")));
        let mut old = env("windows", "x86_64");
        old.os_version = "6.1".into();
        assert!(!allowed(&r, &old));
    }

    #[test]
    fn features() {
        let r = rules(r#"[{"action":"allow","features":{"has_custom_resolution":true}}]"#);
        let mut e = env("linux", "x86_64");
        assert!(!allowed(&r, &e));
        e.features.insert("has_custom_resolution".into(), true);
        assert!(allowed(&r, &e));
    }
}
