//! Reviewed boundary configurations, with feature closure read from Cargo.
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ConfigSet(pub u128);
impl ConfigSet {
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
    pub fn contains(self, index: usize) -> bool {
        self.0 & (1u128 << index) != 0
    }
    pub fn insert(&mut self, index: usize) {
        self.0 |= 1u128 << index;
    }
    pub fn intersect(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    pub fn difference(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }
    pub fn indices(self) -> impl Iterator<Item = usize> {
        (0..128).filter(move |index| self.contains(*index))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Configuration {
    pub label: String,
    pub test: bool,
    pub linux: bool,
    pub debug_assertions: bool,
    pub features: BTreeSet<String>,
}

#[derive(Debug)]
pub struct ConfigSpace {
    pub known_features: BTreeSet<String>,
    pub axes: Vec<(String, String)>,
    pub configurations: Vec<Configuration>,
    predicates: RefCell<BTreeMap<String, Vec<Option<bool>>>>,
}
impl ConfigSpace {
    pub fn from_manifest(manifest: &str, axes: &[(String, String)]) -> Result<Self, Vec<String>> {
        Self::from_manifest_with(manifest, axes, &[])
    }
    /// Axis combinations plus explicitly reviewed helper cases, without an automatic feature cross product.
    pub fn from_manifest_with(
        manifest: &str,
        axes: &[(String, String)],
        cases: &[(String, Vec<String>)],
    ) -> Result<Self, Vec<String>> {
        let table = parse_features_table(manifest)?;
        let mut errors = Vec::new();
        let mut labels = BTreeSet::new();
        for (feature, label) in axes {
            if !table.contains_key(feature) {
                errors.push(format!("configuration feature {feature} is not declared"));
            }
            if !labels.insert(label.clone()) {
                errors.push(format!("duplicate configuration label {label}"));
            }
        }
        for (label, features) in cases {
            if !labels.insert(label.clone()) {
                errors.push(format!("duplicate configuration label {label}"));
            }
            for feature in features {
                if !table.contains_key(feature) {
                    errors.push(format!(
                        "configuration case {label} names undeclared feature {feature}"
                    ));
                }
            }
        }
        if axes.len() > 4 || (1usize << axes.len()) + cases.len() > 16 {
            errors.push("reviewed configuration table exceeds 128 points".to_string());
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        let closure = |features: &mut BTreeSet<String>| loop {
            let before = features.len();
            for name in features.clone() {
                features.extend(
                    table
                        .get(&name)
                        .into_iter()
                        .flatten()
                        .filter(|name| table.contains_key(*name))
                        .cloned(),
                );
            }
            if before == features.len() {
                break;
            }
        };
        let mut selections = Vec::new();
        for mask in 0..(1usize << axes.len()) {
            let features = axes
                .iter()
                .enumerate()
                .filter(|(bit, _)| mask & (1 << bit) != 0)
                .map(|(_, (feature, _))| feature.clone())
                .collect::<BTreeSet<_>>();
            let label = axes
                .iter()
                .enumerate()
                .filter(|(bit, _)| mask & (1 << bit) != 0)
                .map(|(_, (_, label))| label.clone())
                .collect::<Vec<_>>()
                .join("-");
            selections
                .push((if label.is_empty() { "default".to_string() } else { label }, features));
        }
        selections.extend(
            cases
                .iter()
                .map(|(label, features)| (label.clone(), features.iter().cloned().collect())),
        );
        let mut configurations = Vec::new();
        for (label, mut features) in selections {
            features.extend(table.get("default").into_iter().flatten().cloned());
            closure(&mut features);
            for test in [false, true] {
                for linux in [true, false] {
                    for debug_assertions in [true, false] {
                        configurations.push(Configuration {
                            label: format!(
                                "{}{}-{}{}",
                                if test { "test-" } else { "" },
                                label,
                                if linux { "linux" } else { "nonlinux" },
                                if debug_assertions { "" } else { "-release" }
                            ),
                            test,
                            linux,
                            debug_assertions,
                            features: features.clone(),
                        });
                    }
                }
            }
        }
        Ok(Self {
            known_features: table.keys().cloned().collect(),
            axes: axes.to_vec(),
            configurations,
            predicates: RefCell::new(BTreeMap::new()),
        })
    }
    pub fn len(&self) -> usize {
        self.configurations.len()
    }
    pub fn is_empty(&self) -> bool {
        self.configurations.is_empty()
    }
    pub fn all(&self) -> ConfigSet {
        ConfigSet(if self.len() == 128 { u128::MAX } else { (1u128 << self.len()) - 1 })
    }
    pub fn labels(&self, set: ConfigSet) -> Vec<&str> {
        set.indices()
            .filter_map(|index| self.configurations.get(index).map(|point| point.label.as_str()))
            .collect()
    }
    pub fn expression(&self, set: ConfigSet) -> String {
        if set == self.all() {
            "all".to_string()
        } else if set.is_empty() {
            "none".to_string()
        } else {
            self.labels(set).join(",")
        }
    }
    /// Evaluates a `cfg` predicate in one configuration; `None` for an
    /// unsupported predicate or a feature the manifest does not declare.
    pub fn evaluate(&self, predicate: &str, index: usize) -> Option<bool> {
        if let Some(values) = self.predicates.borrow().get(predicate) {
            return values[index];
        }
        let compact = predicate.chars().filter(|ch| !ch.is_whitespace()).collect::<String>();
        let values = self
            .configurations
            .iter()
            .map(|point| self.evaluate_compact(&compact, point))
            .collect::<Vec<_>>();
        let value = values[index];
        self.predicates.borrow_mut().insert(predicate.to_string(), values);
        value
    }

    fn evaluate_compact(&self, predicate: &str, configuration: &Configuration) -> Option<bool> {
        if let Some(inner) =
            predicate.strip_prefix("any(").and_then(|value| value.strip_suffix(')'))
        {
            return split_cfg_arguments(inner)
                .into_iter()
                .filter(|argument| !argument.is_empty())
                .map(|argument| self.evaluate_compact(argument, configuration))
                .try_fold(false, |active, value| value.map(|value| active || value));
        }
        if let Some(inner) =
            predicate.strip_prefix("all(").and_then(|value| value.strip_suffix(')'))
        {
            return split_cfg_arguments(inner)
                .into_iter()
                .filter(|argument| !argument.is_empty())
                .map(|argument| self.evaluate_compact(argument, configuration))
                .try_fold(true, |active, value| value.map(|value| active && value));
        }
        if let Some(inner) =
            predicate.strip_prefix("not(").and_then(|value| value.strip_suffix(')'))
        {
            return self.evaluate_compact(inner, configuration).map(|active| !active);
        }
        match predicate {
            "test" => Some(configuration.test),
            "debug_assertions" => Some(configuration.debug_assertions),
            "unix" | "target_os=\"linux\"" => Some(configuration.linux),
            "windows" | "target_os=\"windows\"" => Some(!configuration.linux),
            _ => predicate
                .strip_prefix("feature=\"")
                .and_then(|value| value.strip_suffix('"'))
                .filter(|feature| self.known_features.contains(*feature))
                .map(|feature| configuration.features.contains(feature)),
        }
    }
}
pub(crate) fn split_cfg_arguments(source: &str) -> Vec<&str> {
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut arguments = Vec::new();
    for (index, ch) in source.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                arguments.push(&source[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    arguments.push(&source[start..]);
    arguments
}

/// Parses the `[features]` table of a Cargo manifest: feature name to the
/// in-crate features it enables (`dep:` and `crate/feature` entries are
/// dependency features and do not enable engine `cfg(feature)` names).
pub fn parse_features_table(manifest: &str) -> Result<BTreeMap<String, Vec<String>>, Vec<String>> {
    let mut table = BTreeMap::new();
    let mut in_features = false;
    let mut pending: Option<(String, String)> = None;
    for raw in manifest.lines() {
        let line = strip_toml_comment(raw).trim().to_string();
        if let Some((name, mut value)) = pending.take() {
            value.push(' ');
            value.push_str(&line);
            if value.contains(']') {
                table.insert(name, parse_feature_array(&value)?);
            } else {
                pending = Some((name, value));
            }
            continue;
        }
        if line.starts_with('[') {
            in_features = line == "[features]";
            continue;
        }
        if !in_features || line.is_empty() {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            return Err(vec![format!("cannot parse manifest feature line {raw:?}")]);
        };
        let name = name.trim().trim_matches('"').to_string();
        let value = value.trim().to_string();
        if !value.starts_with('[') {
            return Err(vec![format!("manifest feature {name} is not an array")]);
        }
        if value.contains(']') {
            table.insert(name, parse_feature_array(&value)?);
        } else {
            pending = Some((name, value));
        }
    }
    if pending.is_some() {
        return Err(vec!["unterminated manifest feature array".to_string()]);
    }
    if table.is_empty() {
        return Err(vec!["engine manifest declares no [features] table".to_string()]);
    }
    Ok(table)
}

fn strip_toml_comment(line: &str) -> &str {
    let mut in_string = false;
    for (index, ch) in line.char_indices() {
        match ch {
            '"' => in_string = !in_string,
            '#' if !in_string => return &line[..index],
            _ => {}
        }
    }
    line
}

fn parse_feature_array(value: &str) -> Result<Vec<String>, Vec<String>> {
    let inner = value
        .trim()
        .strip_prefix('[')
        .and_then(|rest| rest.trim_end().strip_suffix(']'))
        .ok_or_else(|| vec![format!("cannot parse manifest feature array {value:?}")])?;
    Ok(inner
        .split(',')
        .map(|entry| entry.trim().trim_matches('"').to_string())
        .filter(|entry| !entry.is_empty() && !entry.contains('/') && !entry.starts_with("dep:"))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"
[package]
name = "x"

[features]
default = []
# comment
tc5-benchmark = []
test-hooks = ["rusqlite/hooks", "dep:libc"]
operator = []
extra = [
    "operator", # trailing comment
    "dep/feature",
]

[[example]]
name = "y"
"#;

    fn axes() -> Vec<(String, String)> {
        vec![
            ("test-hooks".to_string(), "hooks".to_string()),
            ("tc5-benchmark".to_string(), "tc5".to_string()),
            ("operator".to_string(), "operator".to_string()),
        ]
    }

    #[test]
    fn unrelated_manifest_features_do_not_multiply_boundary_configurations() {
        let space = ConfigSpace::from_manifest(MANIFEST, &axes()).expect("space");
        assert_eq!(space.len(), 64);
    }

    #[test]
    fn manifest_features_and_closures_are_parsed() {
        let table = parse_features_table(MANIFEST).expect("manifest parses");
        assert_eq!(table.get("extra"), Some(&vec!["operator".to_string()]));
        assert_eq!(table.get("test-hooks"), Some(&Vec::new()));
        assert_eq!(table.len(), 5);
    }

    #[test]
    fn undeclared_axes_are_rejected() {
        let error =
            ConfigSpace::from_manifest(MANIFEST, &[("missing".to_string(), "missing".to_string())])
                .expect_err("undeclared axis");
        assert!(error[0].contains("configuration feature missing is not declared"));
    }

    #[test]
    fn explicit_helper_case_closes_features_and_covers_release() {
        let space = ConfigSpace::from_manifest_with(
            MANIFEST,
            &axes(),
            &[("helper".to_string(), vec!["extra".to_string()])],
        )
        .unwrap();
        assert_eq!(space.len(), 72);
        let point = space
            .configurations
            .iter()
            .find(|point| point.label == "test-helper-nonlinux-release")
            .unwrap();
        assert!(point.features.contains("operator"));
        assert!(!point.debug_assertions);
        assert_eq!(space.evaluate("feature = \"undeclared\"", 0), None);
    }
}
