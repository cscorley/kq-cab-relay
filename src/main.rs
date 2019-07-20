#[macro_use]
extern crate log;
extern crate chrono;
extern crate clap;
extern crate fern;
extern crate reqwest;
extern crate websocket;

use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

use clap::{App, Arg};
use reqwest::header::CONTENT_TYPE;
use websocket::client::ClientBuilder;
use websocket::OwnedMessage;

const LOCALHOST: &'static str = "127.0.0.1";
const LOCALHOST_HTTP: &'static str = "http://127.0.0.1";

fn setup_logger() -> Result<(), fern::InitError> {
    let filename = chrono::Local::now().format("%Y-%m-%d.log").to_string();
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(fern::log_file(filename)?)
        .apply()?;
    Ok(())
}

fn main() {
    let matches = App::new("KQ Cab Relay")
        .version("0.1.0")
        .author("Christopher S. Corley <cscorley@gmail.com>")
        .about("Uploads stuff from a KQ cab to a server API")
        .arg(
            Arg::with_name("cab")
                .short("c")
                .long("cab")
                .value_name("CAB ADDRESS")
                .help("Sets address of the cab")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("destination")
                .short("d")
                .long("destination")
                .value_name("DESTINATION ADDRESS")
                .help("Sets address of the destination")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("score-destination")
                .short("s")
                .long("score-destination")
                .value_name("SCORE DESTINATION ADDRESS")
                .help("Sets address of the destination for scores")
                .takes_value(true),
        )
        .get_matches();

    match setup_logger() {
        Ok(_) => {}
        Err(e) => {
            println!("Got error during logging setup, exiting: {:?}", e);
            return;
        }
    };

    loop {
        let cab_address = matches.value_of("cab").unwrap_or(LOCALHOST).to_owned();
        let destination_address = matches
            .value_of("destination")
            .unwrap_or(LOCALHOST_HTTP)
            .to_owned();

        let score_destination_address = matches
            .value_of("score-destination")
            .unwrap_or(LOCALHOST_HTTP)
            .to_owned();

        println!(
            "Connecting to {}, submitting data to {}, {}",
            cab_address, destination_address, score_destination_address
        );

        let http_client = reqwest::Client::new();
        let ws_client = ClientBuilder::new(format!("ws://{}:12749", cab_address).as_str())
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
            let mut gold_on_left = false;

            // Receive loop
            for message in receiver.incoming_messages() {
                let message = match message {
                    Ok(m) => m,
                    Err(_) => {
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
                        let _ = tx_1.send(OwnedMessage::Pong(data));
                    }
                    OwnedMessage::Text(text) => {
                        info!("{}", text);
                        if text.starts_with("![k[alive],") {
                            // cab needs to know we're alive
                            let _ = tx_1
                                .send(OwnedMessage::Text("![k[im alive],v[null]]!".to_string()));
                        } else if text.starts_with("![k[bracket],") {
                            let (_, last) = text.split_at("![k[bracket],v[".len());
                            let (first, _) = last.split_at(last.len() - 3);
                            let _ = http_client
                                .post(format!("{}/api/cab/bracket", destination_address).as_str())
                                .body(first.to_owned())
                                .header(CONTENT_TYPE, "application/json")
                                .send();
                        } else if text.starts_with("![k[goldonleft],") {
                            gold_on_left = text.contains("True");
                            let _ = http_client
                                .post(
                                    format!("{}/api/cab/goldonleft", destination_address).as_str(),
                                )
                                .body(gold_on_left.to_string())
                                .header(CONTENT_TYPE, "application/json")
                                .send();
                        } else if text.starts_with("![k[victory],") {
                            let gold_win = text.contains("Gold");
                            let player_id = if gold_win {
                                if gold_on_left {
                                    0
                                } else {
                                    1
                                }
                            } else {
                                // blue win
                                if gold_on_left {
                                    1
                                } else {
                                    0
                                }
                            };
                            let _ = http_client
                                .post(
                                    format!(
                                        "{}/player/{}/increment-score",
                                        score_destination_address, player_id
                                    )
                                    .as_str(),
                                )
                                .send();
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
            Ok(()) => {
                let _ = tx.send(OwnedMessage::Text("![k[get],v[goldonleft]]!".to_string()));
                thread::sleep(Duration::from_secs(1));

                loop {
                    let _ = tx.send(OwnedMessage::Text("![k[get],v[bracket]]!".to_string()));
                    thread::sleep(Duration::from_secs(30));
                }
            }
            Err(e) => {
                println!("Main Loop error on connect: {:?}", e);
            }
        }

        // We're exiting

        println!("Waiting for child threads to exit");

        let _ = send_loop.join();
        let _ = receive_loop.join();

        println!("Reconnecting in 30 seconds");
        thread::sleep(Duration::from_secs(30));
    }
}
