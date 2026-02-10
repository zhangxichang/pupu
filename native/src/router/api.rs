#[taurpc::procedures(export_to = "../src/generated/ipc_bindings.ts")]
pub trait Api {
    async fn unreachable();
}

#[derive(Clone, Default)]
pub struct ApiImpl;
#[taurpc::resolvers]
impl Api for ApiImpl {
    async fn unreachable(self) {
        panic!("此函数仅用于占位，请勿使用");
    }
}
