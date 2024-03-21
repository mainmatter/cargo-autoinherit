use crate::{DependencySource, SharedDependency};
use semver::{Comparator, Op, Prerelease, VersionReq};
use std::cmp::Ordering;
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
            return;
        }

        if let DependencySource::Version(version_req) = &dep.source {
            let mut swap = None;
            for (source, default_features) in self.seen.iter() {
                let DependencySource::Version(other_version_req) = source else {
                    continue;
                };
                if let Some(merged) = try_merge(version_req, other_version_req) {
                    swap = Some((
                        source.clone(),
                        merged,
                        *default_features && dep.default_features,
                    ));
                    break;
                }
            }
            if let Some((source, merged, default_features)) = swap {
                self.seen.remove(&source);
                self.seen
                    .insert(DependencySource::Version(merged), default_features);
                return;
            }
        }

        self.seen.insert(dep.source, dep.default_features);
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

/// Tries to merge two version requirements into a single version requirement.
///
/// We handle:
///
/// - The case where both version requirements are the same.
/// - The case where one version requirement is a wildcard and the other isn't.
/// - The case where both version requirements are simple carets—e.g. `^1.2` and `^1.3.1`.
///   In this case, we can merge them into `^1.3.1`.
fn try_merge(first: &VersionReq, second: &VersionReq) -> Option<VersionReq> {
    if first == second {
        return Some(first.clone());
    }

    if first == &VersionReq::STAR && second != &VersionReq::STAR {
        // First is wildcard, second isn't
        return Some(second.clone());
    }

    if first != &VersionReq::STAR && second == &VersionReq::STAR {
        // Second is wildcard, first isn't
        return Some(first.clone());
    }

    let first = as_simple_caret(first)?;
    let second = as_simple_caret(second)?;
    if first.major != second.major {
        return None;
    }
    if first.major == 0 {
        if first.minor != second.minor {
            return None;
        }
        if first.minor == Some(0) {
            return None;
        }
        let comparator = Comparator {
            op: Op::Caret,
            major: second.major,
            minor: second.minor,
            patch: first.patch.max(second.patch),
            pre: Prerelease::EMPTY,
        };
        return Some(VersionReq {
            comparators: vec![comparator],
        });
    }
    let comparator = match first.minor.cmp(&second.minor) {
        Ordering::Less => Comparator {
            op: Op::Caret,
            major: second.major,
            minor: second.minor,
            patch: second.patch,
            pre: Prerelease::EMPTY,
        },
        Ordering::Greater => Comparator {
            op: Op::Caret,
            major: first.major,
            minor: first.minor,
            patch: first.patch,
            pre: Prerelease::EMPTY,
        },
        Ordering::Equal => Comparator {
            op: Op::Caret,
            major: first.major,
            minor: first.minor,
            patch: first.patch.max(second.patch),
            pre: Prerelease::EMPTY,
        },
    };
    Some(VersionReq {
        comparators: vec![comparator],
    })
}

/// A `VersionReq` is "a simple caret" if it contains a single comparator with a `^` prefix
/// and there are no pre-release or build identifiers.
fn as_simple_caret(req: &VersionReq) -> Option<&Comparator> {
    if req.comparators.len() != 1 {
        return None;
    }
    let comp = &req.comparators[0];
    if comp.op != Op::Caret {
        return None;
    }
    if comp.pre != Prerelease::EMPTY {
        return None;
    }
    Some(comp)
}
