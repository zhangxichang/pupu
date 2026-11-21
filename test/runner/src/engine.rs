use std::env;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};

use eyre::{Result, eyre};
use tokio::process::Command;

const DEFAULT_PEER_IMAGE: &str = "pupu-test-peer:latest";
const DEFAULT_RELAY_IMAGE: &str = "docker.io/n0computer/iroh-relay:latest";
pub(crate) const PEER_CONTROL_PORT: u16 = 18080;
const RELAY_PORT: u16 = 3340;

#[derive(Debug, Clone, Copy)]
enum EngineKind {
    Docker,
    Podman,
}

#[derive(Clone)]
pub(crate) struct Engine {
    kind: EngineKind,
}

impl Engine {
    fn program(&self) -> &'static str {
        match self.kind {
            EngineKind::Docker => "docker",
            EngineKind::Podman => "podman",
        }
    }

    fn from_name(name: &str) -> Result<Self> {
        match name {
            "docker" => Ok(Self {
                kind: EngineKind::Docker,
            }),
            "podman" => Ok(Self {
                kind: EngineKind::Podman,
            }),
            _ => Err(eyre!("未知容器引擎: {name}")),
        }
    }

    async fn detect() -> Result<Self> {
        if let Ok(name) = env::var("PUPU_CONTAINER_ENGINE") {
            let engine = Self::from_name(&name)?;
            engine.ensure_available().await?;
            return Ok(engine);
        }

        for name in ["docker", "podman"] {
            let Ok(engine) = Self::from_name(name) else {
                continue;
            };
            if engine.ensure_available().await.is_ok() {
                return Ok(engine);
            }
        }

        Err(eyre!(
            "找不到可用的容器引擎，请设置 PUPU_CONTAINER_ENGINE=docker|podman"
        ))
    }

    async fn ensure_available(&self) -> Result<()> {
        let output = Command::new(self.program())
            .arg("info")
            .output()
            .await
            .map_err(|error| eyre!("无法执行 {}: {error}", self.program()))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(eyre!(
                "{} 不可用: {}",
                self.program(),
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    }

    pub(crate) async fn run(&self, args: &[String]) -> Result<String> {
        let output = Command::new(self.program()).args(args).output().await?;
        if !output.status.success() {
            return Err(eyre!(
                "{} {} 失败: {}",
                self.program(),
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn status_inherit(&self, args: &[String]) -> Result<()> {
        let status = Command::new(self.program())
            .args(args)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await?;
        if !status.success() {
            return Err(eyre!(
                "{} {} 失败: {status}",
                self.program(),
                args.join(" ")
            ));
        }
        Ok(())
    }

    async fn command_succeeds(&self, args: &[String]) -> bool {
        Command::new(self.program())
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|status| status.success())
            .unwrap_or(false)
    }
}

pub(crate) struct Sandbox {
    pub(crate) engine: Engine,
    pub(crate) network: String,
    pub(crate) relay_url: String,
    pub(crate) peer_image: String,
    pub(crate) containers: Vec<String>,
}

impl Sandbox {
    pub(crate) async fn start() -> Result<Self> {
        let engine = Engine::detect().await?;
        eprintln!("容器引擎: {:?}", engine.kind);

        let peer_image =
            env::var("PUPU_PEER_IMAGE").unwrap_or_else(|_| DEFAULT_PEER_IMAGE.to_string());
        let relay_image =
            env::var("PUPU_RELAY_IMAGE").unwrap_or_else(|_| DEFAULT_RELAY_IMAGE.to_string());

        ensure_peer_image(&engine, &peer_image).await?;
        ensure_image(&engine, &relay_image).await?;

        let run_id = run_id()?;
        let network = format!("pupu-test-{run_id}");
        let relay_url = format!("http://relay:{RELAY_PORT}");
        let mut sandbox = Self {
            engine,
            network,
            relay_url,
            peer_image,
            containers: Vec::new(),
        };

        if let Err(error) = sandbox.start_network_and_relay(&relay_image).await {
            sandbox.shutdown().await;
            return Err(error);
        }

        eprintln!("测试网络: {}", sandbox.network);
        Ok(sandbox)
    }

    async fn start_network_and_relay(&mut self, relay_image: &str) -> Result<()> {
        self.engine
            .run(&[
                "network".to_string(),
                "create".to_string(),
                self.network.clone(),
            ])
            .await?;

        let name = format!("{}-relay", self.network);
        let args = vec![
            "run".to_string(),
            "-d".to_string(),
            "--rm".to_string(),
            "--name".to_string(),
            name.clone(),
            "--label".to_string(),
            format!("pupu.test.run={}", self.network),
            "--network".to_string(),
            self.network.clone(),
            "--network-alias".to_string(),
            "relay".to_string(),
            relay_image.to_string(),
            "--dev".to_string(),
        ];

        self.engine.run(&args).await?;
        self.containers.push(name);
        Ok(())
    }

    pub(crate) async fn shutdown(&self) {
        for name in self.containers.iter().rev() {
            let _ = self
                .engine
                .run(&["rm".to_string(), "-f".to_string(), name.clone()])
                .await;
        }

        let _ = self
            .engine
            .run(&[
                "network".to_string(),
                "rm".to_string(),
                self.network.clone(),
            ])
            .await;
    }
}

async fn ensure_peer_image(engine: &Engine, image: &str) -> Result<()> {
    let rebuild = env::var("PUPU_REBUILD_PEER")
        .map(|value| value == "1")
        .unwrap_or(false);
    let inspect = vec![
        "image".to_string(),
        "inspect".to_string(),
        image.to_string(),
    ];
    if !rebuild && engine.command_succeeds(&inspect).await {
        return Ok(());
    }

    eprintln!("构建 peer 镜像 {image} ...");
    let repo_root = repo_root()?;
    let status = Command::new(engine.program())
        .current_dir(&repo_root)
        .args([
            "build",
            "-f",
            "test/Dockerfile",
            "--target",
            "peer",
            "-t",
            image,
            ".",
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await?;
    if !status.success() {
        return Err(eyre!("构建 peer 镜像失败: {status}"));
    }
    Ok(())
}

async fn ensure_image(engine: &Engine, image: &str) -> Result<()> {
    let inspect = vec![
        "image".to_string(),
        "inspect".to_string(),
        image.to_string(),
    ];
    if engine.command_succeeds(&inspect).await {
        return Ok(());
    }

    eprintln!("拉取镜像 {image} ...");
    engine
        .status_inherit(&["pull".to_string(), image.to_string()])
        .await
}

fn repo_root() -> Result<PathBuf> {
    if let Ok(path) = env::var("PUPU_REPO_ROOT") {
        return Ok(PathBuf::from(path));
    }
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

fn run_id() -> Result<String> {
    let millis = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    Ok(format!("{}-{millis}", std::process::id()))
}

pub(crate) fn free_port() -> Result<u16> {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0))?;
    Ok(listener.local_addr()?.port())
}
