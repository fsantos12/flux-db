//! Expression tree for filters and projections.
//!
//! Core only describes *what* was asked; `flux-db-sql-render` (and future
//! non-SQL backends) decide how to express it.
//!
//! Expressions are usually built from typed columns, then combined with
//! [`Expr::and`] / [`Expr::or`] or the `&`, `|` and `!` operators.
//!
//! # Examples
//!
//! ```
//! use flux_db_core::ast::expr::Expr;
//! use flux_db_core::column::TypedColumn;
//!
//! struct User;
//! const AGE: TypedColumn<User, i32> = TypedColumn::new("users", "age");
//! const NAME: TypedColumn<User, String> = TypedColumn::new("users", "name");
//!
//! // users.age >= 18 AND users.name LIKE '%ann%'
//! let filter = AGE.ge(18) & NAME.contains("ann");
//! assert!(matches!(filter, Expr::And(ref parts) if parts.len() == 2));
//! ```

use crate::column::{Column, TypedColumn};
use crate::value::{IntoValue, Value};

/// Binary comparison operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    /// `=`
    Eq,
    /// `<>`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
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
    /// A column reference.
    Column(Column),
    /// A bound parameter value.
    Value(Value),
    /// `left op right`.
    Binary {
        /// Left-hand operand.
        left: Box<Expr>,
        /// Comparison operator.
        op: BinaryOp,
        /// Right-hand operand.
        right: Box<Expr>,
    },
    /// `NOT expr`.
    Not(Box<Expr>),
    /// `a AND b AND ...`; empty means always-true.
    And(Vec<Expr>),
    /// `a OR b OR ...`; empty means always-false.
    Or(Vec<Expr>),
    /// `expr IS NULL`.
    IsNull(Box<Expr>),
    /// `expr IS NOT NULL`.
    IsNotNull(Box<Expr>),
    /// `expr IN (list)`; an empty list means always-false.
    In {
        /// Tested expression.
        expr: Box<Expr>,
        /// Candidate values, in order.
        list: Vec<Expr>,
    },
    /// `expr NOT IN (list)`; an empty list means always-true.
    NotIn {
        /// Tested expression.
        expr: Box<Expr>,
        /// Excluded values, in order.
        list: Vec<Expr>,
    },
    /// `expr LIKE pattern`.
    Like {
        /// Tested expression.
        expr: Box<Expr>,
        /// Pattern to match.
        pattern: LikePattern,
    },
    /// `expr NOT LIKE pattern`.
    NotLike {
        /// Tested expression.
        expr: Box<Expr>,
        /// Pattern that must not match.
        pattern: LikePattern,
    },
    /// `expr BETWEEN low AND high`, inclusive; bounds are kept as written.
    Between {
        /// Tested expression.
        expr: Box<Expr>,
        /// Lower bound.
        low: Box<Expr>,
        /// Upper bound.
        high: Box<Expr>,
    },
    /// `expr NOT BETWEEN low AND high`; bounds are kept as written.
    NotBetween {
        /// Tested expression.
        expr: Box<Expr>,
        /// Lower bound.
        low: Box<Expr>,
        /// Upper bound.
        high: Box<Expr>,
    },
    /// Raw SQL fragment; params are always bound, never inlined.
    Raw {
        /// SQL text, sent as-is.
        sql: String,
        /// Parameters bound to the fragment's placeholders.
        params: Vec<Value>,
    },
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
    ///
    /// The SQL text is not checked or escaped; only `params` are safe for
    /// untrusted input.
    ///
    /// # Examples
    ///
    /// ```
    /// use flux_db_core::ast::expr::Expr;
    /// use flux_db_core::value::Value;
    ///
    /// let expr = Expr::raw("age > ?", vec![Value::I32(18)]);
    /// assert!(matches!(expr, Expr::Raw { ref params, .. } if params.len() == 1));
    /// ```
    pub fn raw(sql: impl Into<String>, params: Vec<Value>) -> Self {
        Self::Raw {
            sql: sql.into(),
            params,
        }
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
    ///
    /// Only a left-hand `And` is extended; an `Or` is never merged into it.
    ///
    /// # Examples
    ///
    /// ```
    /// use flux_db_core::ast::expr::Expr;
    ///
    /// let expr = Expr::value(1).and(Expr::value(2)).and(Expr::value(3));
    /// assert!(matches!(expr, Expr::And(ref parts) if parts.len() == 3));
    /// ```
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
        Self::In {
            expr: Box::new(self),
            list: list.into_iter().map(Into::into).collect(),
        }
    }

    /// `self NOT IN (...)`.
    pub fn not_in_list<I: Into<Expr>>(self, list: impl IntoIterator<Item = I>) -> Self {
        Self::NotIn {
            expr: Box::new(self),
            list: list.into_iter().map(Into::into).collect(),
        }
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

    /// Builds `self op other`.
    fn binary(self, op: BinaryOp, other: impl Into<Expr>) -> Self {
        Self::Binary {
            left: Box::new(self),
            op,
            right: Box::new(other.into()),
        }
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

/// Typed comparisons: values must match the column's Rust type `T`.
///
/// Each helper converts the column into an [`Expr`] and delegates to the
/// matching untyped builder, so both paths always build the same tree.
///
/// # Examples
///
/// ```
/// use flux_db_core::ast::expr::{BinaryOp, Expr};
/// use flux_db_core::column::TypedColumn;
/// use flux_db_core::value::Value;
///
/// struct User;
/// const AGE: TypedColumn<User, i32> = TypedColumn::new("users", "age");
///
/// assert_eq!(
///     AGE.gt(18),
///     Expr::Binary {
///         left: Box::new(Expr::col(AGE)),
///         op: BinaryOp::Gt,
///         right: Box::new(Expr::Value(Value::I32(18))),
///     }
/// );
/// ```
impl<E, T: IntoValue> TypedColumn<E, T> {
    /// `col = value`.
    ///
    /// A `None` value builds `col IS NULL` instead, since `= NULL` never matches.
    pub fn eq(self, value: T) -> Expr {
        match value.into_value() {
            Value::Null => Expr::from(self).is_null(),
            value => Expr::from(self).equals(Expr::Value(value)),
        }
    }

    /// `col <> value`.
    ///
    /// A `None` value builds `col IS NOT NULL` instead, since `<> NULL` never matches.
    pub fn ne(self, value: T) -> Expr {
        match value.into_value() {
            Value::Null => Expr::from(self).is_not_null(),
            value => Expr::from(self).not_equals(Expr::Value(value)),
        }
    }

    /// `col < value`.
    pub fn lt(self, value: T) -> Expr {
        Expr::from(self).less_than(Expr::value(value))
    }

    /// `col <= value`.
    pub fn le(self, value: T) -> Expr {
        Expr::from(self).less_or_equal(Expr::value(value))
    }

    /// `col > value`.
    pub fn gt(self, value: T) -> Expr {
        Expr::from(self).greater_than(Expr::value(value))
    }

    /// `col >= value`.
    pub fn ge(self, value: T) -> Expr {
        Expr::from(self).greater_or_equal(Expr::value(value))
    }

    /// `col IN (...)`. An empty list is rendered as always-false.
    pub fn is_in(self, values: impl IntoIterator<Item = T>) -> Expr {
        Expr::from(self).in_list(values.into_iter().map(Expr::value))
    }

    /// `col NOT IN (...)`. An empty list is rendered as always-true.
    pub fn not_in(self, values: impl IntoIterator<Item = T>) -> Expr {
        Expr::from(self).not_in_list(values.into_iter().map(Expr::value))
    }

    /// `col BETWEEN low AND high` (inclusive on both ends, never swapped).
    pub fn between(self, low: T, high: T) -> Expr {
        Expr::from(self).between(Expr::value(low), Expr::value(high))
    }

    /// `col NOT BETWEEN low AND high` (never swapped).
    pub fn not_between(self, low: T, high: T) -> Expr {
        Expr::from(self).not_between(Expr::value(low), Expr::value(high))
    }
}

/// `IS NULL` only exists on nullable columns.
impl<E, T> TypedColumn<E, Option<T>> {
    /// `col IS NULL`.
    pub fn is_null(self) -> Expr {
        Expr::from(self).is_null()
    }

    /// `col IS NOT NULL`.
    pub fn is_not_null(self) -> Expr {
        Expr::from(self).is_not_null()
    }
}

/// LIKE helpers only exist on text columns.
///
/// [`contains`](Self::contains), [`starts_with`](Self::starts_with) and
/// [`ends_with`](Self::ends_with) match text literally (the dialect escapes
/// `%`, `_` and `\`); [`like`](Self::like) and [`not_like`](Self::not_like)
/// take a raw pattern.
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
        Expr::NotLike {
            expr: Box::new(self.into()),
            pattern: LikePattern::Raw(pattern.into()),
        }
    }

    /// Builds `col LIKE pattern`.
    fn like_pattern(self, pattern: LikePattern) -> Expr {
        Expr::Like {
            expr: Box::new(self.into()),
            pattern,
        }
    }
}
