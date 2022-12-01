use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::Status;

use chat::ChatMessage;

pub mod chat {
    tonic::include_proto!("chat");

    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("chat_descriptor");
}

pub type GetMessageStream = ReceiverStream<Result<ChatMessage, Status>>;

pub struct Room {
    clients: Arc<Mutex<Vec<Client>>>
}

impl Room {
    pub fn new() -> Self {
        Room { clients: Arc::new(Mutex::new(vec![])) }
    }

    pub fn get_clients(&self) -> Arc<Mutex<Vec<Client>>> {
        self.clients.clone()
    }
}

pub struct Client {
    pub name: String,
    pub response_stream: Option<mpsc::Sender<Result<ChatMessage,Status>>>,
}
