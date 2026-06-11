pub fn init_logger() {
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();
}
