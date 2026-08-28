//! Package-root path validation.

use crate::error::{HostError, HostResult};
use std::path::{Component, Path, PathBuf};

/// Resolves one portable relative resource and proves canonical containment.
pub fn resolve_resource(root: &Path, resource: &str, extension: &str) -> HostResult<PathBuf> {
    if resource.is_empty()
        || resource.contains('\0')
        || resource.contains('\\')
        || resource.starts_with('/')
        || resource.ends_with('/')
        || resource.contains("://")
    {
        return Err(HostError::new(
            "path-escape",
            "resource path is not portable",
        ));
    }
    let relative = Path::new(resource);
    if relative.extension().and_then(|value| value.to_str()) != Some(extension)
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(HostError::new("path-escape", "resource path is unsafe"));
    }

    let canonical_root = root
        .canonicalize()
        .map_err(|error| HostError::new("path-escape", error.to_string()))?;
    let candidate = canonical_root.join(relative);
    let canonical_candidate = candidate
        .canonicalize()
        .map_err(|error| HostError::new("path-escape", error.to_string()))?;
    if !canonical_candidate.starts_with(&canonical_root) || !canonical_candidate.is_file() {
        return Err(HostError::new(
            "path-escape",
            "resource is outside the package root or is not a regular file",
        ));
    }
    Ok(canonical_candidate)
}
