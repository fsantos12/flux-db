#![allow(unused_imports)]
use flux_db_core::driver::Executor;
use flux_db_core::query::{delete::Delete, insert::{Insert, InsertRow}, select::Select, update::Update};
use flux_db_core::types::{column::Column, row::Row};
use flux_db_core::ast::expr::Expr;

#[allow(dead_code)]
fn run(db: &impl Executor) {
    let _ = Delete::from("t").returning(Column::new("id")).fetch_all::<Row>(db);
}

fn main() {}
