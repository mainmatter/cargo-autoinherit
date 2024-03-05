use crate::{DependencySource, SharedDependency};
use std::collections::HashMap;

/// For a given package, this struct keeps track of the versions that have been seen.
/// It actively tries to minimize the number of versions that are kept.
///
/// In particular:
///
/// - If the same version requirement appears more than once, only one instance is kept.
/// - If different version requirements appear, all instances are kept.
/// - If the same version requirement appears more than once, with default features enabled in one
///   case and disabled in another, only the disabled instance is kept.
#[derive(Default)]
pub(crate) struct MinimalVersionSet {
    seen: HashMap<DependencySource, bool>,
}

impl MinimalVersionSet {
    pub(crate) fn insert(&mut self, dep: SharedDependency) {
        if let Some(default_features) = self.seen.get_mut(&dep.source) {
            *default_features &= dep.default_features;
        } else {
            self.seen.insert(dep.source, dep.default_features);
        }
    }

    pub(crate) fn into_iter(self) -> impl Iterator<Item = SharedDependency> {
        self.seen
            .into_iter()
            .map(|(source, default_features)| SharedDependency {
                default_features,
                source,
            })
    }

    pub(crate) fn len(&self) -> usize {
        self.seen.len()
    }
}
