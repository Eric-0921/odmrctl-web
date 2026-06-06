//! Actix-web HTTP server exposing trace and stats endpoints.

use crate::buffer::TraceRingBuffer;
use crate::types::{CollectorStatus, TraceSnapshot};
use actix_cors::Cors;
use actix_web::{get, web, App, HttpResponse, HttpServer};
use std::sync::{Arc, Mutex};

pub type SharedState = Arc<Mutex<TraceRingBuffer>>;

#[get("/api/trace")]
async fn trace_handler(state: web::Data<SharedState>) -> HttpResponse {
    let buf = state.lock().unwrap();
    let snapshot = TraceSnapshot {
        points: buf.snapshot(),
        frames_total: buf.frames_total(),
        frames_unique: buf.frames_unique(),
        dup_rate: buf.dup_rate(),
        avg_read_us: buf.avg_read_us(),
    };
    HttpResponse::Ok().json(snapshot)
}

#[get("/api/stats")]
async fn stats_handler(state: web::Data<SharedState>) -> HttpResponse {
    let buf = state.lock().unwrap();
    let status = CollectorStatus {
        frames_captured: buf.frames_total(),
        frames_duplicated: buf.frames_total() - buf.frames_unique(),
        frames_parse_error: 0,
        total_reads_attempted: buf.frames_total(),
        avg_read_time_us: buf.avg_read_us(),
        running: true,
    };
    HttpResponse::Ok().json(status)
}

pub fn run_server(state: SharedState, http_port: u16) -> std::io::Result<()> {
    let state_data = web::Data::new(state);
    let rt = actix_web::rt::System::new();
    rt.block_on(async {
        HttpServer::new(move || {
            let cors = Cors::default()
                .allow_any_origin()
                .allow_any_method()
                .allow_any_header()
                .max_age(3600);
            App::new()
                .wrap(cors)
                .app_data(state_data.clone())
                .service(trace_handler)
                .service(stats_handler)
        })
        .bind(("127.0.0.1", http_port))?
        .run()
        .await
    })
}
