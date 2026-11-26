//! WebSocket으로 `ClientTransport`를 구현합니다. PC와 웹에서 같은 코드를 사용합니다 (ewebsock).
//!
//! ewebsock은 PC에서는 별도 스레드의 tungstenite로, 웹에서는 브라우저의 WebSocket으로 동작합니다.
//! 웹의 송신 핸들은 다른 스레드로 옮길 수 없으므로, Bevy 앱은 이 값을 non-send 리소스로 보관합니다.

use ewebsock::{Options, WsEvent, WsMessage, WsReceiver, WsSender};
use warship_protocol::{ClientEvent, ClientMsg, ClientTransport, decode_server, encode_client};

/// 서버가 보내는 메시지 하나의 최대 크기(바이트)입니다. 몬스터 한 마리가 약 10바이트이므로 수만 마리까지 담는 크기입니다.
///
/// 정상적인 서버의 메시지는 이보다 훨씬 작습니다. 이 한계는 터무니없이 큰 메시지로 클라이언트의 메모리를 쓰게 하는 서버를 막습니다.
/// PC에서는 ewebsock이 이 크기를 넘는 프레임을 읽지 않고 접속을 끊고, 브라우저에서는 ewebsock이 이 설정을 무시하므로 받은 뒤에 크기를 확인합니다.
pub const MAX_SERVER_MESSAGE_BYTES: usize = 256 * 1024;

pub struct WsClient {
    sender: WsSender,
    receiver: WsReceiver,
    /// ewebsock은 오류 뒤에 닫힘을 다시 알리기도 하고 알리지 않기도 합니다.
    /// 약속대로 `Disconnected`를 한 번만 돌려주기 위해 끝난 상태를 기억합니다.
    finished: bool,
}

impl WsClient {
    /// 접속을 시작하고 바로 돌아옵니다. 접속의 성공과 실패는 나중에 `poll`로 도착합니다.
    pub fn connect(url: &str) -> Result<WsClient, String> {
        let options = Options {
            max_incoming_frame_size: MAX_SERVER_MESSAGE_BYTES,
            ..Options::default()
        };
        let (sender, receiver) = ewebsock::connect(url, options)?;
        Ok(WsClient {
            sender,
            receiver,
            finished: false,
        })
    }

    fn finish(&mut self, reason: String) -> Option<ClientEvent> {
        self.finished = true;
        self.sender.close();
        Some(ClientEvent::Disconnected { reason })
    }
}

impl ClientTransport for WsClient {
    fn poll(&mut self) -> Option<ClientEvent> {
        if self.finished {
            return None;
        }
        loop {
            match self.receiver.try_recv()? {
                WsEvent::Opened => return Some(ClientEvent::Connected),
                WsEvent::Message(WsMessage::Binary(bytes)) => {
                    if bytes.len() > MAX_SERVER_MESSAGE_BYTES {
                        return self.finish(format!(
                            "the server sent a message of {} bytes, above the limit of {MAX_SERVER_MESSAGE_BYTES}",
                            bytes.len()
                        ));
                    }
                    return match decode_server(&bytes) {
                        Ok(msg) => Some(ClientEvent::Message(msg)),
                        Err(err) => self.finish(format!("bad message from the server: {err}")),
                    };
                }
                // 서버는 이진 프레임만 보냅니다. Ping과 Pong은 ewebsock이 처리합니다.
                WsEvent::Message(_) => {}
                WsEvent::Error(err) => return self.finish(err),
                WsEvent::Closed => {
                    return self.finish("the server closed the connection".to_string());
                }
            }
        }
    }

    fn send(&mut self, msg: &ClientMsg) {
        if !self.finished {
            self.sender.send(WsMessage::Binary(encode_client(msg)));
        }
    }
}
