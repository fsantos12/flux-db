//! ORDER BY nodes.

use crate::ast::expr::Expr;
use crate::column::TypedColumn;

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    #[default]
    Asc,
    Desc,
}

/// Where NULLs go. `Default` leaves it to the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NullsOrder {
    #[default]
    Default,
    First,
    Last,
}

/// One `ORDER BY` item: `expr ASC|DESC [NULLS FIRST|LAST]`.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderBy {
    pub expr: Expr,
    pub direction: Direction,
    pub nulls: NullsOrder,
}

impl OrderBy {
    /// Ascending order on any expression.
    pub fn asc(expr: impl Into<Expr>) -> Self {
        Self { expr: expr.into(), direction: Direction::Asc, nulls: NullsOrder::Default }
    }

    /// Descending order on any expression.
    pub fn desc(expr: impl Into<Expr>) -> Self {
        Self { expr: expr.into(), direction: Direction::Desc, nulls: NullsOrder::Default }
    }

    /// Puts NULLs first.
    pub fn nulls_first(mut self) -> Self {
        self.nulls = NullsOrder::First;
        self
    }

    /// Puts NULLs last.
    pub fn nulls_last(mut self) -> Self {
        self.nulls = NullsOrder::Last;
        self
    }
}

/// `User::AGE.asc()` / `User::AGE.desc()`.
impl<E, T> TypedColumn<E, T> {
    pub fn asc(self) -> OrderBy {
        OrderBy::asc(self)
    }

    pub fn desc(self) -> OrderBy {
        OrderBy::desc(self)
    }
}