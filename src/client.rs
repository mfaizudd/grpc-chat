use std::error::Error;
use std::io::stdin;
use std::io::stdout;
use std::io::Write;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;

use clap::Parser;
use crossterm::cursor::MoveToColumn;
use crossterm::cursor::MoveUp;
use crossterm::execute;
use crossterm::terminal::Clear;
use data::chat::chat_client::ChatClient;
use data::chat::ChatRequest;
use data::chat::JoinRequest;
use data::chat::MessageRequest;
use directories::ProjectDirs;
use dotenvy::dotenv;
use tonic::transport::Certificate;
use tonic::transport::Channel;
use tonic::transport::ClientTlsConfig;
use tonic::Request;

pub mod data;

struct InputStream {
    room_id: i32,
    name: String,
    current: String,
}

impl InputStream {
    fn new(room_id: i32, name: String) -> Self {
        InputStream {
            room_id,
            name,
            current: String::new(),
        }
    }
}

impl Iterator for InputStream {
    type Item = MessageRequest;

    fn next(&mut self) -> Option<Self::Item> {
        let stdin = stdin();
        let mut stdout = stdout();
        let mut input = String::new();
        execute!(
            stdout,
            Clear(crossterm::terminal::ClearType::CurrentLine),
            MoveToColumn(0)
        )
        .unwrap();
        print!("Say: ");
        if let Err(_) = stdout.flush() {
            return None;
        }
        stdin.read_line(&mut input).expect("Invalid input");
        execute!(
            stdout,
            MoveUp(1),
            Clear(crossterm::terminal::ClearType::CurrentLine),
        )
        .unwrap();
        self.current = input.trim().to_owned();
        if &self.current == "q" {
            return None;
        }
        Some(MessageRequest {
            room_id: self.room_id,
            name: self.name.clone(),
            body: self.current.clone(),
        })
    }
}

async fn join(client: &mut ChatClient<Channel>, name: String) -> Result<i32, Box<dyn Error>> {
    let reply = client.join(JoinRequest { name }).await?.into_inner();

    Ok(reply.room_id)
}
async fn get_message(
    client: &mut ChatClient<Channel>,
    room_id: i32,
    name: String,
) -> Result<(), Box<dyn Error>> {
    let mut stream = client
        .get_message(Request::new(ChatRequest { room_id, name }))
        .await?
        .into_inner();
    while let Some(msg) = stream.message().await? {
        let mut stdout = stdout();
        execute!(
            stdout,
            Clear(crossterm::terminal::ClearType::CurrentLine),
            MoveToColumn(0)
        )?;
        println!("{}: {}", msg.name, msg.body);
        print!("Say: ");
        stdout.flush()?;
    }
    Ok(())
}

async fn send_message(
    client: &mut ChatClient<Channel>,
    room_id: i32,
    name: String,
    sigterm_rx: Receiver<()>,
) -> Result<(), Box<dyn Error>> {
    let mut stream = InputStream::new(room_id, name);
    while let Some(msg) = stream.next() {
        if let Ok(()) = sigterm_rx.try_recv() {
            break;
        }
        client.send_message(Request::new(msg)).await?;
    }
    Ok(())
}

async fn disconnect(
    client: &mut ChatClient<Channel>,
    room_id: i32,
    name: String,
) -> Result<(), Box<dyn Error>> {
    client.disconnect(ChatRequest { name, room_id }).await?;
    Ok(())
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    name: Option<String>,

    #[arg(short, long)]
    port: Option<String>,

    #[arg(short, long, env)]
    cert: Option<String>,

    #[arg(long, default_value_t = false)]
    tls: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let config_path = ProjectDirs::from("net", "faizud", "chat").expect("Invalid config path");
    let config_path = config_path.config_dir();
    tokio::fs::create_dir_all(config_path).await?;
    let args = Args::parse();
    let (sigterm_tx, sigterm_rx) = mpsc::channel();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        sigterm_tx.send(()).unwrap();
        println!("Received Ctrl-c, press enter to exit")
    });

    let domain = args.name.unwrap_or("localhost".to_string());
    let protocol = if args.tls { "https" } else { "http" };
    let port = if let Some(port) = args.port {
        format!(":{}", port)
    } else {
        String::from("")
    };
    let server_url = format!("{}://{}{}", protocol, domain, port);

    let mut client = if args.tls {
        let ca_path = config_path.join("ca.pem");
        let pem = tokio::fs::read(&ca_path).await.expect(&format!(
            "Certificate file not found: {}",
            ca_path.to_string_lossy()
        ));
        let ca = Certificate::from_pem(pem);
        let tls = ClientTlsConfig::new()
            .ca_certificate(ca)
            .domain_name(domain);
        let channel = Channel::from_shared(server_url.clone())?
            .tls_config(tls)?
            .connect()
            .await?;
        ChatClient::new(channel)
    } else {
        ChatClient::connect(server_url.clone()).await?
    };

    println!("Connected to {}", &server_url);
    let stdin = stdin();
    let mut name = String::new();
    print!("Enter your name: ");
    stdout().flush()?;
    stdin.read_line(&mut name).expect("Invalid input");
    if let Ok(()) = sigterm_rx.try_recv() {
        return Ok(());
    }
    let name = name.trim().to_owned();
    let room_id = join(&mut client, name.clone()).await?;
    let mut recv_client = client.clone();
    let recv_name = name.clone();
    tokio::spawn(async move {
        get_message(&mut recv_client, room_id.clone(), recv_name)
            .await
            .expect("Disconnected from server");
    });
    send_message(&mut client, room_id.clone(), name.clone(), sigterm_rx).await?;
    disconnect(&mut client, room_id, name).await?;
    Ok(())
}
