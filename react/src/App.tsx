import React from "react";
import ReconnectingWebSocket from "reconnecting-websocket";

function App() {
  const [message, setMessage] = React.useState<string>();
  const [inputValue, setInputValue] = React.useState<string>(""); // 入力値を管理するstateを追加
  const socketRef = React.useRef<ReconnectingWebSocket | null>(null);
  const [msList, setMsList] = React.useState<string[]>([]);

  // #0.WebSocket関連の処理は副作用なので、useEffect内で実装
  React.useEffect(() => {
    // #1.WebSocketオブジェクトを生成しサーバとの接続を開始
    const websocket = new ReconnectingWebSocket("ws://localhost:8080");
    socketRef.current = websocket;

    // #2.メッセージ受信時のイベントハンドラを設定
    const onMessage = (event: MessageEvent<string>) => {
      setMessage(event.data);
      setMsList((prevMsList) => [...prevMsList, event.data]);
    };
    websocket.addEventListener("message", onMessage);

    // #3.useEffectのクリーンアップの中で、WebSocketのクローズ処理を実行
    return () => {
      websocket.close();
      websocket.removeEventListener("message", onMessage);
    };
  }, []);

  const handleInputChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    setInputValue(event.target.value);
  };

  const handleSubmit = () => {
    if (inputValue.trim() !== "") {
      // #4.送信ボタンが押されたときの処理
      socketRef.current?.send(inputValue);
    }
  };

  return (
    <>
      <div>message list</div>
      <ul>
        {msList.map((msg, index) => (
          <li key={index}>{msg}</li>
        ))}
      </ul>
      <input
        type="text"
        id="name"
        name="name"
        required
        value={inputValue} // valueをstateにバインド
        onChange={handleInputChange} // onChangeでstateを更新
      />

      <button type="button" onClick={handleSubmit}>
        送信
      </button>
    </>
  );
}

export default App;
