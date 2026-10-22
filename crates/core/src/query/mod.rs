//! Query builders (`select`, `insert`, `update`, `delete`), built on the [`ast`](crate::ast) nodes.
//!
//! Builders only describe a statement; rendering and execution live in other crates.

pub mod delete;
pub mod insert;
pub mod raw;
pub mod select;
pub mod state;
pub mod update;

use crate::column::Column;

/// Returns the first column name that appears more than once.
///
/// Compares by name only, so `users.name` and `name` count as the same
/// column. A linear scan beats hashing at real row widths.
pub(crate) fn duplicate_column<'a>(
    columns: impl IntoIterator<Item = &'a Column>,
) -> Option<&'static str> {
    let mut seen: Vec<&'static str> = Vec::new();
    columns.into_iter().map(Column::name).find(|name| {
        let duplicate = seen.contains(name);
        seen.push(name);
        duplicate
    })
}
