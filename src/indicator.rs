pub async fn run_indicator() -> anyhow::Result<()> {
    crate::platform::imp::indicator::run_indicator().await
}
