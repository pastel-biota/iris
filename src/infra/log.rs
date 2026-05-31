use tracing_subscriber::EnvFilter;

pub fn initialize_log() {
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env());

    if std::env::var("IRIS_JSON").is_ok_and(|content| !content.trim().is_empty()) {
        subscriber.json().init();
    } else {
        subscriber.init();
    }
}
