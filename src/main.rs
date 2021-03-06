mod conf;
mod html;
mod structs;
mod tcp_listener;
mod tools;
mod ws_mod;

#[macro_use]
extern crate actix_web;
#[macro_use]
extern crate clap;
use crate::structs::Value;
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer, Responder};

use crate::conf::DEFAULT_TCP_PORT;
use actix::Actor;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use std::collections::{BinaryHeap, HashMap};
use std::io;
use std::sync::Mutex;
use std::thread::{sleep, spawn};
use std::time::Duration;

#[get("/")]
async fn index(map: web::Data<Mutex<HashMap<String, structs::Value>>>) -> impl Responder {
    let mut response_str = String::from("<a href=\"/help\">help</a>");
    response_str.push_str(html::INDEX);
    let locked_map = map.lock().unwrap();
    if locked_map.len() != 0 {
        response_str.push_str("<br>list:<ul>");
        for (key, value) in locked_map.iter() {
            if value.public {
                response_str.push_str(&*format!("<li><a href=\"/{}\">{}</a></li>", key, key));
            }
        }
        response_str.push_str("</ul>");
    }
    HttpResponse::Ok()
        .content_type("text/html")
        .body(response_str)
}
#[get("/help")]
async fn help() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html;charset=utf-8")
        .body(html::HELP)
}
#[post("/{key}")]
async fn put(
    web::Path(key): web::Path<String>,
    map: web::Data<Mutex<HashMap<String, structs::Value>>>,
    queue: web::Data<Mutex<BinaryHeap<structs::StructInDeleteQueue>>>,
    req: HttpRequest,
    value: String,
) -> impl Responder {
    if key.len() > conf::KEY_SIZE {
        return "key too long";
    }
    if value.is_empty() {
        return "value is empty";
    }
    let mut locked_map = map.lock().unwrap();
    if locked_map.contains_key(&key) {
        return "key exists";
    }
    let create_time = tools::now_timestamps();
    let mut val = Value::new(&value, create_time);
    let params = match web::Query::<structs::Params>::from_query(req.query_string()) {
        Ok(t) => t,
        Err(e) => {
            println!("bad request: {:?}", e);
            return "bad request";
        }
    };
    if let Some(t) = params.times {
        val.times = 1.max(t);
    }
    let delete_time = if let Some(mut t) = params.minutes {
        t = t.min(60 * 24 * 7);
        create_time + t * 60
    } else {
        create_time + 60
    };
    if params.private.is_some() {
        val.public = false;
    }
    locked_map.insert(key.clone(), val);
    let delete_struct = structs::StructInDeleteQueue::new(delete_time, create_time, key);
    let mut locked_queue = queue.lock().unwrap();
    locked_queue.push(delete_struct);
    "ok"
}
#[get("/{key}")]
async fn get(
    web::Path(key): web::Path<String>,
    map: web::Data<Mutex<HashMap<String, structs::Value>>>,
    req: HttpRequest,
) -> impl Responder {
    if key.len() > conf::KEY_SIZE {
        return HttpResponse::Ok()
            .content_type("text/plain")
            .body("key too long");
    }
    let mut times = -1;
    let mut locked_map = map.lock().unwrap();

    let value = if let Some(val) = locked_map.get_mut(&key) {
        val.times -= 1;
        times = val.times;
        val.value.clone()
    } else {
        String::from("")
    };
    if 0 == times {
        locked_map.remove(&key);
    }
    if let Ok(params) = web::Query::<structs::Params>::from_query(req.query_string()) {
        if params.quiet.is_some() {
            return HttpResponse::Ok().content_type("text/plain").body(value);
        }
    };
    HttpResponse::Ok()
        .content_type("text/html")
        .body(html::GET.replace("{{}}", &*value))
}
#[actix_web::main]
async fn main() -> io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    let yaml = load_yaml!("cli.yml");
    let matches = clap::App::from_yaml(yaml).get_matches();
    let http_port = matches
        .value_of("http_port")
        .unwrap_or("")
        .parse::<i32>()
        .unwrap_or(conf::DEFAULT_HTTP_PORT);
    let tcp_port = matches
        .value_of("tcp_port")
        .unwrap_or("")
        .parse::<i32>()
        .unwrap_or(DEFAULT_TCP_PORT);
    let ssl_key = matches.value_of("key");
    let ssl_cert = matches.value_of("cert");

    let map = web::Data::new(Mutex::new(HashMap::<String, structs::Value>::new()));
    let delete_queue =
        web::Data::new(Mutex::new(BinaryHeap::<structs::StructInDeleteQueue>::new()));

    let ws_server = ws_mod::WsServer::new(map.clone(), delete_queue.clone()).start();

    delete_expired_thread(delete_queue.clone(), map.clone());
    tcp_listener::tcp_listener(map.clone(), tcp_port);
    let server = HttpServer::new(move || {
        App::new()
            .app_data(map.clone())
            .app_data(delete_queue.clone())
            .data(ws_server.clone())
            .wrap(middleware::Logger::default())
            .service(index)
            .service(help)
            .service(put)
            .service(web::resource("/ws/").to(ws_mod::chat_route))
            .service(get)
    });
    if ssl_key.is_some() && ssl_cert.is_some() {
        let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
        builder
            .set_private_key_file(ssl_key.unwrap(), SslFiletype::PEM)
            .unwrap();
        builder
            .set_certificate_chain_file(ssl_cert.unwrap())
            .unwrap();
        println!("Started https server: 0.0.0.0:{}", http_port);
        server
            .bind_openssl(format!("0.0.0.0:{}", http_port), builder)?
            .run()
            .await
    } else {
        println!("Started http server: 0.0.0.0:{}", http_port);
        server.bind(format!("0.0.0.0:{}", http_port))?.run().await
    }
}

fn delete_expired_thread(
    queue: web::Data<Mutex<BinaryHeap<structs::StructInDeleteQueue>>>,
    map: web::Data<Mutex<HashMap<String, structs::Value>>>,
) {
    spawn(move || {
        let mut cur: Option<structs::StructInDeleteQueue> = None;
        loop {
            if let Some(v) = &mut cur {
                let now = tools::now_timestamps();
                if v.delete_time > 60 + now {
                    let mut locked_queue = queue.lock().unwrap();
                    locked_queue.push(v.clone());
                    cur = None;
                    drop(locked_queue);
                } else if v.delete_time <= now {
                    let mut lock_map = map.lock().unwrap();
                    // ??????????????????????????????????????????times?????????????????????
                    let do_delete = if let Some(value) = lock_map.get(&v.key) {
                        value.create_time == v.create_time
                    } else {
                        false
                    };
                    if do_delete {
                        lock_map.remove(&v.key);
                    }
                    drop(lock_map);
                    let mut locked_queue = queue.lock().unwrap();
                    cur = locked_queue.pop();
                    drop(locked_queue);
                } else {
                    sleep(Duration::from_secs(v.delete_time - now));
                }
            } else {
                let mut locked_queue = queue.lock().unwrap();
                cur = locked_queue.pop();
                drop(locked_queue);
                if cur.is_none() {
                    sleep(Duration::from_secs(60));
                }
            }
        }
    });
}
