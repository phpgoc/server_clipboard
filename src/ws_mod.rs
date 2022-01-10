use actix::prelude::*;
use rand::{self, rngs::ThreadRng, Rng};

use std::collections::{BinaryHeap, HashMap};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::structs::WsResponse;
use crate::{structs, tools, Value};
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;

#[derive(Message)]
#[rtype(result = "()")]
pub struct Message(pub String);

#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<Message>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: usize,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ClientMessage {
    pub id: usize,
    pub msg: String,
    pub room: String,
}

pub struct ListRooms;

impl actix::Message for ListRooms {
    type Result = Vec<String>;
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Join {
    pub(crate) id: usize,
    pub(crate) name: String,
    pub(crate) times: i32,
    pub(crate) minutes: u64,
}

pub struct WsServer {
    sessions: HashMap<usize, (Recipient<Message>, String)>,
    rooms: HashMap<String, (i32, u64, Vec<usize>)>,
    map: web::Data<Mutex<HashMap<String, structs::Value>>>,
    queue: web::Data<Mutex<BinaryHeap<structs::StructInDeleteQueue>>>,
    rng: ThreadRng,
}

impl WsServer {
    pub(crate) fn new(
        map: web::Data<Mutex<HashMap<String, structs::Value>>>,
        queue: web::Data<Mutex<BinaryHeap<structs::StructInDeleteQueue>>>,
    ) -> WsServer {
        let rooms = HashMap::new();

        WsServer {
            sessions: HashMap::new(),
            rooms,
            map,
            queue,
            rng: rand::thread_rng(),
        }
    }
}

impl WsServer {
    fn send_join_message(&self, room: &str, websocket_response: WsResponse) {
        if let Some((_, _, session_vec)) = self.rooms.get(room) {
            let result_string = serde_json::to_string(&websocket_response).unwrap();

            for id in session_vec {
                if let Some((address, _)) = self.sessions.get(id) {
                    let _ = address.do_send(Message(result_string.clone()));
                }
            }
        }
    }

    fn send_message(&self, room: &str, mut websocket_response: WsResponse, skip_id: usize) {
        let mut locked_map = self.map.lock().unwrap();
        if let Some(v) = locked_map.get(room) {
            //key 没有被消耗光
            if let Some((addr, _)) = self.sessions.get(&skip_id) {
                websocket_response.result = Some("remaining is not zero".to_string());
                websocket_response.remaining = Some(v.times);
                let _ = addr.do_send(Message(serde_json::to_string(&websocket_response).unwrap()));
            }
            return;
        } else {
            websocket_response.result = Some("".to_string());
        }
        if let Some((mut times, minutes, sessions)) = self.rooms.get(room) {
            let mut address_vec = vec![];
            for id in sessions {
                if *id != skip_id {
                    if let Some((address, _)) = self.sessions.get(id) {
                        // let _ = addr.do_send(Message(message.to_owned()));
                        address_vec.push(address);
                        times -= 1;
                        if times == 0 {
                            break;
                        }
                    }
                }
            }
            websocket_response.remaining = Some(times);
            let result_string = serde_json::to_string(&websocket_response).unwrap();
            for i in address_vec {
                let _ = i.do_send(Message(result_string.clone()));
            }

            if 0 != times {
                let create_time = tools::now_timestamps();
                let mut val = Value::new(websocket_response.message.unwrap(), create_time);
                val.times = times;
                locked_map.insert(String::from(room), val);
                let delete_struct = structs::StructInDeleteQueue::new(
                    create_time + 60 * minutes,
                    create_time,
                    String::from(room),
                );
                let mut locked_queue = self.queue.lock().unwrap();
                locked_queue.push(delete_struct);
            }
            if let Some((address, _)) = self.sessions.get(&skip_id) {
                websocket_response.message = None;
                websocket_response.result = Some("ok".to_string());
                let _ =
                    address.do_send(Message(serde_json::to_string(&websocket_response).unwrap()));
            }
        }
    }
}
impl Actor for WsServer {
    type Context = Context<Self>;
}

impl Handler<Connect> for WsServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        let id = self.rng.gen::<usize>();
        self.sessions.insert(id, (msg.addr, String::from("")));
        id
    }
}

impl Handler<Disconnect> for WsServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        if let Some((_, room)) = self.sessions.remove(&msg.id) {
            let vec_len = if let Some(tup) = self.rooms.get_mut(&room) {
                let index = (*tup).2.iter().position(|x| *x == msg.id).unwrap();
                tup.2.remove(index);
                tup.2.len()
            } else {
                1
            };
            if 0 == vec_len {
                self.rooms.remove(&room);
            } else {
                let res = WsResponse {
                    times: None,
                    minutes: None,
                    message: None,
                    remaining: None,
                    total: Some(vec_len),
                    result: None,
                };

                self.send_join_message(&room, res);
            }
        }
    }
}

impl Handler<ClientMessage> for WsServer {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, _: &mut Context<Self>) {
        self.send_message(
            &msg.room,
            WsResponse {
                times: None,
                minutes: None,
                message: Some(msg.msg),
                remaining: None,
                total: None,
                result: None,
            },
            msg.id,
        );
    }
}

impl Handler<Join> for WsServer {
    type Result = ();

    fn handle(&mut self, msg: Join, _: &mut Context<Self>) {
        let Join {
            id,
            name,
            times,
            minutes,
        } = msg;
        let cur_room = self
            .rooms
            .entry(name.clone())
            .or_insert((times, minutes, Vec::new()));
        cur_room.2.push(id);
        let (_, room) = self.sessions.get_mut(&id).unwrap();
        *room = name.clone();
        let res = WsResponse {
            times: Some((*cur_room).0),
            minutes: Some((*cur_room).1),
            message: None,
            remaining: None,
            total: Some(cur_room.2.len()),
            result: None,
        };

        self.send_join_message(&name, res);
    }
}
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) async fn chat_route(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<WsServer>>,
) -> Result<HttpResponse, Error> {
    ws::start(
        WsChatSession {
            id: 0,
            hb: Instant::now(),
            room: "".to_owned(),
            name: None,
            address: srv.get_ref().clone(),
        },
        &req,
        stream,
    )
}

struct WsChatSession {
    id: usize,
    hb: Instant,
    room: String,
    name: Option<String>,
    address: Addr<WsServer>,
}

impl Actor for WsChatSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
        let addr = ctx.address();
        self.address
            .send(Connect {
                addr: addr.recipient(),
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(res) => act.id = res,
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        let _ = self.address.do_send(Disconnect { id: self.id });
        Running::Stop
    }
}

impl Handler<Message> for WsChatSession {
    type Result = ();
    fn handle(&mut self, msg: Message, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsChatSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let msg = match msg {
            Err(_) => {
                ctx.stop();
                return;
            }
            Ok(msg) => msg,
        };

        match msg {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.hb = Instant::now();
            }
            ws::Message::Text(text) => {
                let m = text.trim();
                if m.starts_with("/join") {
                    let v: Vec<&str> = m.splitn(3, ' ').collect();

                    if v.len() >= 2 {
                        self.room = v[1][1..].to_owned();
                        let mut times = 1;
                        let mut minutes = 1;
                        if v.len() == 3 {
                            if let Ok(params) =
                                web::Query::<structs::Params>::from_query(&v[2][1..])
                            {
                                if let Some(t) = params.times {
                                    times = times.max(t);
                                }
                                if let Some(mut t) = params.minutes {
                                    t = t.min(60 * 24 * 7);
                                    minutes = t;
                                }
                            }
                        }
                        let _ = self.address.do_send(Join {
                            id: self.id,
                            name: self.room.clone(),
                            times,
                            minutes,
                        });
                    } else {
                        ctx.text("!!! room name is required");
                    }
                } else {
                    let msg = if let Some(ref name) = self.name {
                        format!("{}: {}", name, m)
                    } else {
                        m.to_owned()
                    };
                    // send message to chat server
                    self.address.do_send(ClientMessage {
                        id: self.id,
                        msg,
                        room: self.room.clone(),
                    })
                }
            }
            ws::Message::Binary(_) => println!("Unexpected binary"),
            ws::Message::Close(reason) => {
                ctx.close(reason);
                ctx.stop();
            }
            ws::Message::Continuation(_) => {
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}

impl WsChatSession {
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                println!(
                    "Websocket Client heartbeat failed, disconnecting!id = {}.",
                    act.id
                );
                let _ = act.address.do_send(Disconnect { id: act.id });
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }
}
