mod engine;
mod peer;
mod peer_client;

use std::time::Duration;

use eyre::{Result, eyre};
use test_protocol::{MessageView, Netem};

use engine::Sandbox;
use peer::Peer;

async fn private_chat(a: &Peer, b: &Peer) -> Result<()> {
    let a_id = a.identity().await?.endpoint_id;
    let b_id = b.identity().await?.endpoint_id;
    assert_ne!(a_id, b_id, "两个节点不能有相同身份");

    a.contact_request(&b_id, "请求添加联系人").await?;
    wait_contact(b, &a_id, "pending_in").await?;

    b.contact_accept(&a_id).await?;
    wait_contact(b, &a_id, "accepted").await?;
    wait_contact(a, &b_id, "accepted").await?;

    a.send_message(&b_id, "hello from a").await?;

    let received = wait_message(b, "hello from a").await?;
    assert_eq!(received.status, "sent");

    let sent = wait_message(a, "hello from a").await?;
    assert_eq!(sent.status, "sent");

    a.netem(Netem {
        delay_ms: Some(80),
        jitter_ms: Some(20),
        loss_percent: Some(2.0),
        rate_kbit: None,
    })
    .await?;
    a.send_message(&b_id, "hello under netem").await?;
    wait_message(b, "hello under netem").await?;
    a.netem(Netem::default()).await?;

    Ok(())
}

async fn wait_contact(peer: &Peer, endpoint_id: &str, expected: &str) -> Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        let snapshot = peer.state().await?;
        if snapshot
            .contacts
            .iter()
            .any(|contact| contact.endpoint_id == endpoint_id && contact.state == expected)
        {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(eyre!(
                "等待联系人状态 {expected} 超时，当前状态：{:?}",
                snapshot.contacts
            ));
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

async fn wait_message(peer: &Peer, markdown: &str) -> Result<MessageView> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        let snapshot = peer.state().await?;
        if let Some(message) = snapshot
            .messages
            .iter()
            .find(|message| message.markdown == markdown)
        {
            return Ok(message.clone());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(eyre!("等待消息超时，当前消息：{:?}", snapshot.messages));
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

async fn run(sandbox: &mut Sandbox) -> Result<()> {
    let a = Peer::start(sandbox, "a").await?;
    let b = Peer::start(sandbox, "b").await?;

    let result = private_chat(&a, &b).await;
    let _ = a.stop().await;
    let _ = b.stop().await;
    result
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut sandbox = Sandbox::start().await?;
    let result = run(&mut sandbox).await;
    sandbox.shutdown().await;
    result?;

    println!("private_chat: ok");
    Ok(())
}
