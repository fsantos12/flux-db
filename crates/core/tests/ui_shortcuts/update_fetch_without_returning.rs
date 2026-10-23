#![allow(unused_imports)]
use flux_db_core::driver::Executor;
use flux_db_core::query::{delete::Delete, insert::{Insert, InsertRow}, select::Select, update::Update};
use flux_db_core::types::{column::Column, row::Row};
use flux_db_core::ast::expr::Expr;

#[allow(dead_code)]
fn run(db: &impl Executor) {
    let _ = Update::table("t").set(Column::new("a"), Expr::value(1)).all().fetch_one::<Row>(db);
}

fn main() {}
