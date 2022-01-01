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

use actix::Actor;
use std::collections::{BinaryHeap, HashMap};
use std::io;
use std::sync::Mutex;
use std::thread::{sleep, spawn};
use std::time::Duration;

#[get("/")]
async fn index(map: web::Data<Mutex<HashMap<String, structs::Value>>>) -> impl Responder {
    let mut r = String::from("<a href=\"/help\">help</a>");
    r.push_str(html::INDEX);
    let locked_map = map.lock().unwrap();
    if locked_map.len() != 0 {
        r.push_str("<br>list:<ul>");
        for (k, v) in locked_map.iter() {
            if v.public {
                r.push_str(&*format!("<li><a href=\"/{}\">{}</a></li>", k, k));
            }
        }
        r.push_str("</ul>");
    }
    HttpResponse::Ok().content_type("text/html").body(r)
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
    if key.len() > 32 {
        return "key too long";
    }
    if value.len() == 0 {
        return "value is empty";
    }
    let mut locked_map = map.lock().unwrap();
    if locked_map.contains_key(&key) {
        return "key exists";
    }
    let create_time = tools::now_timestamps();
    let mut v = Value::new(&value, create_time);
    let params = match web::Query::<structs::Params>::from_query(req.query_string()) {
        Ok(t) => t,
        Err(e) => {
            println!("bad request: {:?}", e);
            return "bad request!";
        }
    };
    if let Some(t) = params.times {
        v.times = t;
    }
    let delete_time = if let Some(mut t) = params.minutes {
        t = t.min(60 * 24 * 7);
        create_time + t * 60
    } else {
        create_time + 60
    };
    if let Some(_) = params.private {
        v.public = false;
    }
    locked_map.insert(key.clone(), v);
    let delete_struct = structs::StructInDeleteQueue::new(delete_time, create_time, key);
    let mut q = queue.lock().unwrap();
    q.push(delete_struct);
    "ok"
}
#[get("/{key}")]
async fn get(
    web::Path(key): web::Path<String>,
    map: web::Data<Mutex<HashMap<String, structs::Value>>>,
) -> impl Responder {
    if key.len() > 50 {
        return HttpResponse::Ok()
            .content_type("text/plain")
            .body("key too long");
    }
    let mut times = -1;
    let mut locked_map = map.lock().unwrap();

    let before_value: String;
    if let Some(v) = locked_map.get_mut(&key) {
        v.times -= 1;
        times = v.times;
        before_value = v.value.clone();
    } else {
        before_value = String::from("");
    };
    if 0 == times {
        locked_map.remove(&key);
    }
    HttpResponse::Ok()
        .content_type("text/html")
        .body(html::GET.replace("{{}}", &*before_value))
}
#[actix_web::main]
async fn main() -> io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    let yaml = load_yaml!("cli.yml");
    let matches = clap::App::from_yaml(yaml).get_matches();
    let http_port = matches
        .value_of("http_port")
        .unwrap_or("7259")
        .parse::<i32>()
        .unwrap_or(7259);
    let tcp_port = matches
        .value_of("tcp_port")
        .unwrap_or("9527")
        .parse::<i32>()
        .unwrap_or(9527);
    let map = web::Data::new(Mutex::new(HashMap::<String, structs::Value>::new()));
    let delete_queue =
        web::Data::new(Mutex::new(BinaryHeap::<structs::StructInDeleteQueue>::new()));

    let ws_server = ws_mod::WsServer::new(map.clone(), delete_queue.clone()).start();

    delete_expired_thread(delete_queue.clone(), map.clone());
    tcp_listener::tcp_listener(map.clone(), tcp_port);
    HttpServer::new(move || {
        App::new()
            .app_data(map.clone())
            .app_data(delete_queue.clone())
            .data(ws_server.clone())
            .wrap(middleware::Logger::default())
            .service(index)
            .service(help)
            .service(put)
            .service(web::resource("/html").route(web::get().to(|| {
                HttpResponse::Ok()
                    .content_type("text/html")
                    .body(html::WS_HTML)
            })))
            .service(web::resource("/ws/").to(ws_mod::chat_route))
            .service(get)
    })
    .bind(format!("0.0.0.0:{}", http_port))?
    .run()
    .await
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
                    // 验证是否是要删除的那个，不是times到期之后再加的
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
