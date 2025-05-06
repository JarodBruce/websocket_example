use futures_util::{SinkExt as _, StreamExt};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, broadcast, mpsc}; // broadcast を追加
use tokio_tungstenite::accept_async;
use tungstenite::protocol::Message;

#[tokio::main]
async fn main() {
    const HOSTING_URL: &str = "localhost:8080";
    let try_socket = TcpListener::bind(HOSTING_URL).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", HOSTING_URL);

    let (broadcast_tx, _) = broadcast::channel::<Message>(100); // ブロードキャストチャネルを作成 (バッファサイズは適宜調整)
    let active_connections = Arc::new(AtomicUsize::new(0));
    let shared_received_msg_list = Arc::new(Mutex::new(Vec::<Message>::new())); // 共有リストを初期化

    // 初期メッセージを追加
    {
        let mut guard = shared_received_msg_list.lock().await;
        guard.push(Message::text("fast msg"));
    }

    while let Ok((stream, _)) = listener.accept().await {
        let connections_clone = Arc::clone(&active_connections);
        let list_clone_for_task = Arc::clone(&shared_received_msg_list);
        let current_broadcast_tx = broadcast_tx.clone(); // 各接続タスク用に送信側をクローン
        let mut current_broadcast_rx = broadcast_tx.subscribe(); // 各接続タスク用に受信側を作成

        tokio::spawn(async move {
            let current_total_connections = connections_clone.fetch_add(1, Ordering::SeqCst) + 1;
            println!("NEW WS connection. TAC: {}", current_total_connections);

            let ws_stream = match accept_async(stream).await {
                Ok(ws) => ws,
                Err(e) => {
                    eprintln!("Error during the websocket handshake: {:?}", e);
                    let current_total_connections =
                        connections_clone.fetch_sub(1, Ordering::SeqCst) - 1;
                    println!("WS failed. TAC: {}", current_total_connections);
                    return;
                }
            };

            let (mut ws_sender, mut ws_receiver) = ws_stream.split();
            // let (tx, mut rx) = mpsc::unbounded_channel(); // このmpscチャネルはブロードキャストには不要

            // 各クライアントがブロードキャストされたメッセージを受信して自身のWebSocketに送信するタスク
            tokio::spawn(async move {
                while let Ok(msg) = current_broadcast_rx.recv().await {
                    if ws_sender.send(msg).await.is_err() {
                        // クライアントが切断されたなどの理由で送信に失敗した場合
                        break;
                    }
                }
            });

            while let Some(msg_result) = ws_receiver.next().await {
                match msg_result {
                    Ok(msg) => {
                        if msg.is_close() {
                            break;
                        }

                        let mut received_msg_list_guard = list_clone_for_task.lock().await;
                        received_msg_list_guard.push(msg.clone());

                        println!(
                            "{}",
                            received_msg_list_guard[received_msg_list_guard.len() - 1]
                        );

                        let ws_return_msg_content = {
                            received_msg_list_guard[received_msg_list_guard.len() - 1]
                                .to_string()
                                .replace("Text(", "")
                                .replace(")", "")
                                .replace("\"", "")
                        };

                        drop(received_msg_list_guard);

                        // 変更点: ここでブロードキャストチャネルにメッセージを送信します
                        if current_broadcast_tx
                            .send(Message::text(ws_return_msg_content.to_string()))
                            .is_err()
                        {
                            eprintln!(
                                "Failed to broadcast message. Maybe no active subscribers or channel lagged."
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("Error receiving message: {:?}", e);
                        break;
                    }
                }
            }

            let current_total_connections = connections_clone.fetch_sub(1, Ordering::SeqCst) - 1;
            println!("WS closed: TAC: {}", current_total_connections);
        });
    }
}
