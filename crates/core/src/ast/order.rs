//! ORDER BY nodes.
//!
//! # Examples
//!
//! ```
//! use flux_db_core::ast::order::{Direction, NullsOrder};
//! use flux_db_core::column::TypedColumn;
//!
//! struct User;
//! const AGE: TypedColumn<User, Option<i32>> = TypedColumn::new("users", "age");
//!
//! // ORDER BY users.age DESC NULLS LAST
//! let order = AGE.desc().nulls_last();
//! assert_eq!(order.direction, Direction::Desc);
//! assert_eq!(order.nulls, NullsOrder::Last);
//! ```

use crate::ast::expr::Expr;
use crate::column::TypedColumn;

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// Ascending (`ASC`), the default.
    #[default]
    Asc,
    /// Descending (`DESC`).
    Desc,
}

/// Where NULLs go. `Default` leaves it to the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NullsOrder {
    /// No `NULLS` clause; the database decides.
    #[default]
    Default,
    /// `NULLS FIRST`.
    First,
    /// `NULLS LAST`.
    Last,
}

/// One `ORDER BY` item: `expr ASC|DESC [NULLS FIRST|LAST]`.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderBy {
    /// Expression to sort by.
    pub expr: Expr,
    /// Sort direction.
    pub direction: Direction,
    /// Placement of NULLs.
    pub nulls: NullsOrder,
}

impl OrderBy {
    /// Orders by `expr` in `direction`, leaving NULL placement to the database.
    pub fn new(expr: impl Into<Expr>, direction: Direction) -> Self {
        Self {
            expr: expr.into(),
            direction,
            nulls: NullsOrder::Default,
        }
    }

    /// Ascending order on any expression.
    pub fn asc(expr: impl Into<Expr>) -> Self {
        Self::new(expr, Direction::Asc)
    }

    /// Descending order on any expression.
    pub fn desc(expr: impl Into<Expr>) -> Self {
        Self::new(expr, Direction::Desc)
    }

    /// Puts NULLs first.
    pub fn nulls_first(self) -> Self {
        self.with_nulls(NullsOrder::First)
    }

    /// Puts NULLs last.
    pub fn nulls_last(self) -> Self {
        self.with_nulls(NullsOrder::Last)
    }

    /// Sets the NULL placement.
    fn with_nulls(mut self, nulls: NullsOrder) -> Self {
        self.nulls = nulls;
        self
    }
}

/// Sort shortcuts on typed columns: `User::AGE.asc()` / `User::AGE.desc()`.
impl<E, T> TypedColumn<E, T> {
    /// Ascending order on this column.
    pub fn asc(self) -> OrderBy {
        OrderBy::asc(self)
    }

    /// Descending order on this column.
    pub fn desc(self) -> OrderBy {
        OrderBy::desc(self)
    }
}
