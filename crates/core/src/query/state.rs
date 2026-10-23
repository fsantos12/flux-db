//! Typestate markers shared by the write builders.
//!
//! A builder starts [`Unfiltered`] and becomes executable only once it is
//! [`Filtered`] or explicitly targets [`All`] rows, so an accidental
//! unfiltered write cannot compile.

/// No filter and no `.all()` yet; not executable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unfiltered;

/// Has at least one filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Filtered;

/// Explicitly targets every row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct All;

/// No `RETURNING` yet: the write runs with `execute` only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoReturning;

/// Has `RETURNING`: the fetch shortcuts are available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Returning;

/// States in which an `UPDATE` or `DELETE` may be executed: [`Filtered`] and [`All`].
///
/// Sealed: it cannot be implemented outside this crate.
pub trait Executable: sealed::Sealed {}

impl Executable for Filtered {}
impl Executable for All {}

mod sealed {
    pub trait Sealed {}

    impl Sealed for super::Filtered {}
    impl Sealed for super::All {}
}
