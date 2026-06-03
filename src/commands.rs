use crate::app_config::AppConfig;
use crate::config;
use crate::error::SetContextError;

use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
    path::Path,
    process::{Command, Stdio},
    sync::Arc,
};
extern crate skim;
use kube::config::Kubeconfig;
use regex::Regex;
use skim::prelude::*;

pub fn set_default_namespace(ns: &str, ctx: &str, target: &Path) {
    Command::new("kubectl")
        .arg("config")
        .arg(format!("--kubeconfig={}", target.to_string_lossy()))
        .arg("set-context")
        .arg(ctx)
        .arg(format!("--namespace={}", ns))
        .stdout(Stdio::null())
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
}

pub fn set_default_context(ctx: &str, target: &Path) {
    let output = Command::new("kubectl")
        .arg("config")
        .arg(format!("--kubeconfig={}", target.to_string_lossy()))
        .arg("use-context")
        .arg(ctx)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute command");

    if !output.status.success() {
        eprintln!("Error: {}", String::from_utf8_lossy(&output.stderr));
    }
}

pub fn get_namespaces() -> Vec<String> {
    let output = Command::new("kubectl")
        .args(["get", "namespace", "-o=custom-columns=Name:.metadata.name"])
        .output()
        .unwrap();

    let string = String::from_utf8(output.stdout).unwrap();
    string.lines().skip(1).map(ToOwned::to_owned).collect()
}

struct ContextItem {
    name: String,
    display: String,
}

impl SkimItem for ContextItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.name)
    }

    fn display<'a>(&'a self, _context: DisplayContext<'a>) -> AnsiString<'a> {
        AnsiString::parse(&self.display)
    }

    fn output(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.name)
    }
}

struct PrioritizedEngine {
    inner: Box<dyn MatchEngine>,
    // Maps context name → tier penalty added to rank[0] (higher = sorted later)
    penalties: Arc<HashMap<String, i32>>,
}

impl MatchEngine for PrioritizedEngine {
    fn match_item(&self, item: Arc<dyn SkimItem>) -> Option<MatchResult> {
        self.inner.match_item(item.clone()).map(|mut result| {
            let name = item.output();
            if let Some(&penalty) = self.penalties.get(name.as_ref()) {
                // rank[0] is -score (sorted ascending = lower first).
                // Adding a penalty pushes higher-tier items later in the list.
                result.rank[0] = result.rank[0].saturating_add(penalty);
            }
            result
        })
    }
}

impl Display for PrioritizedEngine {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "PrioritizedEngine({})", self.inner)
    }
}

struct PrioritizedMatchEngineFactory {
    inner: AndOrEngineFactory,
    penalties: Arc<HashMap<String, i32>>,
}

impl MatchEngineFactory for PrioritizedMatchEngineFactory {
    fn create_engine_with_case(&self, query: &str, case: CaseMatching) -> Box<dyn MatchEngine> {
        let inner = self.inner.create_engine_with_case(query, case);
        Box::new(PrioritizedEngine {
            inner,
            penalties: Arc::clone(&self.penalties),
        })
    }
}

/// Prompts the user to select an item from a list.
/// Returns the selected item or `None` if no item was selected.
pub fn selectable_list(input: Vec<String>, cfg: &AppConfig) -> Option<String> {
    let selector = &cfg.selector;

    // Pre-compile priority regexes
    let priority_regexes: Vec<(u32, Regex)> = selector
        .priority_rules
        .iter()
        .filter_map(|rule| match Regex::new(&rule.pattern) {
            Ok(re) => Some((rule.priority, re)),
            Err(e) => {
                eprintln!(
                    "kubesess: warning: invalid priority pattern {:?}: {}",
                    rule.pattern, e
                );
                None
            }
        })
        .collect();

    // Pre-compile color regexes
    let color_regexes: Vec<(&str, Regex)> = selector
        .color_rules
        .iter()
        .filter_map(|rule| match Regex::new(&rule.pattern) {
            Ok(re) => Some((rule.ansi.as_str(), re)),
            Err(e) => {
                eprintln!(
                    "kubesess: warning: invalid color pattern {:?}: {}",
                    rule.pattern, e
                );
                None
            }
        })
        .collect();

    let has_priority = !priority_regexes.is_empty();
    let has_colors = !color_regexes.is_empty();

    // Build score-offset map (positive offset → more negative rank[0] → sorts first)
    let penalties: HashMap<String, i32> = if has_priority {
        let score_offset = selector.score_offset;
        input
            .iter()
            .map(|name| {
                let tier = priority_regexes
                    .iter()
                    .find(|(_, re)| re.is_match(name))
                    .map(|(p, _)| *p)
                    .unwrap_or(u32::MAX);
                // Add tier * score_offset to rank[0] (which is -score, sorted ascending).
                // Adding a larger penalty for higher tier numbers pushes them later.
                // Tier 1 → +10, tier 7 → +70, unmatched (u32::MAX) → capped.
                let penalty = (tier.min(10_000) as i32).saturating_mul(score_offset);
                (name.clone(), penalty)
            })
            .collect()
    } else {
        HashMap::new()
    };

    // Build items (apply color if configured)
    let items: Vec<ContextItem> = {
        let mut v: Vec<ContextItem> = input
            .into_iter()
            .map(|name| {
                let display = if has_colors {
                    color_regexes
                        .iter()
                        .find(|(_, re)| re.is_match(&name))
                        .map(|(ansi, _)| format!("{}{}\x1b[0m", ansi, name))
                        .unwrap_or_else(|| name.clone())
                } else {
                    name.clone()
                };
                ContextItem { name, display }
            })
            .collect();

        // Preserve original behaviour (most recent first) when no priority rules
        if !has_priority {
            v.reverse();
        }
        v
    };

    let (tx, rx): (SkimItemSender, SkimItemReceiver) = unbounded();
    for item in items {
        let _ = tx.send(Arc::new(item));
    }
    drop(tx);

    let height = selector.height.clone();
    let prompt = selector.prompt.clone();

    let mut builder = SkimOptionsBuilder::default();
    builder
        .multi(false)
        .height(Some(&height))
        .prompt(Some(&prompt));

    if selector.no_sort {
        builder.nosort(true);
    }

    if selector.layout == "reverse" || selector.layout == "reverse-list" {
        builder.reverse(true);
    }

    if has_priority {
        let factory = PrioritizedMatchEngineFactory {
            inner: AndOrEngineFactory::new(ExactOrFuzzyEngineFactory::builder().build()),
            penalties: Arc::new(penalties),
        };
        builder.engine_factory(Some(Rc::new(factory)));
    }

    let options = builder.build().unwrap();

    Skim::run_with(&options, Some(rx))
        .and_then(|out| match out.final_key {
            Key::Enter => Some(out.selected_items),
            _ => None,
        })
        .filter(|selected_items| !selected_items.is_empty())
        .map(|selected_items| selected_items[0].output().to_string())
}

pub fn set_namespace(ctx: &str, selection: &str, temp_dir: &str, config: &Kubeconfig) -> String {
    let choice = config.contexts.iter().find(|x| x.name == ctx);
    config::write(choice.unwrap(), Some(selection), temp_dir, config)
}

pub fn set_context(
    ctx: &str,
    temp_dir: &str,
    config: &Kubeconfig,
) -> Result<String, SetContextError> {
    if let Some(choice) = config.contexts.iter().find(|x| x.name == ctx) {
        let filename = config::write(choice, None, temp_dir, config);
        Ok(filename)
    } else {
        Err(SetContextError::KubeContextNotFound {
            ctx: ctx.to_owned(),
        })
    }
}
