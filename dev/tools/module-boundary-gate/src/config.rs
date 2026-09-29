//! Compiler configuration space derived from the engine manifest.
//!
//! Every evaluated configuration is a point over boolean `test`, reviewed
//! axis-feature, `linux` and `debug` variables plus one `profile` variable:
//!
//! - profile `base` enumerates the axis features (named by the policy) in
//!   every combination, with no other feature;
//! - every other `[features]` entry gets its own profile enabling exactly
//!   that feature's closure, and every reviewed consumer profile (a named
//!   feature list from the policy) enables the closure of its list; each of
//!   these profiles is crossed with every combination of the policy's
//!   `configuration-cross` axis features, all other axes off;
//! - profile `all-features` enables the closure of every declared feature.
//!
//! Each point is crossed with test/non-test, Linux/non-Linux and
//! debug/release. A feature set outside this enumeration (for example two
//! non-axis features that no consumer profile combines, or a non-cross axis
//! together with a non-axis feature) is not evaluated; only the all-features
//! profile contains it.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Mutex;

/// Maximum number of evaluated configurations.
pub const MAX_CONFIGURATIONS: usize = 256;

/// A set of evaluated configurations, as a bit mask over [`ConfigSpace`].
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ConfigSet(pub [u128; 2]);

impl ConfigSet {
    pub fn is_empty(self) -> bool {
        self.0 == [0, 0]
    }

    pub fn contains(self, index: usize) -> bool {
        self.0[index / 128] & (1u128 << (index % 128)) != 0
    }

    pub fn insert(&mut self, index: usize) {
        self.0[index / 128] |= 1u128 << (index % 128);
    }

    pub fn intersect(self, other: Self) -> Self {
        Self([self.0[0] & other.0[0], self.0[1] & other.0[1]])
    }

    pub fn union(self, other: Self) -> Self {
        Self([self.0[0] | other.0[0], self.0[1] | other.0[1]])
    }

    pub fn difference(self, other: Self) -> Self {
        Self([self.0[0] & !other.0[0], self.0[1] & !other.0[1]])
    }

    pub fn len(self) -> u32 {
        self.0[0].count_ones() + self.0[1].count_ones()
    }

    pub fn is_subset(self, other: Self) -> bool {
        self.difference(other).is_empty()
    }

    pub fn indices(self) -> impl Iterator<Item = usize> {
        (0..MAX_CONFIGURATIONS).filter(move |index| self.contains(*index))
    }
}

/// One evaluated compiler configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Configuration {
    pub label: String,
    pub test: bool,
    pub linux: bool,
    pub debug_assertions: bool,
    /// `base`, `all-features`, or the non-axis feature whose closure is on.
    pub profile: String,
    /// Enabled engine features, closed under the manifest's implications.
    pub features: BTreeSet<String>,
}

/// The configuration space: manifest features, reviewed axes, and points.
#[derive(Debug)]
pub struct ConfigSpace {
    pub known_features: BTreeSet<String>,
    /// `(feature, label)` pairs in policy order.
    pub axes: Vec<(String, String)>,
    /// Profile values in configuration order: `base`, one per non-axis
    /// feature, one per consumer profile, then `all-features`.
    pub profiles: Vec<String>,
    /// Axis features crossed with every single-feature and consumer profile.
    pub cross: Vec<String>,
    /// Reviewed consumer profiles: name and the features it enables.
    pub consumer_profiles: Vec<(String, Vec<String>)>,
    pub configurations: Vec<Configuration>,
    expressions: Mutex<HashMap<ConfigSet, String>>,
    /// Members of each boolean variable's `[false, true]` literal.
    literal_members: Vec<[ConfigSet; 2]>,
    /// Members of each profile value.
    profile_members: Vec<ConfigSet>,
}

const ALL_FEATURES: &str = "all-features";
const BASE: &str = "base";

impl ConfigSpace {
    /// Builds the space from the engine `Cargo.toml` text and the policy's
    /// `configuration-feature <feature> <label>` axes, with no crossed axes
    /// and no consumer profiles.
    pub fn from_manifest(manifest: &str, axes: &[(String, String)]) -> Result<Self, Vec<String>> {
        Self::from_manifest_with(manifest, axes, &[], &[])
    }

    /// Builds the space from the engine `Cargo.toml` text, the policy's
    /// axes, the axes crossed with every non-base profile
    /// (`configuration-cross <feature>`) and the reviewed consumer profiles
    /// (`configuration-profile <name> <feature>[,<feature>..]`).
    pub fn from_manifest_with(
        manifest: &str,
        axes: &[(String, String)],
        cross: &[String],
        consumer_profiles: &[(String, Vec<String>)],
    ) -> Result<Self, Vec<String>> {
        let table = parse_features_table(manifest)?;
        let mut errors = Vec::new();
        for feature in cross {
            if !axes.iter().any(|(axis, _)| axis == feature) {
                errors.push(format!(
                    "configuration-cross {feature} is not a configuration-feature axis"
                ));
            }
        }
        for (name, features) in consumer_profiles {
            if table.contains_key(name)
                || matches!(name.as_str(), "base" | "all-features")
                || name.contains(['&', '|', '!', ',', ':'])
                || consumer_profiles.iter().filter(|(other, _)| other == name).count() > 1
            {
                errors.push(format!(
                    "configuration-profile {name} collides with a feature, a reserved profile or \
                     another profile"
                ));
            }
            if features.is_empty() {
                errors.push(format!("configuration-profile {name} names no feature"));
            }
            for feature in features {
                if !table.contains_key(feature) {
                    errors.push(format!(
                        "configuration-profile {name} feature {feature} is not declared in the \
                         engine manifest [features] table"
                    ));
                }
                if axes.iter().any(|(axis, _)| axis == feature) {
                    errors.push(format!(
                        "configuration-profile {name} feature {feature} is an axis; profiles \
                         name non-axis features"
                    ));
                }
            }
        }
        let mut labels = BTreeSet::new();
        for (feature, label) in axes {
            if !table.contains_key(feature) {
                errors.push(format!(
                    "configuration feature {feature} is not declared in the engine manifest \
                     [features] table"
                ));
            }
            if matches!(label.as_str(), "test" | "linux" | "debug" | "all" | "none" | "profile")
                || !labels.insert(label.clone())
                || label.contains(['&', '|', '!', ',', ':'])
            {
                errors
                    .push(format!("configuration feature label {label} is reserved or duplicated"));
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        let closure = |seed: &mut BTreeSet<String>| {
            let mut pending = seed.iter().cloned().collect::<Vec<_>>();
            while let Some(feature) = pending.pop() {
                for implied in table.get(&feature).into_iter().flatten() {
                    if seed.insert(implied.clone()) {
                        pending.push(implied.clone());
                    }
                }
            }
        };
        let mut defaults = table.get("default").cloned().unwrap_or_default().into_iter().collect();
        closure(&mut defaults);
        let mut profiles = vec![BASE.to_string()];
        profiles.extend(
            table
                .keys()
                .filter(|feature| {
                    feature.as_str() != "default" && !axes.iter().any(|(axis, _)| axis == *feature)
                })
                .cloned(),
        );
        profiles.extend(consumer_profiles.iter().map(|(name, _)| name.clone()));
        profiles.push(ALL_FEATURES.to_string());
        let cross_indices = axes
            .iter()
            .enumerate()
            .filter(|(_, (feature, _))| cross.contains(feature))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let mut configurations = Vec::new();
        for test in [false, true] {
            for profile in &profiles {
                // Axis masks: every combination under base; every
                // combination of the crossed axes under the other profiles
                // except all-features.
                let masks = if profile == BASE {
                    (0..(1usize << axes.len())).collect::<Vec<_>>()
                } else if profile == ALL_FEATURES {
                    vec![0]
                } else {
                    (0..(1usize << cross_indices.len()))
                        .map(|combination| {
                            cross_indices
                                .iter()
                                .enumerate()
                                .filter(|(bit, _)| combination & (1 << bit) != 0)
                                .fold(0usize, |mask, (_, axis)| mask | (1 << axis))
                        })
                        .collect()
                };
                for mask in masks {
                    for linux in [true, false] {
                        for debug_assertions in [true, false] {
                            let mut features = defaults.clone();
                            let mut parts = Vec::new();
                            if test {
                                parts.push("test".to_string());
                            }
                            for (index, (feature, label)) in axes.iter().enumerate() {
                                if mask & (1 << index) != 0 {
                                    features.insert(feature.clone());
                                    parts.push(label.clone());
                                }
                            }
                            if profile == ALL_FEATURES {
                                features.extend(table.keys().cloned());
                                parts.push(ALL_FEATURES.to_string());
                            } else if let Some((_, enabled)) =
                                consumer_profiles.iter().find(|(name, _)| name == profile)
                            {
                                features.extend(enabled.iter().cloned());
                                parts.push(format!("profile-{profile}"));
                            } else if profile != BASE {
                                features.insert(profile.clone());
                                parts.push(format!("feature-{profile}"));
                            }
                            closure(&mut features);
                            if parts.is_empty() {
                                parts.push("default".to_string());
                            }
                            parts.push(if linux { "linux" } else { "nonlinux" }.to_string());
                            if !debug_assertions {
                                parts.push("release".to_string());
                            }
                            configurations.push(Configuration {
                                label: parts.join("-"),
                                test,
                                linux,
                                debug_assertions,
                                profile: profile.clone(),
                                features,
                            });
                        }
                    }
                }
            }
        }
        if configurations.len() > MAX_CONFIGURATIONS {
            return Err(vec![format!(
                "{} configurations exceed the {MAX_CONFIGURATIONS}-configuration evaluation limit",
                configurations.len()
            )]);
        }
        let mut space = Self {
            known_features: table.keys().cloned().collect(),
            axes: axes.to_vec(),
            profiles,
            cross: cross.to_vec(),
            consumer_profiles: consumer_profiles.to_vec(),
            configurations,
            expressions: Mutex::new(HashMap::new()),
            literal_members: Vec::new(),
            profile_members: Vec::new(),
        };
        let variables = space.variables().len();
        let mut literal_members = vec![[ConfigSet::default(); 2]; variables];
        let mut profile_members = vec![ConfigSet::default(); space.profiles.len()];
        for (index, configuration) in space.configurations.iter().enumerate() {
            let (point, profile) = space.point(configuration);
            for (variable, value) in point.into_iter().enumerate() {
                literal_members[variable][usize::from(value)].insert(index);
            }
            profile_members[profile].insert(index);
        }
        space.literal_members = literal_members;
        space.profile_members = profile_members;
        Ok(space)
    }

    pub fn len(&self) -> usize {
        self.configurations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.configurations.is_empty()
    }

    pub fn all(&self) -> ConfigSet {
        let mut set = ConfigSet::default();
        for index in 0..self.len() {
            set.insert(index);
        }
        set
    }

    pub fn labels(&self, set: ConfigSet) -> Vec<&str> {
        set.indices()
            .filter(|index| *index < self.len())
            .map(|index| self.configurations[index].label.as_str())
            .collect()
    }

    fn variables(&self) -> Vec<String> {
        let mut variables = vec!["test".to_string()];
        variables.extend(self.axes.iter().map(|(_, label)| label.clone()));
        variables.extend(["linux".to_string(), "debug".to_string()]);
        variables
    }

    fn point(&self, configuration: &Configuration) -> (Vec<bool>, usize) {
        let mut point = vec![configuration.test];
        point.extend(self.axes.iter().map(|(feature, _)| configuration.features.contains(feature)));
        point.extend([configuration.linux, configuration.debug_assertions]);
        let profile = self
            .profiles
            .iter()
            .position(|profile| *profile == configuration.profile)
            .expect("configuration profile is declared");
        (point, profile)
    }

    fn cube_members(&self, cube: &Cube) -> ConfigSet {
        let mut members = cube.profile.map_or_else(|| self.all(), |p| self.profile_members[p]);
        for (variable, literal) in cube.literals.iter().enumerate() {
            if let Some(value) = literal {
                members = members.intersect(self.literal_members[variable][usize::from(*value)]);
            }
        }
        members
    }

    /// Canonical compact expression for a configuration set: `all`, `none`,
    /// or `|`-separated cubes of `&`-joined literals. A literal is a boolean
    /// variable (`test`, an axis label, `linux`, `debug`), its `!` negation,
    /// or `profile:<value>`. Cubes are chosen deterministically from the
    /// set's prime cubes, so the expression is a function of the set.
    pub fn expression(&self, set: ConfigSet) -> String {
        let set = set.intersect(self.all());
        if let Some(cached) = self.expressions.lock().expect("expression cache").get(&set) {
            return cached.clone();
        }
        let expression = self.compute_expression(set);
        self.expressions.lock().expect("expression cache").insert(set, expression.clone());
        expression
    }

    fn compute_expression(&self, set: ConfigSet) -> String {
        if set.is_empty() {
            return "none".to_string();
        }
        if set == self.all() {
            return "all".to_string();
        }
        let variables = self.variables();
        let mut cubes = Vec::new();
        for profile in std::iter::once(None).chain((0..self.profiles.len()).map(Some)) {
            for code in 0..3usize.pow(variables.len() as u32) {
                let mut remainder = code;
                let literals = (0..variables.len())
                    .map(|_| {
                        let digit = remainder % 3;
                        remainder /= 3;
                        match digit {
                            0 => None,
                            1 => Some(true),
                            _ => Some(false),
                        }
                    })
                    .collect::<Vec<_>>();
                cubes.push(Cube { literals, profile });
            }
        }
        let valid = cubes
            .into_iter()
            .filter_map(|cube| {
                let members = self.cube_members(&cube);
                (!members.is_empty() && members.is_subset(set)).then_some((cube, members))
            })
            .collect::<HashMap<_, _>>();
        let mut primes = BTreeMap::new();
        for (cube, members) in &valid {
            let generalisations = (0..cube.literals.len())
                .filter(|index| cube.literals[*index].is_some())
                .map(|index| {
                    let mut general = cube.clone();
                    general.literals[index] = None;
                    general
                })
                .chain(
                    cube.profile.map(|_| Cube { literals: cube.literals.clone(), profile: None }),
                );
            let mut prime = true;
            for general in generalisations {
                if valid.contains_key(&general) {
                    prime = false;
                    break;
                }
            }
            if prime {
                primes.insert(self.render_cube(&variables, cube), *members);
            }
        }
        let mut uncovered = set;
        let mut chosen = BTreeSet::new();
        while !uncovered.is_empty() {
            let best = primes
                .iter()
                .max_by(|(left_text, left), (right_text, right)| {
                    let left_gain = left.intersect(uncovered).len();
                    let right_gain = right.intersect(uncovered).len();
                    left_gain
                        .cmp(&right_gain)
                        .then_with(|| right_text.len().cmp(&left_text.len()))
                        .then_with(|| right_text.cmp(left_text))
                })
                .expect("a nonempty set has a covering prime cube");
            uncovered = uncovered.difference(*best.1);
            chosen.insert(best.0.clone());
        }
        chosen.into_iter().collect::<Vec<_>>().join("|")
    }

    fn render_cube(&self, variables: &[String], cube: &Cube) -> String {
        let mut literals = variables
            .iter()
            .zip(&cube.literals)
            .filter_map(|(name, literal)| match literal {
                Some(true) => Some(name.clone()),
                Some(false) => Some(format!("!{name}")),
                None => None,
            })
            .collect::<Vec<_>>();
        if let Some(profile) = cube.profile {
            literals.insert(0, format!("profile:{}", self.profiles[profile]));
        }
        if literals.is_empty() {
            "all".to_string()
        } else {
            literals.join("&")
        }
    }

    /// Parses a compact configuration expression back to its set.
    pub fn parse_expression(&self, expression: &str) -> Result<ConfigSet, String> {
        if expression == "none" {
            return Ok(ConfigSet::default());
        }
        let variables = self.variables();
        let mut set = ConfigSet::default();
        for text in expression.split('|') {
            let mut cube = Cube { literals: vec![None; variables.len()], profile: None };
            if text != "all" {
                for literal in text.split('&') {
                    if let Some(value) = literal.strip_prefix("profile:") {
                        let index =
                            self.profiles.iter().position(|profile| profile == value).ok_or_else(
                                || format!("unknown configuration profile {value:?}"),
                            )?;
                        cube.profile = Some(index);
                        continue;
                    }
                    let (value, name) =
                        literal.strip_prefix('!').map_or((true, literal), |name| (false, name));
                    let index = variables
                        .iter()
                        .position(|variable| variable == name)
                        .ok_or_else(|| format!("unknown configuration variable {name:?}"))?;
                    cube.literals[index] = Some(value);
                }
            }
            set = set.union(self.cube_members(&cube));
        }
        Ok(set)
    }

    /// Evaluates a `cfg` predicate in one configuration; `None` for an
    /// unsupported predicate or a feature the manifest does not declare.
    pub fn evaluate(&self, predicate: &str, index: usize) -> Option<bool> {
        let predicate = predicate.chars().filter(|ch| !ch.is_whitespace()).collect::<String>();
        self.evaluate_compact(&predicate, &self.configurations[index])
    }

    fn evaluate_compact(&self, predicate: &str, configuration: &Configuration) -> Option<bool> {
        if let Some(inner) =
            predicate.strip_prefix("any(").and_then(|value| value.strip_suffix(')'))
        {
            return split_cfg_arguments(inner)
                .into_iter()
                .map(|argument| self.evaluate_compact(argument, configuration))
                .try_fold(false, |active, value| value.map(|value| active || value));
        }
        if let Some(inner) =
            predicate.strip_prefix("all(").and_then(|value| value.strip_suffix(')'))
        {
            return split_cfg_arguments(inner)
                .into_iter()
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

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Cube {
    literals: Vec<Option<bool>>,
    profile: Option<usize>,
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
    fn manifest_features_and_closures_are_parsed() {
        let table = parse_features_table(MANIFEST).expect("manifest parses");
        assert_eq!(table.get("extra"), Some(&vec!["operator".to_string()]));
        assert_eq!(table.get("test-hooks"), Some(&Vec::new()));
        assert_eq!(table.len(), 5);
    }

    #[test]
    fn space_covers_axes_release_and_all_features() {
        let space = ConfigSpace::from_manifest(MANIFEST, &axes()).expect("space");
        // base: 2^3 axis combinations; one profile per non-axis feature
        // (`extra`); all-features; each x test x linux x debug.
        assert_eq!(space.len(), (8 + 1 + 1) * 8);
        let labels = space.labels(space.all());
        for label in [
            "default-linux",
            "test-hooks-tc5-operator-nonlinux-release",
            "all-features-linux",
            "test-all-features-nonlinux-release",
            "feature-extra-linux",
            "tc5-linux",
        ] {
            assert!(labels.contains(&label), "missing {label}");
        }
        let extra = (0..space.len())
            .filter(|index| space.evaluate("feature = \"extra\"", *index) == Some(true))
            .count();
        assert_eq!(extra, 16);
        assert_eq!(space.evaluate("feature = \"undeclared\"", 0), None);
    }

    #[test]
    fn consumer_profiles_and_crossed_axes_are_evaluated() {
        let space = ConfigSpace::from_manifest_with(
            MANIFEST,
            &axes(),
            &["operator".to_string()],
            &[("both".to_string(), vec!["extra".to_string(), "test-hooks".to_string()])],
        );
        assert!(space.is_err(), "an axis inside a consumer profile is rejected");
        let space = ConfigSpace::from_manifest_with(
            MANIFEST,
            &[("test-hooks".to_string(), "hooks".to_string())],
            &["test-hooks".to_string()],
            &[("product".to_string(), vec!["extra".to_string()])],
        )
        .expect("space");
        // base: 2 axis combinations; `extra`, `operator`, `tc5-benchmark`
        // single-feature profiles and the `product` profile, each x 2
        // hooks combinations; all-features; each x test x linux x debug.
        assert_eq!(space.len(), (2 + 4 * 2 + 1) * 8);
        let labels = space.labels(space.all());
        for label in ["hooks-feature-extra-linux", "profile-product-nonlinux-release"] {
            assert!(labels.contains(&label), "missing {label}");
        }
        let joint = (0..space.len())
            .filter(|index| {
                space.evaluate("all(feature = \"extra\", feature = \"test-hooks\", not(feature = \"tc5-benchmark\"))", *index)
                    == Some(true)
            })
            .count();
        assert_eq!(joint, 16, "extra and product profiles crossed with hooks");
        let error = ConfigSpace::from_manifest_with(MANIFEST, &axes(), &["extra".to_string()], &[])
            .expect_err("cross must name an axis");
        assert!(error[0].contains("configuration-cross extra is not a configuration-feature axis"));
    }

    #[test]
    fn undeclared_axes_are_rejected() {
        let error =
            ConfigSpace::from_manifest(MANIFEST, &[("missing".to_string(), "missing".to_string())])
                .expect_err("undeclared axis");
        assert!(error[0].contains("configuration feature missing is not declared"));
    }

    #[test]
    fn expressions_round_trip_and_are_canonical() {
        let space = ConfigSpace::from_manifest(MANIFEST, &axes()).expect("space");
        let mut state = 0x9e3779b97f4a7c15u64;
        for _ in 0..200 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let set = ConfigSet([
                u128::from(state) | (u128::from(state.rotate_left(17)) << 64),
                u128::from(state.rotate_left(29)),
            ])
            .intersect(space.all());
            let expression = space.expression(set);
            assert_eq!(space.parse_expression(&expression), Ok(set), "{expression}");
        }
        let test_only = (0..space.len()).filter(|index| space.configurations[*index].test).fold(
            ConfigSet::default(),
            |mut set, index| {
                set.insert(index);
                set
            },
        );
        assert_eq!(space.expression(test_only), "test");
        assert_eq!(space.expression(space.all()), "all");
        assert_eq!(space.expression(ConfigSet::default()), "none");
        assert!(space.parse_expression("bogus").is_err());
    }
}
