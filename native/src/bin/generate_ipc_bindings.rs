use app_lib::router;
use tauri::test::mock_builder;

#[taurpc::procedures(export_to = "src/generated/ipc_bindings.ts")]
trait Api {
    async fn unreachable();
}

#[derive(Clone, Default)]
struct ApiImpl;
#[taurpc::resolvers]
impl Api for ApiImpl {
    async fn unreachable(self) {
        panic!("此函数仅用于占位，请勿使用");
    }
}

#[tokio::main]
async fn main() {
    mock_builder()
        .invoke_handler(
            router()
                .merge(ApiImpl::default().into_handler())
                .into_handler(),
        )
        .build(tauri::generate_context!())
        .unwrap();
}
