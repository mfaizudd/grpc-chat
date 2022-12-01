use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tonic::Status;
use tonic::codegen::futures_core::Stream;

use chat::ChatMessgae;

pub mod chat {
    tonic::include_proto!("chat");

    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("chat_descriptor");
}

pub type SendMessageStream = Pin<Box<dyn Stream<Item = Result<ChatMessgae, Status>> + Send + 'static>>;

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
    pub response_stream: mpsc::Sender<Result<ChatMessgae,Status>>,
}
