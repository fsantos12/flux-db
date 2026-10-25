#![allow(unused_imports)]
use flux_db_core::driver::Executor;
use flux_db_core::query::{delete::Delete, insert::{Insert, InsertRow}, select::Select, update::Update};
use flux_db_core::types::{column::Column, row::Row};
use flux_db_core::ast::expr::Expr;

#[allow(dead_code)]
fn run(db: &impl Executor) {
    let _ = Select::from("t").fetch_all::<String>(db);
}

fn main() {}
