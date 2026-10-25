//! Executor shortcuts shared by the builders.
//!
//! Each shortcut builds the [`Query`](crate::query::Query) through
//! [`IntoQuery`](crate::query::IntoQuery) and delegates to the matching
//! [`Executor`](crate::driver::executor::Executor) method. A build error is
//! returned without touching the executor. There is no streaming `fetch`
//! shortcut: the stream borrows the query, so call `db.fetch(&query)`.

/// Adds `execute`.
macro_rules! execute_shortcut {
    () => {
        /// Builds the query and runs it, returning the number of affected rows.
        ///
        /// # Errors
        ///
        /// Returns [`Error::InvalidQuery`](crate::types::error::Error::InvalidQuery)
        /// when the builder is invalid, or any error from
        /// [`Executor::execute`](crate::driver::executor::Executor::execute).
        pub fn execute(
            self,
            db: &impl crate::driver::executor::Executor,
        ) -> impl ::std::future::Future<Output = crate::types::error::Result<u64>> + Send {
            let query = crate::query::IntoQuery::into_query(self);
            async move { db.execute(&query?).await }
        }
    };
}

/// Adds `fetch_all`, `fetch_one` and `fetch_one_optional`, typed through
/// [`FromRow`](crate::types::row::FromRow).
macro_rules! fetch_shortcuts {
    () => {
        /// Builds the query and collects every row as `T` (use `Row` for untyped rows).
        ///
        /// # Errors
        ///
        /// Returns [`Error::InvalidQuery`](crate::types::error::Error::InvalidQuery)
        /// when the builder is invalid, any error from
        /// [`Executor::fetch_all`](crate::driver::executor::Executor::fetch_all),
        /// or the first [`FromRow`](crate::types::row::FromRow) decode error.
        pub fn fetch_all<T: crate::types::row::FromRow>(
            self,
            db: &impl crate::driver::executor::Executor,
        ) -> impl ::std::future::Future<Output = crate::types::error::Result<Vec<T>>> + Send {
            let query = crate::query::IntoQuery::into_query(self);
            async move {
                db.fetch_all(&query?)
                    .await?
                    .iter()
                    .map(T::from_row)
                    .collect()
            }
        }

        /// Builds the query and decodes the first row as `T`.
        ///
        /// # Errors
        ///
        /// Returns [`Error::InvalidQuery`](crate::types::error::Error::InvalidQuery)
        /// when the builder is invalid, [`Error::NotFound`](crate::types::error::Error::NotFound)
        /// when there are no rows, any executor error, or a
        /// [`FromRow`](crate::types::row::FromRow) decode error.
        pub fn fetch_one<T: crate::types::row::FromRow>(
            self,
            db: &impl crate::driver::executor::Executor,
        ) -> impl ::std::future::Future<Output = crate::types::error::Result<T>> + Send {
            let query = crate::query::IntoQuery::into_query(self);
            async move { T::from_row(&db.fetch_one(&query?).await?) }
        }

        /// Builds the query and decodes the first row as `T`; `Ok(None)` when there are none.
        ///
        /// # Errors
        ///
        /// Returns [`Error::InvalidQuery`](crate::types::error::Error::InvalidQuery)
        /// when the builder is invalid, any executor error, or a
        /// [`FromRow`](crate::types::row::FromRow) decode error.
        pub fn fetch_one_optional<T: crate::types::row::FromRow>(
            self,
            db: &impl crate::driver::executor::Executor,
        ) -> impl ::std::future::Future<Output = crate::types::error::Result<Option<T>>> + Send {
            let query = crate::query::IntoQuery::into_query(self);
            async move {
                db.fetch_one_optional(&query?)
                    .await?
                    .as_ref()
                    .map(T::from_row)
                    .transpose()
            }
        }
    };
}

pub(crate) use {execute_shortcut, fetch_shortcuts};
