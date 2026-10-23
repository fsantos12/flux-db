//! Core of flux-db: shared [`types`], the query [`ast`], query builders in
//! [`query`] and the [`driver`] traits.
//!
//! This crate is database-agnostic; driver implementations and SQL rendering
//! live in other crates.

pub mod ast;
pub mod driver;
pub mod entity;
pub mod query;
pub mod types;
