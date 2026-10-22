//! GROUP BY / HAVING nodes.

use crate::ast::expr::Expr;

/// `GROUP BY exprs [HAVING condition]`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GroupBy {
    pub exprs: Vec<Expr>,
    pub having: Option<Expr>,
}

impl GroupBy {
    /// Groups by the given expressions (columns, usually).
    pub fn new<I: Into<Expr>>(exprs: impl IntoIterator<Item = I>) -> Self {
        Self { exprs: exprs.into_iter().map(Into::into).collect(), having: None }
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
    pub fn is_empty(&self) -> bool {
        self.exprs.is_empty()
    }
}