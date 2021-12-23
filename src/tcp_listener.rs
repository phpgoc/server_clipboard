use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Mutex;
use std::thread::spawn;
use actix_web::web;
use crate::structs;

pub(crate) fn tcp_listener( map: web::Data<Mutex<HashMap<String, structs::Value>>>) {
    spawn(move ||{
        let listener = TcpListener::bind("127.0.0.1:9527").unwrap();

        // accept connections and process them serially
        for stream in listener.incoming() {
            match stream {
                Ok(s) => {handle_client(s,&map);}
                Err(e) => {println!("tcp err:{:?}",e);}
            };

        }
    });

}
fn handle_client(mut stream: TcpStream, map: & web::Data<Mutex<HashMap<String, structs::Value>>>) {
    let mut key = String::new();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    stream.write(b" choose delete key.").unwrap();
    stream.flush().unwrap();
    while  let Ok(_) = reader.read_line(&mut key)  {
        let key_trim = key.trim();
        let mut lock_map = map.lock().unwrap();
        if key_trim == "clearall" {
            stream.write(b"clear all\n").unwrap();
            lock_map.clear();
        }else{
            if let Some(_)  = lock_map.remove(key_trim){
                stream.write(b"ok\n").unwrap();
            }else{
                stream.write(b"none\n").unwrap();
            }
        }
        drop(lock_map);
        key.clear();
    }
    println!("done");
}