use crate::structs;
use actix_web::web;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Mutex;
use std::thread::spawn;

pub(crate) fn tcp_listener(map: web::Data<Mutex<HashMap<String, structs::Value>>>, tcp_port: i32) {
    spawn(move || {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", tcp_port)).unwrap();

        // accept connections and process them serially
        for stream in listener.incoming() {
            match stream {
                Ok(s) => {
                    handle_client(s, &map);
                }
                Err(e) => {
                    println!("tcp err:{:?}", e);
                }
            };
        }
    });
}
fn handle_client(mut stream: TcpStream, map: &web::Data<Mutex<HashMap<String, structs::Value>>>) {
    let mut key = String::new();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    stream.write_all(b" choose delete key.").unwrap();
    stream.flush().unwrap();
    while reader.read_line(&mut key).is_ok() {
        let key_trim = key.trim();
        let mut lock_map = map.lock().unwrap();
        if key_trim == "clearall" {
            stream.write_all(b"clear all\n").unwrap();
            lock_map.clear();
        } else if lock_map.remove(key_trim).is_some() {
            stream.write_all(b"ok\n").unwrap();
        } else {
            stream.write_all(b"none\n").unwrap();
        }

        drop(lock_map);
        key.clear();
    }
    println!("done");
}
