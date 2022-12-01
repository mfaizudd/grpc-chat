use data::Client;
use data::chat::chat_server::{Chat, ChatServer};
use data::chat::FILE_DESCRIPTOR_SET;
use data::chat::{ChatMessgae, JoinReply, JoinRequest};
use std::error::Error;
use std::io::ErrorKind;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Server;
use tonic::{Request, Response, Status, Streaming};

use crate::data::Room;

pub mod data;

fn match_for_io_error(err_status: &Status) -> Option<&std::io::Error> {
    let mut err: &(dyn Error + 'static) = err_status;

    loop {
        if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
            return Some(io_err);
        }

        // h2::Error do not expose std::io::Error with `source()`
        // https://github.com/hyperium/h2/pull/462
        if let Some(h2_err) = err.downcast_ref::<h2::Error>() {
            if let Some(io_err) = h2_err.get_io() {
                return Some(io_err);
            }
        }

        err = match err.source() {
            Some(err) => err,
            None => return None,
        };
    }
}

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
        if rooms.len() <= 0 {
            rooms.push(Room::new())
        }
        let room_id = rooms.len() - 1;
        let reply = JoinReply {
            room_id: room_id as i32,
        };

        Ok(Response::new(reply))
    }

    type SendMessageStream = data::SendMessageStream;

    async fn send_message(
        &self,
        request: Request<Streaming<ChatMessgae>>,
    ) -> Result<Response<Self::SendMessageStream>, Status> {
        let mut in_stream = request.into_inner();
        let (sender, receiver) = mpsc::channel(128);
        let rooms = self.rooms.clone();
        tokio::spawn(async move {
            let rooms = rooms.lock().await;
            loop {
                let result = in_stream.message().await;
                match result {
                    Ok(result) => {
                        if let Some(msg) = result {
                            let room = &rooms[msg.room_id as usize];
                            let clients = room.get_clients();
                            let mut clients = clients.lock().await;
                            clients.push(Client {
                                name: msg.name.to_owned(),
                                response_stream: sender.clone()
                            });
                            sender.send(Ok(msg)).await.expect("working sender");
                        }
                    }
                    Err(err) => {
                        if let Some(io_err) = match_for_io_error(&err) {
                            if io_err.kind() == ErrorKind::BrokenPipe {
                                eprintln!("\tclient disconnected: broken pipe");
                                break;
                            }
                            match sender.send(Err(err)).await {
                                Ok(_) => (),
                                Err(_) => break,
                            }
                        }
                    }
                }
            }
        });
        let out_stream = ReceiverStream::new(receiver);
        Ok(Response::new(
            Box::pin(out_stream) as Self::SendMessageStream
        ))
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
