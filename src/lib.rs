use crate::dedup::MinimalVersionSet;
use anyhow::Context;
use cargo_manifest::{Dependency, DependencyDetail, DepsSet, Manifest};
use guppy::VersionReq;
use std::collections::BTreeMap;
use std::fmt::Formatter;
use toml_edit::{Array, Key};

mod dedup;

pub fn auto_inherit() -> Result<(), anyhow::Error> {
    let metadata = guppy::MetadataCommand::new().exec().context(
        "Failed to execute `cargo metadata`. Was the command invoked inside a Rust project?",
    )?;
    let graph = metadata
        .build_graph()
        .context("Failed to build package graph")?;
    let workspace_root = graph.workspace().root();
    let mut root_manifest: Manifest = {
        let contents = fs_err::read_to_string(workspace_root.join("Cargo.toml").as_std_path())
            .context("Failed to read root manifest")?;
        toml::from_str(&contents).context("Failed to parse root manifest")?
    };
    let Some(workspace) = &mut root_manifest.workspace else {
        anyhow::bail!(
            "`cargo autoinherit` can only be run in a workspace. \
            The root manifest ({}) does not have a `workspace` field.",
            workspace_root
        )
    };

    let mut package_name2specs: BTreeMap<String, Action> = BTreeMap::new();
    if let Some(deps) = &workspace.dependencies {
        process_deps(deps, &mut package_name2specs);
    }

    for member_id in graph.workspace().member_ids() {
        let package = graph.metadata(member_id)?;
        assert!(package.in_workspace());
        let manifest: Manifest = {
            let contents = fs_err::read_to_string(package.manifest_path().as_std_path())
                .context("Failed to read root manifest")?;
            toml::from_str(&contents).context("Failed to parse root manifest")?
        };
        if let Some(deps) = &manifest.dependencies {
            process_deps(deps, &mut package_name2specs);
        }
        if let Some(deps) = &manifest.dev_dependencies {
            process_deps(deps, &mut package_name2specs);
        }
        if let Some(deps) = &manifest.build_dependencies {
            process_deps(deps, &mut package_name2specs);
        }
    }

    let mut package_name2inherited_source: BTreeMap<String, SharedDependency> = BTreeMap::new();
    'outer: for (package_name, action) in package_name2specs {
        let Action::TryInherit(specs) = action else {
            eprintln!("`{package_name}` won't be auto-inherited because it appears at least once from a source type \
                that we currently don't support (e.g. private registry, path dependency).");
            continue;
        };
        if specs.len() > 1 {
            eprintln!("`{package_name}` won't be auto-inherited because there are multiple sources for it:");
            for spec in specs.into_iter() {
                eprintln!("  - {}", spec.source);
            }
            continue 'outer;
        }

        let spec = specs.into_iter().next().unwrap();
        package_name2inherited_source.insert(package_name, spec);
    }

    // Add new "shared" dependencies to `[workspace.dependencies]`
    let mut workspace_toml: toml_edit::DocumentMut = {
        let contents = fs_err::read_to_string(workspace_root.join("Cargo.toml").as_std_path())
            .context("Failed to read root manifest")?;
        contents.parse().context("Failed to parse root manifest")?
    };
    let workspace_table = workspace_toml.as_table_mut()["workspace"]
        .as_table_mut()
        .expect(
            "Failed to find `[workspace]` table in root manifest. \
        This is a bug in `cargo_autoinherit`.",
        );
    let workspace_deps = workspace_table
        .entry("dependencies")
        .or_insert(toml_edit::Item::Table(toml_edit::Table::new()))
        .as_table_mut()
        .expect("Failed to find `[workspace.dependencies]` table in root manifest.");
    let mut was_modified = false;
    for (package_name, source) in &package_name2inherited_source {
        if workspace_deps.get(package_name).is_some() {
            continue;
        } else {
            insert_preserving_decor(
                workspace_deps,
                package_name,
                dep2toml_item(&shared2dep(source)),
            );
            was_modified = true;
        }
    }
    if was_modified {
        fs_err::write(
            workspace_root.join("Cargo.toml").as_std_path(),
            workspace_toml.to_string(),
        )
        .context("Failed to write manifest")?;
    }

    // Inherit new "shared" dependencies in each member's manifest
    for member_id in graph.workspace().member_ids() {
        let package = graph.metadata(member_id)?;
        let manifest_contents = fs_err::read_to_string(package.manifest_path().as_std_path())
            .context("Failed to read root manifest")?;
        let manifest: Manifest =
            toml::from_str(&manifest_contents).context("Failed to parse root manifest")?;
        let mut manifest_toml: toml_edit::DocumentMut = manifest_contents
            .parse()
            .context("Failed to parse root manifest")?;
        let mut was_modified = false;
        if let Some(deps) = &manifest.dependencies {
            let deps_toml = manifest_toml["dependencies"]
                .as_table_mut()
                .expect("Failed to find `[dependencies]` table in root manifest.");
            inherit_deps(
                deps,
                deps_toml,
                &package_name2inherited_source,
                &mut was_modified,
            );
        }
        if let Some(deps) = &manifest.dev_dependencies {
            let deps_toml = manifest_toml["dev-dependencies"]
                .as_table_mut()
                .expect("Failed to find `[dev-dependencies]` table in root manifest.");
            inherit_deps(
                deps,
                deps_toml,
                &package_name2inherited_source,
                &mut was_modified,
            );
        }
        if let Some(deps) = &manifest.build_dependencies {
            let deps_toml = manifest_toml["build-dependencies"]
                .as_table_mut()
                .expect("Failed to find `[build-dependencies]` table in root manifest.");
            inherit_deps(
                deps,
                deps_toml,
                &package_name2inherited_source,
                &mut was_modified,
            );
        }
        if was_modified {
            fs_err::write(
                package.manifest_path().as_std_path(),
                manifest_toml.to_string(),
            )
            .context("Failed to write manifest")?;
        }
    }

    Ok(())
}

enum Action {
    TryInherit(MinimalVersionSet),
    Skip,
}

impl Default for Action {
    fn default() -> Self {
        Action::TryInherit(MinimalVersionSet::default())
    }
}

fn inherit_deps(
    deps: &DepsSet,
    toml_deps: &mut toml_edit::Table,
    package_name2spec: &BTreeMap<String, SharedDependency>,
    was_modified: &mut bool,
) {
    for (name, dep) in deps {
        let package_name = dep.package().unwrap_or(name.as_str());
        if !package_name2spec.contains_key(package_name) {
            continue;
        }
        match dep {
            Dependency::Simple(_) => {
                let mut inherited = toml_edit::InlineTable::new();
                inherited.insert("workspace", toml_edit::value(true).into_value().unwrap());

                insert_preserving_decor(toml_deps, name, toml_edit::Item::Value(inherited.into()));
                *was_modified = true;
            }
            Dependency::Inherited(_) => {
                // Nothing to do.
            }
            Dependency::Detailed(details) => {
                let mut inherited = toml_edit::InlineTable::new();
                inherited.insert("workspace", toml_edit::value(true).into_value().unwrap());
                if let Some(features) = &details.features {
                    inherited.insert(
                        "features",
                        toml_edit::Value::Array(Array::from_iter(features.iter())),
                    );
                }
                if let Some(optional) = details.optional {
                    inherited.insert("optional", toml_edit::value(optional).into_value().unwrap());
                }

                insert_preserving_decor(toml_deps, name, toml_edit::Item::Value(inherited.into()));
                *was_modified = true;
            }
        }
    }
}

fn insert_preserving_decor(table: &mut toml_edit::Table, key: &str, mut value: toml_edit::Item) {
    fn get_decor(item: &toml_edit::Item) -> Option<toml_edit::Decor> {
        match item {
            toml_edit::Item::Value(v) => Some(v.decor().clone()),
            toml_edit::Item::Table(t) => Some(t.decor().clone()),
            _ => None,
        }
    }

    fn set_decor(item: &mut toml_edit::Item, decor: toml_edit::Decor) {
        match item {
            toml_edit::Item::Value(v) => {
                *v.decor_mut() = decor;
            }
            toml_edit::Item::Table(t) => {
                *t.decor_mut() = decor;
            }
            _ => unreachable!(),
        }
    }

    let mut new_key = Key::new(key);
    if let Some((existing_key, existing_value)) = table.get_key_value(key) {
        new_key = new_key.with_leaf_decor(existing_key.leaf_decor().to_owned());
        if let Some(decor) = get_decor(existing_value) {
            set_decor(&mut value, decor);
        }
    }
    table.insert_formatted(&new_key, value);
}

fn process_deps(deps: &DepsSet, package_name2specs: &mut BTreeMap<String, Action>) {
    for (name, details) in deps {
        match dep2shared_dep(details) {
            SourceType::Shareable(source) => {
                let action = package_name2specs.entry(name.clone()).or_default();
                if let Action::TryInherit(set) = action {
                    set.insert(source);
                }
            }
            SourceType::Inherited => {}
            SourceType::MustBeSkipped => {
                package_name2specs.insert(name.clone(), Action::Skip);
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct SharedDependency {
    default_features: bool,
    source: DependencySource,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum DependencySource {
    Version(VersionReq),
    Git {
        git: String,
        branch: Option<String>,
        tag: Option<String>,
        rev: Option<String>,
    },
}

impl std::fmt::Display for DependencySource {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DependencySource::Version(version) => write!(f, "version: {}", version),
            DependencySource::Git {
                git,
                branch,
                tag,
                rev,
            } => {
                write!(f, "git: {}", git)?;
                if let Some(branch) = branch {
                    write!(f, ", branch: {}", branch)?;
                }
                if let Some(tag) = tag {
                    write!(f, ", tag: {}", tag)?;
                }
                if let Some(rev) = rev {
                    write!(f, ", rev: {}", rev)?;
                }
                Ok(())
            }
        }
    }
}

enum SourceType {
    Shareable(SharedDependency),
    Inherited,
    MustBeSkipped,
}

fn dep2shared_dep(dep: &Dependency) -> SourceType {
    match dep {
        Dependency::Simple(version) => {
            let version_req =
                VersionReq::parse(version).expect("Failed to parse version requirement");
            SourceType::Shareable(SharedDependency {
                default_features: true,
                source: DependencySource::Version(version_req),
            })
        }
        Dependency::Inherited(_) => SourceType::Inherited,
        Dependency::Detailed(d) => {
            let mut source = None;
            // We ignore custom registries for now.
            if d.registry.is_some() || d.registry_index.is_some() {
                return SourceType::MustBeSkipped;
            }
            // We ignore path deps for now.
            if d.path.is_some() {
                return SourceType::MustBeSkipped;
            }
            if let Some(git) = &d.git {
                source = Some(DependencySource::Git {
                    git: git.to_owned(),
                    branch: d.branch.to_owned(),
                    tag: d.tag.to_owned(),
                    rev: d.rev.to_owned(),
                });
            } else if let Some(version) = &d.version {
                let version_req =
                    VersionReq::parse(version).expect("Failed to parse version requirement");
                source = Some(DependencySource::Version(version_req));
            }
            match source {
                None => SourceType::MustBeSkipped,
                Some(source) => SourceType::Shareable(SharedDependency {
                    default_features: d.default_features.unwrap_or(true),
                    source,
                }),
            }
        }
    }
}

fn shared2dep(shared_dependency: &SharedDependency) -> Dependency {
    let SharedDependency {
        default_features,
        source,
    } = shared_dependency;
    match source {
        DependencySource::Version(version) => {
            if *default_features {
                Dependency::Simple(version.to_string())
            } else {
                Dependency::Detailed(DependencyDetail {
                    version: Some(version.to_string()),
                    default_features: Some(false),
                    ..DependencyDetail::default()
                })
            }
        }
        DependencySource::Git {
            git,
            branch,
            tag,
            rev,
        } => Dependency::Detailed(DependencyDetail {
            package: None,
            version: None,
            registry: None,
            registry_index: None,
            path: None,
            git: Some(git.clone()),
            branch: branch.clone(),
            tag: tag.clone(),
            rev: rev.clone(),
            features: None,
            optional: None,
            default_features: if *default_features { None } else { Some(false) },
        }),
    }
}

fn dep2toml_item(dependency: &Dependency) -> toml_edit::Item {
    match dependency {
        Dependency::Simple(version) => toml_edit::value(version.trim_start_matches('^').to_owned()),
        Dependency::Inherited(inherited) => {
            let mut table = toml_edit::InlineTable::new();
            table.get_or_insert("workspace", true);
            if let Some(features) = &inherited.features {
                table.get_or_insert("features", Array::from_iter(features.iter()));
            }
            if let Some(optional) = inherited.optional {
                table.get_or_insert("optional", optional);
            }
            toml_edit::Item::Value(toml_edit::Value::InlineTable(table))
        }
        Dependency::Detailed(details) => {
            let mut table = toml_edit::InlineTable::new();
            let DependencyDetail {
                version,
                registry,
                registry_index,
                path,
                git,
                branch,
                tag,
                rev,
                features,
                optional,
                default_features,
                package,
            } = details;

            if let Some(version) = version {
                table.get_or_insert("version", version.trim_start_matches('^'));
            }
            if let Some(registry) = registry {
                table.get_or_insert("registry", registry);
            }
            if let Some(registry_index) = registry_index {
                table.get_or_insert("registry-index", registry_index);
            }
            if let Some(path) = path {
                table.get_or_insert("path", path);
            }
            if let Some(git) = git {
                table.get_or_insert("git", git);
            }
            if let Some(branch) = branch {
                table.get_or_insert("branch", branch);
            }
            if let Some(tag) = tag {
                table.get_or_insert("tag", tag);
            }
            if let Some(rev) = rev {
                table.get_or_insert("rev", rev);
            }
            if let Some(features) = features {
                table.get_or_insert("features", Array::from_iter(features.iter()));
            }
            if let Some(optional) = optional {
                table.get_or_insert(
                    "optional",
                    toml_edit::value(*optional).into_value().unwrap(),
                );
            }
            if let Some(default_features) = default_features {
                table.get_or_insert(
                    "default-features",
                    toml_edit::value(*default_features).into_value().unwrap(),
                );
            }
            if let Some(package) = package {
                table.get_or_insert("package", package);
            }

            toml_edit::Item::Value(toml_edit::Value::InlineTable(table))
        }
    }
}
