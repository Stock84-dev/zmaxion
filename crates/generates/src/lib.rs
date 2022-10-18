pub fn generate() -> &'static str {
    std::any::type_name::<bincode::config::DefaultOptions>()
}
