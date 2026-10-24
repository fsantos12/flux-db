//! `GROUP BY` / `HAVING` nodes.

use crate::ast::expr::Expr;

/// `GROUP BY exprs [HAVING condition]`.
///
/// # Examples
///
/// ```
/// use flux_db_core::ast::expr::Expr;
/// use flux_db_core::ast::group::GroupBy;
/// use flux_db_core::types::column::TypedColumn;
///
/// struct User;
/// const CITY: TypedColumn<User, String> = TypedColumn::new("users", "city");
///
/// let group = GroupBy::new([CITY])
///     .having(Expr::raw("COUNT(*) > 1", vec![]))
///     .having(Expr::raw("MAX(age) < 90", vec![]));
/// assert_eq!(group.exprs.len(), 1);
/// // Repeated `having` calls are AND-ed together.
/// assert!(matches!(group.having, Some(Expr::And(ref parts)) if parts.len() == 2));
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GroupBy {
    /// Grouping expressions, in order.
    pub exprs: Vec<Expr>,
    /// Optional `HAVING` condition.
    pub having: Option<Expr>,
}

impl GroupBy {
    /// Groups by the given expressions (columns, usually).
    pub fn new<I: Into<Expr>>(exprs: impl IntoIterator<Item = I>) -> Self {
        Self {
            exprs: exprs.into_iter().map(Into::into).collect(),
            having: None,
        }
    }

    /// Adds a `HAVING` condition; repeated calls are AND-ed.
    pub fn having(mut self, condition: Expr) -> Self {
        self.having = Some(match self.having.take() {
            Some(existing) => existing.and(condition),
            None => condition,
        });
        self
    }

    /// `true` if there is nothing to group by (renderer skips the clause).
    ///
    /// Only `exprs` is checked: a `HAVING` condition without grouping
    /// expressions still counts as empty, so the renderer must decide
    /// whether to reject it or render it on its own.
    pub fn is_empty(&self) -> bool {
        self.exprs.is_empty()
    }
}
