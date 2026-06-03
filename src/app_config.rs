use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Debug, Deserialize, Default, Clone)]
pub struct AppConfig {
    #[serde(default)]
    pub selector: SelectorConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SelectorConfig {
    #[serde(default = "default_height")]
    pub height: String,
    #[serde(default = "default_layout")]
    pub layout: String,
    #[serde(default = "default_prompt")]
    pub prompt: String,
    #[serde(default)]
    pub no_sort: bool,
    #[serde(default = "default_score_offset")]
    pub score_offset: i32,
    #[serde(default)]
    pub priority_rules: Vec<PriorityRule>,
    #[serde(default)]
    pub color_rules: Vec<ColorRule>,
}

impl Default for SelectorConfig {
    fn default() -> Self {
        Self {
            height: default_height(),
            layout: default_layout(),
            prompt: default_prompt(),
            no_sort: false,
            score_offset: default_score_offset(),
            priority_rules: Vec::new(),
            color_rules: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct PriorityRule {
    pub pattern: String,
    pub priority: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ColorRule {
    pub pattern: String,
    /// ANSI escape sequence to prefix the context name.
    /// Use TOML unicode escapes, e.g. "[34m" for blue.
    pub ansi: String,
}

fn default_height() -> String {
    "100%".to_string()
}
fn default_layout() -> String {
    "default".to_string()
}
fn default_prompt() -> String {
    "> ".to_string()
}
fn default_score_offset() -> i32 {
    10
}

pub fn config_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"));
    base.join("kubesess").join("config.toml")
}

static APP_CONFIG: OnceLock<AppConfig> = OnceLock::new();

pub fn get() -> &'static AppConfig {
    APP_CONFIG.get_or_init(load)
}

fn load() -> AppConfig {
    let path = config_path();
    if !path.exists() {
        return AppConfig::default();
    }
    match fs::read_to_string(&path)
        .map_err(|e| e.to_string())
        .and_then(|s| toml::from_str::<AppConfig>(&s).map_err(|e| e.to_string()))
    {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("kubesess: warning: {}: {}", path.display(), e);
            AppConfig::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_no_op() {
        let cfg = AppConfig::default();
        assert!(cfg.selector.priority_rules.is_empty());
        assert!(cfg.selector.color_rules.is_empty());
        assert!(!cfg.selector.no_sort);
        assert_eq!(cfg.selector.layout, "default");
        assert_eq!(cfg.selector.height, "100%");
    }

    #[test]
    fn test_parse_minimal_toml() {
        let toml_str = r#"
[selector]
height = "60%"
layout = "reverse"
no_sort = true
"#;
        let cfg: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.selector.height, "60%");
        assert_eq!(cfg.selector.layout, "reverse");
        assert!(cfg.selector.no_sort);
        assert!(cfg.selector.priority_rules.is_empty());
    }

    #[test]
    fn test_parse_priority_rules() {
        let toml_str = r#"
[[selector.priority_rules]]
pattern = "^prod"
priority = 1

[[selector.priority_rules]]
pattern = "^dev"
priority = 5
"#;
        let cfg: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.selector.priority_rules.len(), 2);
        assert_eq!(cfg.selector.priority_rules[0].priority, 1);
        assert_eq!(cfg.selector.priority_rules[1].priority, 5);
    }

    #[test]
    fn test_parse_color_rules() {
        // TOML does not support \x1b; use the  unicode escape for ESC.
        let toml_str = "[[selector.color_rules]]\npattern = \"^prod\"\nansi = \"\\u001b[32m\"\n";
        let cfg: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.selector.color_rules.len(), 1);
        assert_eq!(cfg.selector.color_rules[0].pattern, "^prod");
        assert!(cfg.selector.color_rules[0].ansi.contains('\x1b'));
    }

    #[test]
    fn test_invalid_toml_is_error() {
        let result = toml::from_str::<AppConfig>("invalid = [[[");
        assert!(result.is_err());
    }
}
