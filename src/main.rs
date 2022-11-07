use dotenv::dotenv;
use regex::Regex;
use secp256k1::rand::rngs::OsRng;
use secp256k1::{KeyPair, PublicKey, Secp256k1, SecretKey};
use std::env;
use std::net::TcpStream;
use tungstenite::stream::MaybeTlsStream;

pub mod lightning;

use anyhow::Result;
use nostr::{ClientMessage, Event, RelayMessage, SubscriptionFilter};
use tungstenite::{connect, Message as WsMessage, WebSocket};
use url::Url;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();
    let bot_secret = env::var("BOT_PRIVATE_KEY").expect("BOT_PRIVATE_KEY must be set");
    let relay = env::var("RELAY").expect("RELAY must be set");

    let keypair = keypair_from_secret(&bot_secret);
    start(keypair, &relay).await?;

    Ok(())
}

async fn handle_event(event: Box<Event>) {
    let user_input = parse_user_input(&event.content);

    if user_input.get(0) == Some(&"!invoice") {
        let invoice = lightning::create_invoice(&event.content, 888)
            .await
            .unwrap();

        println!("invoice: {}", invoice.payment_request);
    }
}

fn generate_keys() -> Result<(SecretKey, PublicKey)> {
    let secp = Secp256k1::new();
    Ok(secp.generate_keypair(&mut OsRng))
}

/// Returns keypair parsed from string
fn keypair_from_secret(secret: &str) -> secp256k1::KeyPair {
    let secp = secp256k1::Secp256k1::new();
    secp256k1::KeyPair::from_seckey_str(&secp, secret).unwrap()
}

fn parse_user_input(s: &str) -> Vec<&str> {
    let re = Regex::new(r#"["]([^"]*)["]|([^" ]+)"#).unwrap();
    let mut input: Vec<&str> = re
        .captures_iter(s)
        .map(|cap| {
            if let Some(quoted) = cap.get(1) {
                quoted
            } else {
                cap.get(2).unwrap()
            }
            .as_str()
        })
        .collect();

    if let Some(el) = input.get(0) {
        if el == &"#[0]" {
            input.remove(0);
        }
    }

    input
}

fn connect_relay(relay: &str) -> Result<WebSocket<MaybeTlsStream<TcpStream>>> {
    let url = Url::parse(relay)?;
    let (socket, _) = connect(url).expect("Can't connect");

    println!("Connected to the server");
    Ok(socket)
}

async fn start(keys: KeyPair, relay: &str) -> Result<()> {
    // we use the same function to generate a unique number which is out subscription_id
    let subscription_id = generate_keys().unwrap().0.display_secret().to_string();
    println!("subscription_id: {subscription_id}");

    let subscribe_to_bot = ClientMessage::new_req(
        subscription_id,
        vec![SubscriptionFilter::new().pubkey(keys.x_only_public_key().0)],
    );

    let mut socket = connect_relay(relay).unwrap();

    socket.write_message(WsMessage::Text(subscribe_to_bot.to_json()))?;

    loop {
        let msg = socket.read_message().expect("Error reading message");
        let msg_text = msg.to_text().expect("Failed to conver message to text");
        if let Ok(handled_message) = RelayMessage::from_json(msg_text) {
            match handled_message {
                RelayMessage::Empty => {
                    println!("Empty message")
                }
                RelayMessage::Notice { message } => {
                    println!("Got a notice: {}", message);
                }
                RelayMessage::Event {
                    event: e,
                    subscription_id: _,
                } => {
                    println!("EVENT: {e:?}");
                    handle_event(e).await;
                }
                RelayMessage::EndOfStoredEvents { subscription_id: i } => {
                    println!("Relay signalled End of Stored Events, sid: {i}");
                }
            }
        } else {
            println!("Got unexpected message: {}", msg_text);
        }
    }
}
