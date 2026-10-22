//! Typestate markers shared by `UPDATE` and `DELETE`.
/// No filter and no `.all()` yet; not executable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unfiltered;
/// Has at least one filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Filtered;
/// Explicitly targets every row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct All;