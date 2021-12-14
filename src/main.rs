mod structs;
mod tools;

#[macro_use]
extern crate actix_web;
use std::collections::{BinaryHeap, HashMap};
use std::io;
use std::sync::Mutex;
use std::thread::{sleep, spawn};
use std::time::Duration;

use crate::structs::Value;
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer, Responder};

#[get("/")]
async fn index(map: web::Data<Mutex<HashMap<String, structs::Value>>>) -> impl Responder {
    let mut r = String::from("<a href=\"/help\">help</a>");
    r.push_str(r#"
<form id="f" method="post">
    key: <input id="k">
    value: <textarea  id="v"></textarea>
    <span onclick=s()>submmit</span>
</form>
<script>
function s(){
    let k = (document.getElementById("k").value)
    let v = (document.getElementById("v").value)
    var xhr = new XMLHttpRequest();
    xhr.open("POST", "http://localhost:7259/"+k, true);
    xhr.onreadystatechange = function () {
        if (this.readyState != 4) return;

        if (this.status == 200) {
            location.reload()
        }else{
            alert("err")
        }
    };
    xhr.send(v)
}
</script>
    "#);
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
        .body(format!(
            r#"
<ol>
    <li> 存入k,v<br><code>curl -X POST -d "[value]" [server]/[key] </code></li>
    <li> 获取k <br><code> curl [server]/[key]</code></li>
    <li> 可以获取的次数默认1 <br> 可选项  times int  </li>
    <li> 保存的分钟 默认1分钟 <br> 可选项 minutes int </li>
    <li> 是否再首页列表显示 <br> 可选项 private 任意string </li>
    <li> demo <br> curl -X POST -d "abcdefg" "localhost:7259/abc?times=2&private=a" </li>
</ol>
        "#
        ))
}
#[post("/{key}")]
async fn put(
    web::Path(key): web::Path<String>,
    map: web::Data<Mutex<HashMap<String, structs::Value>>>,
    queue: web::Data<Mutex<BinaryHeap<structs::StructInDeleteQueue>>>,
    req: HttpRequest,
    value: String,
) -> impl Responder {
    if key.len() > 20 {
        return "key too long";
    }
    let mut locked_map = map.lock().unwrap();
    if locked_map.contains_key(&key) {
        return "key exists";
    }
    let create_time = tools::now_timestamps();
    let mut v = Value::new(&value, create_time);
    let params = web::Query::<structs::Params>::from_query(req.query_string()).unwrap();
    if let Some(t) = params.times {
        v.times = t;
    }
    let delete_time = if let Some(t) = params.minutes {
        println!("minutes = {}",t);
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
    let r: HttpResponse;
    let mut times = -1;
    let mut locked_map = map.lock().unwrap();
    if let Some(v) = locked_map.get_mut(&key) {
        v.times -= 1;
        times = v.times;
        r = HttpResponse::Ok()
            .content_type("text/plain")
            .body(format!("{}", &v.value))
    } else {
        r = HttpResponse::Ok().content_type("text/plain").body("")
    }
    if 0 == times {
        locked_map.remove(&key);
    }
    r
}
#[actix_web::main]
async fn main() -> io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info");
    let map = web::Data::new(Mutex::new(HashMap::<String, structs::Value>::new()));
    let delete_queue =
        web::Data::new(Mutex::new(BinaryHeap::<structs::StructInDeleteQueue>::new()));
    delete_expired_thread(delete_queue.clone(), map.clone());
    HttpServer::new(move || {
        App::new()
            .app_data(map.clone())
            .app_data(delete_queue.clone())
            .wrap(middleware::Logger::default())
            .service(index)
            .service(help)
            .service(put)
            .service(get)
    })
    .bind("0.0.0.0:7259")?
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
                    sleep(Duration::from_secs(v.delete_time-now));
                }
            } else {
                let mut locked_queue = queue.lock().unwrap();
                cur = locked_queue.pop();
                drop(locked_queue);
                if cur.is_none(){
                    sleep(Duration::from_secs(60));
                }
            }
        }
    });
}
