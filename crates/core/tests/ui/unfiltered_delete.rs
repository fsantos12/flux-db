use flux_db_core::query::IntoQuery;
use flux_db_core::query::delete::Delete;

fn main() {
    let _ = Delete::from("users").into_query();
}
