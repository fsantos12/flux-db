use flux_db_core::query::IntoQuery;
use flux_db_core::query::update::Update;
use flux_db_core::types::column::Column;
use flux_db_core::ast::expr::Expr;

fn main() {
    let _ = Update::table("users").set(Column::new("name"), Expr::now()).into_query();
}
