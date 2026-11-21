use eyre::Result;
use test_protocol::{Identity, Netem, PeerState};

use crate::engine::{Engine, PEER_CONTROL_PORT, Sandbox, free_port};
use crate::peer_client::PeerClient;

pub(crate) struct Peer {
    engine: Engine,
    container: String,
    client: PeerClient,
}

impl Peer {
    pub(crate) async fn start(sandbox: &mut Sandbox, name: &str) -> Result<Self> {
        let host_port = free_port()?;
        let container = format!("{}-peer-{name}", sandbox.network);
        let url = format!("http://127.0.0.1:{host_port}");

        let args = vec![
            "run".to_string(),
            "-d".to_string(),
            "--rm".to_string(),
            "--name".to_string(),
            container.clone(),
            "--label".to_string(),
            format!("pupu.test.run={}", sandbox.network),
            "--network".to_string(),
            sandbox.network.clone(),
            "--network-alias".to_string(),
            format!("peer-{name}"),
            "--cap-add".to_string(),
            "NET_ADMIN".to_string(),
            "--cap-add".to_string(),
            "NET_RAW".to_string(),
            "-p".to_string(),
            format!("127.0.0.1:{host_port}:{PEER_CONTROL_PORT}"),
            "--tmpfs".to_string(),
            "/data".to_string(),
            "-e".to_string(),
            format!("PUPU_NAME=peer-{name}"),
            "-e".to_string(),
            format!("PUPU_RELAY_URL={}", sandbox.relay_url),
            "-e".to_string(),
            format!("PUPU_CONTROL_ADDR=0.0.0.0:{PEER_CONTROL_PORT}"),
            "-e".to_string(),
            "PUPU_DATA_DIR=/data".to_string(),
            "-e".to_string(),
            "PUPU_ALLOW_NETEM=1".to_string(),
            "-e".to_string(),
            "RUST_LOG=info".to_string(),
            sandbox.peer_image.clone(),
        ];

        sandbox.engine.run(&args).await?;
        sandbox.containers.push(container.clone());

        let client = PeerClient::new(url.clone());
        if let Err(error) = client.wait_healthy().await {
            let _ = sandbox
                .engine
                .run(&["rm".to_string(), "-f".to_string(), container.clone()])
                .await;
            sandbox.containers.retain(|existing| existing != &container);
            return Err(error);
        }

        eprintln!("peer-{name}: {url}");

        Ok(Self {
            engine: sandbox.engine.clone(),
            container,
            client,
        })
    }

    pub(crate) async fn identity(&self) -> Result<Identity> {
        self.client.identity().await
    }

    pub(crate) async fn state(&self) -> Result<PeerState> {
        self.client.state().await
    }

    pub(crate) async fn contact_request(&self, target: &str, message: &str) -> Result<()> {
        self.client.contact_request(target, message).await
    }

    pub(crate) async fn contact_accept(&self, target: &str) -> Result<()> {
        self.client.contact_accept(target).await
    }

    pub(crate) async fn send_message(&self, target: &str, markdown: &str) -> Result<()> {
        self.client.send_message(target, markdown).await
    }

    pub(crate) async fn netem(&self, netem: Netem) -> Result<()> {
        self.client.netem(netem).await
    }

    pub(crate) async fn stop(self) -> Result<()> {
        self.engine
            .run(&["rm".to_string(), "-f".to_string(), self.container])
            .await?;
        Ok(())
    }
}
