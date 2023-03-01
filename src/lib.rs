mod http;
mod filestore;

use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::{Arc, Mutex};

use std::borrow::BorrowMut;

use http::{HttpMethod, HttpRequest, HttpStatusCode};
use filestore::{FileCache};

use log::*;

use lazy_static::lazy_static;

static BIND_ADDR: &str = "127.0.0.1:8080";
static DOC_ROOT: &str = "./html/";
static DEFAULT_INDEX: &str = "index.html";
static _NOTFOUND_PAGE: &str = "404.html";

//This project currently is referencing RFC 2616 for the implementation of HTTP/1.1, I wouldn't change this...
static HTTP_PROTO_VERSION: &str = "HTTP/1.1";

lazy_static! {
    static ref FILECACHE: FileCache = FileCache::new(&DOC_ROOT);
}

#[tokio::main]
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(BIND_ADDR).await?;
    info!("Listening on {}", &BIND_ADDR);

    lazy_static::initialize(&FILECACHE);

    loop {
        let boxed_result= Box::new(listener.accept().await?);
        
        tokio::spawn(async move { 
            handle_connection(boxed_result).await;
        });
    }
}

async fn handle_connection(mut boxed_result: Box<(TcpStream, std::net::SocketAddr)>) {
    let mut buf: [u8; 1024] = [0; 1024];
    let (stream, addr) = boxed_result.borrow_mut();
    info!("New client connection from {}", addr);

    let bytes_read = stream.read(&mut buf).await;
    let _bytes_read = match bytes_read {
        Ok(b) => {
            debug!("received {} bytes from {}", &b, &addr);
            b
        },
        Err(e) => {
            debug!("received an error on bytes read: {} from {}", e, &addr);
            return;
        }
    };
    

    let request = tokio::task::block_in_place(|| {
            let buf = String::from_utf8_lossy(&buf);
            return HttpRequest::parse(&buf);
    });

    match request {
        Ok(req) => match req.method {
            HttpMethod::GET => {
                debug!(
                    "GET request from {} -> \n{:#?}",
                    stream.peer_addr().unwrap(),
                    &req
                );

                let index_page = tokio::task::block_in_place(|| {
                    FILECACHE.open(&req.req_uri.file.to_str().unwrap()); 
                    return FILECACHE.read(&req.req_uri.file.to_str().unwrap()); 
                });
                

                let response = format!(
                    "{} {} {}\r\n\r\n{}",
                    HTTP_PROTO_VERSION,
                    HttpStatusCode::HttpOk.value().0,
                    HttpStatusCode::HttpOk.value().1,
                    &index_page
                );
                stream.write_all(response.as_bytes()).await.unwrap();
                stream.flush().await.unwrap();
            }
            HttpMethod::POST => {}
            HttpMethod::DELETE => {}
            HttpMethod::UPDATE => {}
            HttpMethod::HEAD => {}
            HttpMethod::OPTION => {}
            HttpMethod::CONNECT => {}
            HttpMethod::TRACE => {}
        },
        Err(ref e) => {
            let response = format!(
                "{} {} {}\r\n\r\n",
                HTTP_PROTO_VERSION,
                e.value().0,
                e.value().1
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            stream.flush().await.unwrap();
            debug!(
                "received {:?}  from {} -> {:?}",
                e,
                stream.peer_addr().unwrap().ip(),
                e
            );
        }
    }
}
