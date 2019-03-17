extern crate reqwest;
extern crate websocket;

use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

use reqwest::header::CONTENT_TYPE;

use websocket::client::ClientBuilder;
use websocket::OwnedMessage;

const CONNECTION: &'static str = "ws://192.168.50.3:12749";
//const CONNECTION: &'static str = "ws://127.0.0.1:12749";

fn main() {
    println!("Connecting to {}", CONNECTION);

    let http_client = reqwest::Client::new();
    let ws_client = ClientBuilder::new(CONNECTION)
        .unwrap()
        .add_protocol("rust-websocket")
        .connect_insecure()
        .unwrap();

    println!("Successfully connected");

    let (mut receiver, mut sender) = ws_client.split().unwrap();

    let (tx, rx) = channel();

    let tx_1 = tx.clone();

    let send_loop = thread::spawn(move || {
        loop {
            // Send loop
            let message = match rx.recv() {
                Ok(m) => m,
                Err(e) => {
                    println!("Send Loop: {:?}", e);
                    continue;
                }
            };
            match message {
                OwnedMessage::Close(_) => {
                    let _ = sender.send_message(&message);
                    // If it's a close message, just send it and then return.
                    return;
                }
                _ => (),
            }

            println!("Sending Loop: {:?}", message);

            // Send the message
            match sender.send_message(&message) {
                Ok(()) => (),
                Err(e) => {
                    println!("Send Loop: {:?}", e);
                    //let _ = sender.send_message(&Message::close());
                    continue;
                }
            }
        }
    });

    let receive_loop = thread::spawn(move || {
        // Receive loop
        for message in receiver.incoming_messages() {
            let message = match message {
                Ok(m) => m,
                Err(e) => {
                    println!("Receive Loop: {:?}", e);
                    //let _ = tx_1.send(OwnedMessage::Close(None));
                    continue;
                }
            };
            match message {
                OwnedMessage::Close(_) => {
                    // Got a close message, so send a close message and return
                    let _ = tx_1.send(OwnedMessage::Close(None));
                    return;
                }
                OwnedMessage::Ping(data) => {
                    match tx_1.send(OwnedMessage::Pong(data)) {
                        // Send a pong in response
                        Ok(()) => (),
                        Err(e) => {
                            println!("Receive Loop: {:?}", e);
                        }
                    }
                }
                OwnedMessage::Text(text) => {
                    println!("Receive Loop text: {:?}", text);
                    if text.starts_with("![k[alive],") {
                        match tx_1.send(OwnedMessage::Text("![k[im alive],v[null]]!".to_string())) {
                            Ok(()) => (),
                            Err(e) => println!("Alive: {:?}", e),
                        }
                    } else if text.starts_with("![k[bracket],") {
                        let (_, last) = text.split_at("![k[bracket],v[".len());
                        let (first, _) = last.split_at(last.len() - 3);
                        println!("Bracket: {:?}", first);
                        let result = http_client
                            .post("http://killer-queen-chattanooga.herokuapp.com/api/cab/bracket")
                            //.post("http://localhost:5000/api/cab/bracket")
                            .body(first.to_owned())
                            .header(CONTENT_TYPE, "application/json")
                            .send();

                        match result {
                            Ok(mut j) => match j.text() {
                                Ok(resp) => println!("Resp {:#?}", resp),
                                Err(e) => println!("some json error i dunno {:?}", e),
                            },
                            Err(e) => println!("some http error i dunno {:?}", e),
                        }
                    } else if text.starts_with("![k[goldonleft],") {
                        let gold_on_left = if text.contains("True") {
                            "true"
                        } else {
                            "false"
                        };
                        println!("GoldOnLeft: {:?}", gold_on_left);
                        let result = http_client
                            .post(
                                "http://killer-queen-chattanooga.herokuapp.com/api/cab/goldonleft",
                            )
                            //.post("http://localhost:5000/api/cab/goldonleft")
                            .body(gold_on_left)
                            .header(CONTENT_TYPE, "application/json")
                            .send();

                        match result {
                            Ok(mut j) => match j.text() {
                                Ok(resp) => println!("Resp {:#?}", resp),
                                Err(e) => println!("some json error i dunno {:?}", e),
                            },
                            Err(e) => println!("some http error i dunno {:?}", e),
                        }
                    }
                }
                // Say what we received
                _ => println!("Receive Loop: {:?}", message),
            }
        }
    });

    match tx.send(OwnedMessage::Text(
        "![k[connect],v[{\"name\":\"null\",\"isGameMachine\":false}]]!".to_string(),
    )) {
        Ok(()) => loop {
            let _ = tx.send(OwnedMessage::Text("![k[get],v[goldonleft]]!".to_string()));
            thread::sleep(Duration::from_secs(1));
            let _ = tx.send(OwnedMessage::Text("![k[get],v[bracket]]!".to_string()));
            thread::sleep(Duration::from_secs(30));
        },
        Err(e) => {
            println!("Main Loop: {:?}", e);
        }
    }

    // We're exiting

    println!("Waiting for child threads to exit");

    let _ = send_loop.join();
    let _ = receive_loop.join();

    thread::sleep(Duration::from_secs(60));
    println!("Exited");
}
