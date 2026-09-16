use diesel::{expression::SqlLiteral, sql_types::Double};

pub fn sqlite_random() -> SqlLiteral<Double> {
    diesel::dsl::sql("RANDOM()")
}
