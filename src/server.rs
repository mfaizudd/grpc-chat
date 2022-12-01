use data::chat::chat_server::{Chat, ChatServer};
use data::chat::{ChatMessage, ChatRequest, JoinReply, JoinRequest, MessageRequest};
use data::chat::{Empty, FILE_DESCRIPTOR_SET};
use data::Client;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

use crate::data::Room;

pub mod data;

pub struct ChatService {
    rooms: Arc<Mutex<Vec<Room>>>,
}

impl Default for ChatService {
    fn default() -> Self {
        ChatService {
            rooms: Arc::new(Mutex::new(vec![])),
        }
    }
}

#[tonic::async_trait]
impl Chat for ChatService {
    async fn join(&self, request: Request<JoinRequest>) -> Result<Response<JoinReply>, Status> {
        println!("Got a request: {:?}", request);
        let mut rooms = self.rooms.lock().await;
        let request = request.into_inner();
        if rooms.len() <= 0 {
            rooms.push(Room::new())
        }
        let room_id = rooms.len() - 1;
        let room = &rooms[room_id];
        let clients = room.get_clients();
        let mut clients = clients.lock().await;
        clients.push(Client {
            name: request.name,
            response_stream: None
        });
        let reply = JoinReply {
            room_id: room_id as i32,
        };

        Ok(Response::new(reply))
    }

    type GetMessageStream = data::GetMessageStream;

    async fn get_message(
        &self,
        request: Request<ChatRequest>,
    ) -> Result<Response<Self::GetMessageStream>, Status> {
        let request = request.into_inner();
        let (sender, receiver) = mpsc::channel(128);
        let rooms = self.rooms.clone();
        tokio::spawn(async move {
            let rooms = rooms.lock().await;
            let room = &rooms[request.room_id as usize];
            let clients = room.get_clients();
            let mut clients = clients.lock().await;
            let client = clients.iter_mut().find(|c| c.name == request.name);
            if let Some(client) = client {
                client.response_stream = Some(sender);
            }
        });
        Ok(Response::new(ReceiverStream::new(receiver)))
    }

    async fn send_message(
        &self,
        request: Request<MessageRequest>,
    ) -> Result<Response<Empty>, Status> {
        let request = request.into_inner();
        let rooms = self.rooms.lock().await;
        let room = &rooms[request.room_id as usize];
        let clients = room.get_clients();
        let clients = clients.lock().await;
        for client in clients.iter() {
            if let Some(stream) = &client.response_stream {
                stream.send(Ok(ChatMessage {
                    name: request.name.clone(),
                    body: request.body.clone(),
                })).await.expect("Failed to send message");
            }
        }
        Ok(Response::new(Empty {}))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build()?;
    let addr = "0.0.0.0:5000".parse()?;
    let chat_service = ChatService::default();
    Server::builder()
        .add_service(reflection)
        .add_service(ChatServer::new(chat_service))
        .serve(addr)
        .await?;

    Ok(())
}
