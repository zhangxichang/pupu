#[taurpc::procedures(path = "log")]
pub trait LogApi {
    async fn info(message: String);
    async fn warn(message: String);
    async fn error(message: String);
}

#[derive(Clone, Default)]
pub struct LogApiImpl;
#[taurpc::resolvers]
impl LogApi for LogApiImpl {
    async fn info(self, message: String) {
        log::info!("{}", message);
    }
    async fn warn(self, message: String) {
        log::warn!("{}", message);
    }
    async fn error(self, message: String) {
        log::error!("{}", message);
    }
}
