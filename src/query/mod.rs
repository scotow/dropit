#[macro_export]
macro_rules! include_query {
    ($name:expr) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/query/",
            $name,
            ".sql"
        ))
    };
}
