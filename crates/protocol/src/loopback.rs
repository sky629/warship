//! 같은 프로세스 안에서 메시지를 전달하는 접속 방식입니다.
//!
//! 서버 테스트는 이 접속 방식으로 소켓과 스레드 없이 실제 `GameServer`를 실행합니다.
//! 메시지를 바이트로 바꾸지 않고 값 그대로 옮깁니다.

use std::collections::{BTreeMap, VecDeque};
use std::net::{IpAddr, Ipv4Addr};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};

use crate::messages::{ClientMsg, ServerMsg};
use crate::transport::{
    ClientEvent, ClientTransport, LinkId, PeerKey, ServerEvent, ServerTransport,
};

/// 서버 쪽 끝과, 클라이언트 쪽 끝을 원하는 만큼 만드는 연결기를 만듭니다.
pub fn loopback() -> (LoopbackServer, LoopbackConnector) {
    let (to_server, inbox) = channel();
    let server = LoopbackServer {
        inbox,
        outboxes: BTreeMap::new(),
        closed: VecDeque::new(),
    };
    let connector = LoopbackConnector {
        to_server,
        next_link: 1,
    };
    (server, connector)
}

enum ToServer {
    Open(LinkId, PeerKey, Sender<ToClient>),
    Msg(LinkId, ClientMsg),
    Closed(LinkId),
}

enum ToClient {
    Msg(ServerMsg),
    Closed,
}

pub struct LoopbackServer {
    /// 모든 클라이언트 끝이 같은 채널로 보냅니다. 그래서 서버는 이벤트를 도착한 순서대로 받습니다.
    inbox: Receiver<ToServer>,
    /// 열려 있는 접속만 들어 있습니다. 여기에 없는 접속의 메시지는 버립니다.
    outboxes: BTreeMap<LinkId, Sender<ToClient>>,
    /// 서버가 `close`를 호출해서 아직 `Disconnected`를 돌려주지 않은 접속입니다.
    closed: VecDeque<LinkId>,
}

impl ServerTransport for LoopbackServer {
    fn poll(&mut self) -> Option<ServerEvent> {
        if let Some(link) = self.closed.pop_front() {
            return Some(ServerEvent::Disconnected(link));
        }
        loop {
            match self.inbox.try_recv().ok()? {
                ToServer::Open(link, peer, outbox) => {
                    self.outboxes.insert(link, outbox);
                    return Some(ServerEvent::Connected(link, peer));
                }
                ToServer::Msg(link, msg) => {
                    if self.outboxes.contains_key(&link) {
                        return Some(ServerEvent::Message(link, msg));
                    }
                }
                ToServer::Closed(link) => {
                    if self.outboxes.remove(&link).is_some() {
                        return Some(ServerEvent::Disconnected(link));
                    }
                }
            }
        }
    }

    fn send(&mut self, link: LinkId, msg: &ServerMsg) {
        if let Some(outbox) = self.outboxes.get(&link) {
            // 클라이언트 끝이 이미 사라졌으면 보내기가 실패합니다. 그 접속의 `Closed`가 곧 도착하므로 여기서는 무시합니다.
            let _ = outbox.send(ToClient::Msg(msg.clone()));
        }
    }

    fn close(&mut self, link: LinkId) {
        if let Some(outbox) = self.outboxes.remove(&link) {
            let _ = outbox.send(ToClient::Closed);
            self.closed.push_back(link);
        }
    }
}

pub struct LoopbackConnector {
    to_server: Sender<ToServer>,
    next_link: u64,
}

impl LoopbackConnector {
    /// 접속 하나를 엽니다. 서버와 클라이언트는 각각 다음 `poll`에서 `Connected`를 받습니다. 상대의 주소는 `127.0.0.1`입니다.
    pub fn connect(&mut self) -> LoopbackClient {
        self.connect_from(IpAddr::V4(Ipv4Addr::LOCALHOST))
    }

    /// `connect`와 같고, 상대의 주소를 정합니다. 주소가 같은 접속은 서버에서 같은 상대입니다.
    pub fn connect_from(&mut self, address: IpAddr) -> LoopbackClient {
        let link = LinkId(self.next_link);
        self.next_link += 1;
        let (outbox, inbox) = channel();
        let _ = self
            .to_server
            .send(ToServer::Open(link, PeerKey::of(address), outbox));
        LoopbackClient {
            link,
            to_server: self.to_server.clone(),
            inbox,
            state: ClientState::Opening,
        }
    }
}

enum ClientState {
    Opening,
    Open,
    Finished,
}

/// 클라이언트 쪽 끝입니다. 이 값을 버리면(drop) 서버는 `Disconnected`를 받습니다.
pub struct LoopbackClient {
    link: LinkId,
    to_server: Sender<ToServer>,
    inbox: Receiver<ToClient>,
    state: ClientState,
}

impl ClientTransport for LoopbackClient {
    fn poll(&mut self) -> Option<ClientEvent> {
        match self.state {
            ClientState::Opening => {
                self.state = ClientState::Open;
                Some(ClientEvent::Connected)
            }
            ClientState::Finished => None,
            ClientState::Open => match self.inbox.try_recv() {
                Ok(ToClient::Msg(msg)) => Some(ClientEvent::Message(msg)),
                Ok(ToClient::Closed) => self.finish("closed by the server"),
                Err(TryRecvError::Disconnected) => self.finish("the server is gone"),
                Err(TryRecvError::Empty) => None,
            },
        }
    }

    fn send(&mut self, msg: &ClientMsg) {
        if let ClientState::Open = self.state {
            let _ = self.to_server.send(ToServer::Msg(self.link, msg.clone()));
        }
    }
}

impl LoopbackClient {
    fn finish(&mut self, reason: &str) -> Option<ClientEvent> {
        self.state = ClientState::Finished;
        Some(ClientEvent::Disconnected {
            reason: reason.to_string(),
        })
    }
}

impl Drop for LoopbackClient {
    fn drop(&mut self) {
        let _ = self.to_server.send(ToServer::Closed(self.link));
    }
}
