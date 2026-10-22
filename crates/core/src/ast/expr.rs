//! Expression tree for filters and projections.
//!
//! Core only describes *what* was asked; `flux-db-sql-render` (and future
//! non-SQL backends) decide how to express it.

use crate::column::{Column, TypedColumn};
use crate::value::{IntoValue, Value};

/// Binary comparison operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// A LIKE pattern. The dialect escapes the text, except for [`LikePattern::Raw`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum LikePattern {
    /// Matches values containing the text literally.
    Contains(String),
    /// Matches values starting with the text literally.
    StartsWith(String),
    /// Matches values ending with the text literally.
    EndsWith(String),
    /// Raw pattern: `%` and `_` are wildcards, sent as-is.
    Raw(String),
}

/// An owned expression node. Holds values, never SQL (except [`Expr::Raw`]).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Expr {
    Column(Column),
    Value(Value),
    Binary { left: Box<Expr>, op: BinaryOp, right: Box<Expr> },
    Not(Box<Expr>),
    And(Vec<Expr>),
    Or(Vec<Expr>),
    IsNull(Box<Expr>),
    IsNotNull(Box<Expr>),
    In { expr: Box<Expr>, list: Vec<Expr> },
    NotIn { expr: Box<Expr>, list: Vec<Expr> },
    Like { expr: Box<Expr>, pattern: LikePattern },
    NotLike { expr: Box<Expr>, pattern: LikePattern },
    Between { expr: Box<Expr>, low: Box<Expr>, high: Box<Expr> },
    NotBetween { expr: Box<Expr>, low: Box<Expr>, high: Box<Expr> },
    /// Raw SQL fragment; params are always bound, never inlined.
    Raw { sql: String, params: Vec<Value> },
}

impl Expr {
    /// Wraps a Rust value as a bound-parameter expression.
    pub fn value(value: impl IntoValue) -> Self {
        Self::Value(value.into_value())
    }

    /// Wraps a column (typed or untyped) as an expression.
    pub fn col(column: impl Into<Expr>) -> Self {
        column.into()
    }

    /// Raw SQL escape hatch with bound params.
    pub fn raw(sql: impl Into<String>, params: Vec<Value>) -> Self {
        Self::Raw { sql: sql.into(), params }
    }

    // --- Building from a list ---

    /// `a AND b AND ...`. An empty list yields an empty `And` (always-true).
    pub fn all(exprs: impl IntoIterator<Item = Expr>) -> Self {
        Self::And(exprs.into_iter().collect())
    }

    /// `a OR b OR ...`. An empty list yields an empty `Or` (always-false).
    pub fn any(exprs: impl IntoIterator<Item = Expr>) -> Self {
        Self::Or(exprs.into_iter().collect())
    }

    /// Always-true condition (empty `And`).
    pub fn always_true() -> Self {
        Self::And(Vec::new())
    }

    /// Always-false condition (empty `Or`).
    pub fn always_false() -> Self {
        Self::Or(Vec::new())
    }

    /// `self AND other`, flattening nested `And`s.
    pub fn and(self, other: Expr) -> Self {
        match self {
            Self::And(mut list) => {
                list.push(other);
                Self::And(list)
            }
            first => Self::And(vec![first, other]),
        }
    }

    /// `self OR other`, flattening nested `Or`s.
    pub fn or(self, other: Expr) -> Self {
        match self {
            Self::Or(mut list) => {
                list.push(other);
                Self::Or(list)
            }
            first => Self::Or(vec![first, other]),
        }
    }

    // --- Column-free comparisons (any expression on both sides) ---

    /// `self = other`.
    pub fn equals(self, other: impl Into<Expr>) -> Self {
        self.binary(BinaryOp::Eq, other)
    }

    /// `self <> other`.
    pub fn not_equals(self, other: impl Into<Expr>) -> Self {
        self.binary(BinaryOp::Ne, other)
    }

    /// `self < other`.
    pub fn less_than(self, other: impl Into<Expr>) -> Self {
        self.binary(BinaryOp::Lt, other)
    }

    /// `self <= other`.
    pub fn less_or_equal(self, other: impl Into<Expr>) -> Self {
        self.binary(BinaryOp::Le, other)
    }

    /// `self > other`.
    pub fn greater_than(self, other: impl Into<Expr>) -> Self {
        self.binary(BinaryOp::Gt, other)
    }

    /// `self >= other`.
    pub fn greater_or_equal(self, other: impl Into<Expr>) -> Self {
        self.binary(BinaryOp::Ge, other)
    }

    /// `self IS NULL`.
    pub fn is_null(self) -> Self {
        Self::IsNull(Box::new(self))
    }

    /// `self IS NOT NULL`.
    pub fn is_not_null(self) -> Self {
        Self::IsNotNull(Box::new(self))
    }

    /// `self IN (...)`.
    pub fn in_list<I: Into<Expr>>(self, list: impl IntoIterator<Item = I>) -> Self {
        Self::In { expr: Box::new(self), list: list.into_iter().map(Into::into).collect() }
    }

    /// `self NOT IN (...)`.
    pub fn not_in_list<I: Into<Expr>>(self, list: impl IntoIterator<Item = I>) -> Self {
        Self::NotIn { expr: Box::new(self), list: list.into_iter().map(Into::into).collect() }
    }

    /// `self BETWEEN low AND high`.
    pub fn between(self, low: impl Into<Expr>, high: impl Into<Expr>) -> Self {
        Self::Between {
            expr: Box::new(self),
            low: Box::new(low.into()),
            high: Box::new(high.into()),
        }
    }

    /// `self NOT BETWEEN low AND high`.
    pub fn not_between(self, low: impl Into<Expr>, high: impl Into<Expr>) -> Self {
        Self::NotBetween {
            expr: Box::new(self),
            low: Box::new(low.into()),
            high: Box::new(high.into()),
        }
    }

    fn binary(self, op: BinaryOp, other: impl Into<Expr>) -> Self {
        Self::Binary { left: Box::new(self), op, right: Box::new(other.into()) }
    }
}

// --- Operator traits: `!a`, `a & b`, `a | b` ---

impl std::ops::Not for Expr {
    type Output = Expr;

    fn not(self) -> Expr {
        Expr::Not(Box::new(self))
    }
}

impl std::ops::BitAnd for Expr {
    type Output = Expr;

    fn bitand(self, rhs: Expr) -> Expr {
        self.and(rhs)
    }
}

impl std::ops::BitOr for Expr {
    type Output = Expr;

    fn bitor(self, rhs: Expr) -> Expr {
        self.or(rhs)
    }
}

impl From<Value> for Expr {
    fn from(value: Value) -> Self {
        Self::Value(value)
    }
}

impl From<Column> for Expr {
    fn from(column: Column) -> Self {
        Self::Column(column)
    }
}

impl<E, T> From<TypedColumn<E, T>> for Expr {
    fn from(column: TypedColumn<E, T>) -> Self {
        Self::Column(column.column())
    }
}

// --- Typed column helpers ---

/// Helpers available on every typed column, regardless of `T`.
impl<E, T> TypedColumn<E, T> {
    fn boxed(self) -> Box<Expr> {
        Box::new(Expr::Column(self.column()))
    }
}

/// Typed comparisons: values must match the column's Rust type `T`.
impl<E, T: IntoValue> TypedColumn<E, T> {
    /// `col = value`.
    pub fn eq(self, value: T) -> Expr {
        self.binary(BinaryOp::Eq, value)
    }

    /// `col <> value`.
    pub fn ne(self, value: T) -> Expr {
        self.binary(BinaryOp::Ne, value)
    }

    /// `col < value`.
    pub fn lt(self, value: T) -> Expr {
        self.binary(BinaryOp::Lt, value)
    }

    /// `col <= value`.
    pub fn le(self, value: T) -> Expr {
        self.binary(BinaryOp::Le, value)
    }

    /// `col > value`.
    pub fn gt(self, value: T) -> Expr {
        self.binary(BinaryOp::Gt, value)
    }

    /// `col >= value`.
    pub fn ge(self, value: T) -> Expr {
        self.binary(BinaryOp::Ge, value)
    }

    /// `col IN (...)`. An empty list is rendered as always-false.
    pub fn is_in(self, values: impl IntoIterator<Item = T>) -> Expr {
        Expr::In { expr: self.boxed(), list: Self::values(values) }
    }

    /// `col NOT IN (...)`. An empty list is rendered as always-true.
    pub fn not_in(self, values: impl IntoIterator<Item = T>) -> Expr {
        Expr::NotIn { expr: self.boxed(), list: Self::values(values) }
    }

    /// `col BETWEEN low AND high` (inclusive on both ends).
    pub fn between(self, low: T, high: T) -> Expr {
        Expr::Between {
            expr: self.boxed(),
            low: Box::new(Expr::value(low)),
            high: Box::new(Expr::value(high)),
        }
    }

    /// `col NOT BETWEEN low AND high`.
    pub fn not_between(self, low: T, high: T) -> Expr {
        Expr::NotBetween {
            expr: self.boxed(),
            low: Box::new(Expr::value(low)),
            high: Box::new(Expr::value(high)),
        }
    }

    fn binary(self, op: BinaryOp, value: T) -> Expr {
        Expr::Binary { left: self.boxed(), op, right: Box::new(Expr::value(value)) }
    }

    fn values(values: impl IntoIterator<Item = T>) -> Vec<Expr> {
        values.into_iter().map(Expr::value).collect()
    }
}

/// `IS NULL` only exists on nullable columns.
impl<E, T> TypedColumn<E, Option<T>> {
    /// `col IS NULL`.
    pub fn is_null(self) -> Expr {
        Expr::IsNull(self.boxed())
    }

    /// `col IS NOT NULL`.
    pub fn is_not_null(self) -> Expr {
        Expr::IsNotNull(self.boxed())
    }
}

/// LIKE helpers only exist on text columns.
impl<E> TypedColumn<E, String> {
    /// Matches values containing `text` literally.
    pub fn contains(self, text: impl Into<String>) -> Expr {
        self.like_pattern(LikePattern::Contains(text.into()))
    }

    /// Matches values starting with `text` literally.
    pub fn starts_with(self, text: impl Into<String>) -> Expr {
        self.like_pattern(LikePattern::StartsWith(text.into()))
    }

    /// Matches values ending with `text` literally.
    pub fn ends_with(self, text: impl Into<String>) -> Expr {
        self.like_pattern(LikePattern::EndsWith(text.into()))
    }

    /// Raw pattern: `%` and `_` are wildcards, taken as-is.
    pub fn like(self, pattern: impl Into<String>) -> Expr {
        self.like_pattern(LikePattern::Raw(pattern.into()))
    }

    /// `NOT LIKE` with a raw pattern.
    pub fn not_like(self, pattern: impl Into<String>) -> Expr {
        Expr::NotLike { expr: self.boxed(), pattern: LikePattern::Raw(pattern.into()) }
    }

    fn like_pattern(self, pattern: LikePattern) -> Expr {
        Expr::Like { expr: self.boxed(), pattern }
    }
}

