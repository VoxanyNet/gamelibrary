
use std::{fs, time::Instant};

use diff::Diff;
use ewebsock::{WsReceiver, WsSender};
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use macroquad::input::{is_key_down, KeyCode};
use serde::{de::DeserializeOwned, Serialize};

use crate::log;

pub struct SyncClient<T: Serialize + DeserializeOwned + Diff + Clone + PartialEq> {
    previous_state: T,
    server_send: WsSender,
    server_receive: WsReceiver

}

impl<T> SyncClient<T>
where 
    T: Serialize + DeserializeOwned + Diff + Clone + PartialEq,
    <T as Diff>::Repr: DeserializeOwned + Serialize {
    
    pub async fn connect(url: &str) -> (Self, T) {

    
        let (server_send, server_receive) = match ewebsock::connect(url, ewebsock::Options::default()) {
            Ok(result) => result,
            Err(error) => {
                panic!("failed to connect to server: {}", error)
            },
        };

        // wait for Opened event from server
        loop {
            match server_receive.try_recv() {
                Some(event) => {
                    match event {
                        ewebsock::WsEvent::Opened => {
                            println!("we got the opened message!");
                            break;
                        },
                        ewebsock::WsEvent::Message(message) => {
                            match message {
                                _ => panic!("received a message from the server")
                            }
                        },
                        ewebsock::WsEvent::Error(error) => panic!("received error when trying to connect to server: {}", error),
                        ewebsock::WsEvent::Closed => panic!("server closed when trying to connect"),
                        
                    }
                },
                None => {
                    log("Waiting for open message");
                    
                    macroquad::window::next_frame().await; // let js runtime main thread continue execution while we wait

                    continue;
                },
            }
        };

        // wait for initial state
        let compressed_state_bytes = loop {

            match server_receive.try_recv() {
                Some(event) => {
                    match event {
                        ewebsock::WsEvent::Opened => todo!("unhandled opened event on connect"),
                        ewebsock::WsEvent::Message(message) => {
                            match message {
                                ewebsock::WsMessage::Binary(bytes) => break bytes,
                                _ => todo!("unhandled message type when receiving initial state")
                            }
                        },
                        ewebsock::WsEvent::Error(error) => todo!("unhandled error when receiving initial state: {}", error),
                        ewebsock::WsEvent::Closed => todo!("unhandled closed event when receiving initial state"),
                    }
                },
                None => {
                    macroquad::window::next_frame().await;
                    continue;
                }, // this means that the server would have blocked, so we try again
            };
        };
        
        let state_bytes = decompress_size_prepended(&compressed_state_bytes).expect("Failed to decompress initial state");

        let state: T = match bitcode::deserialize(&state_bytes) {
            Ok(state) => state,
            Err(error) => {
                panic!("failed to deserialize initial state: {}", error);
            },
        };

        return (
            Self {
                previous_state: state.clone(),
                server_receive,
                server_send
            },

            state
        )


    }
    pub fn sync(&mut self, state: &mut T) {
        
        // send & receive state updates
        self.send_update(state);
        
        self.receive_updates(state);
       

        self.previous_state = state.clone();
    }
    
    fn send_update(&mut self, state: &T) {

        if self.previous_state == *state {
            return;
        }

        let state_diff = self.previous_state.diff(&state);

        if is_key_down(KeyCode::M) {
            fs::write("diff.yaml", serde_yaml::to_string(&state_diff).unwrap()).unwrap();
            fs::write("diff.bin", bitcode::serialize(&state_diff).unwrap()).unwrap();
            fs::write("diff.json", serde_json::to_string_pretty(&state_diff).unwrap()).unwrap();
        }



        let diff_bytes = bitcode::serialize(&state_diff).expect("failed to serialize state diff");
        
        let compressed_diff_bytes = compress_prepend_size(&diff_bytes);
        
        self.server_send.send(
            ewebsock::WsMessage::Binary(
                compressed_diff_bytes.to_vec()
            )
        );
        
    }

    fn receive_updates(&mut self, state: &mut T) {
        // we loop until there are no new updates
        loop {

            let compressed_state_diff_bytes = match self.server_receive.try_recv() {
                Some(event) => {
                    match event {
                        ewebsock::WsEvent::Opened => todo!("unhandled 'Opened' event"),
                        ewebsock::WsEvent::Message(message) => {
                            match message {
                                ewebsock::WsMessage::Binary(bytes) => bytes,
                                _ => todo!("unhandled message type when trying to receive updates from server")
                            }
                        },
                        ewebsock::WsEvent::Error(error) => {

                            // this is stupid
                            if error.contains("A non-blocking socket operation could not be completed immediately)") {
                                println!("fortnite");

                                // attempt to receive again if blocking
                                continue;
                            }
                            todo!("unhandled 'Error' event when trying to receive update from server: {}", error)
                        },
                        ewebsock::WsEvent::Closed => todo!("server closed"),
                    }
                },
                None => break, // this means there are no more updates
            };
            
            let state_diff_bytes = decompress_size_prepended(&compressed_state_diff_bytes).expect("Failed to decompress incoming update");

            let state_diff: <T as Diff>::Repr = match bitcode::deserialize(&state_diff_bytes) {
                Ok(state_diff) => state_diff,
                Err(error) => {
                    panic!("failed to deserialize game state diff: {}", error);
                },
            };

            state.apply(&state_diff); 
        }
    }
}