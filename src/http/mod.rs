use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, PartialEq)]
#[allow(dead_code)]
pub enum HttpMethod {
    GET,
    POST,
    UPDATE,
    DELETE,
    CONNECT,
    TRACE,
    HEAD,
    OPTION,
}

#[derive(Debug, PartialEq)]
#[allow(dead_code)]
pub enum HttpStatusCode {
    Continue,
    HttpOk,
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    InternalServerError,
    NotImplemented,
}

impl HttpStatusCode {
    pub fn value(&self) -> (u16, &str) {
        match *self {
            HttpStatusCode::Continue => (100, "Continue"),
            HttpStatusCode::HttpOk => (200, "OK"),
            HttpStatusCode::BadRequest => (400, "Bad request"),
            HttpStatusCode::Unauthorized => (401, "Unauthorized"),
            HttpStatusCode::Forbidden => (403, "Forbidden"),
            HttpStatusCode::NotFound => (404, "Not found"),
            HttpStatusCode::InternalServerError => (500, "Internal server error"),
            HttpStatusCode::NotImplemented => (501, "Not implemented"),
        }
    }
}
#[derive(Debug)]
pub struct ReqURI {
    pub uri: String,
    pub file: PathBuf
}

impl ReqURI {
    fn new(uri: String, file: PathBuf) -> ReqURI {
        ReqURI {
            uri,
            file
        }
    }
}

//TODO: prob should just make a HttpRequest structure with an option<T> for different types?
#[derive(Debug)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub req_uri: ReqURI,
    pub proto_ver: String,
    pub req_headers: Option<HashMap<String, String>>,
}

impl HttpRequest {
    fn new(
        method: HttpMethod,
        req_uri: PathBuf,
        proto_ver: &str,
        req_headers: Option<HashMap<String, String>>,
    ) -> HttpRequest {

        HttpRequest {
            method,
            req_uri: ReqURI::new(req_uri.to_str().unwrap().replace(crate::DOC_ROOT, ""), req_uri),
            proto_ver: String::from(proto_ver),
            req_headers: req_headers,
        }

    }

    //TODO: This needs to be refactored to only parse the http request and return the completed struct...valid or not
    //by returning an Err(_) we mask the original request and cannot get anymore information out of it later
    pub fn parse(request: &str) -> Result<Box<HttpRequest>, HttpStatusCode> {
        let raw_headers = request.split("\r\n")
            .filter_map(
                |v|
                match v.trim().contains("\u{0}") {
                    true => None,
                    false => Some(v)
                })
            .collect::<Vec<&str>>();
        
        
        let req_headers = raw_headers.clone()
            .into_iter().map(
                |s|
                match s.split_once(' ') {
                    Some((k,v)) => (String::from(k), String::from(v)),
                    None => ("none".to_string(), "none".to_string()),
                }).collect::<HashMap<String, String>>();

        let mut req_vec = match raw_headers.get(0) {
            Some(s) => {
                s.split(' ').collect()
            },
            // empty req_vec means there was an issue parsing the request, it will return BadRequest below
            None => vec![]
        };

        //I am pretty sure all http requests have to specify at least the Method, URI, HTTP Protocol so the minimum length
        //for a valid request should be 3
        if req_vec.len() < 3 {
            return Err(HttpStatusCode::BadRequest);
        }

        // should prob check that req_vec has elements?
        let result = match req_vec[0] {
            "GET" => HttpRequest::parse_get(&mut req_vec), 
            "POST" => HttpRequest::parse_post(&mut req_vec),
            "UPDATE" => HttpRequest::parse_update(&mut req_vec),
            "DELETE" => HttpRequest::parse_delete(&mut req_vec),
            "CONNECT" => HttpRequest::parse_connect(&mut req_vec),
            "TRACE" => HttpRequest::parse_trace(&mut req_vec),
            "HEAD" => HttpRequest::parse_head(&mut req_vec),
            "OPTION" => HttpRequest::parse_option(&mut req_vec),
            _ => Err(HttpStatusCode::BadRequest)
        };

        match result {
            Ok(hr) => {
                return Ok(Box::new(HttpRequest::new(
                    hr.method,
                    hr.req_uri.file,
                    &String::from(hr.proto_ver),
                    Some(req_headers)
                )))
            },
            Err(e) => {
                return Err(e)
            }
        }

    }

    fn parse_get(req_vec: &mut Vec<&str>) -> Result<HttpRequest, HttpStatusCode> {
        crate::debug!("GET -> {:?}", &req_vec);
            //Requesting http://example.com would result in GET / HTTP/1.1
            //so we rewrite the request to the default index which is index.html -> GET index.html HTTP/1.1
            if req_vec[1] == "/" {
                req_vec[1] = crate::DEFAULT_INDEX;
            }

            //Requesting http://example.com/afile.html would result in GET /afile.html HTTP/1.1
            //we just chop off the / here so when we canonicalize it it doesn't look at the root of the drive
            // ie /afile.html instead of ./afile.html
            if req_vec[1].starts_with('/') && req_vec[1].len() > 1 {
                let mut s = req_vec[1];
                s = &s[1..];
                req_vec[1] = s;
            }

            //Attempt to prevent directory recursion exploits hopfully and it has the added bonus
            //of checking if the file exists so we can return a 404
            let uri_path = PathBuf::from(format!("{}{}", crate::DOC_ROOT,&req_vec[1])).canonicalize();
            crate::debug!("uri: {:?}", &req_vec[1]);
            crate::debug!("PathBuf: {:?}", &uri_path);
            let uri_path = match uri_path {
                Ok(p) => p,
                Err(_) => return Err(HttpStatusCode::NotFound),
            };
            //Check if the (canonical)file is in the allowed doc root path
            let doc_root_path = PathBuf::from(&crate::DOC_ROOT).canonicalize().unwrap();
            if !uri_path.starts_with(&doc_root_path) {
                return Err(HttpStatusCode::BadRequest);
            }

            Ok(HttpRequest::new(
                HttpMethod::GET,
                uri_path,
                crate::HTTP_PROTO_VERSION,
                None
            ))

    }

    fn parse_post(_req_vec: &mut Vec<&str>) -> Result<HttpRequest, HttpStatusCode> {
        Err(HttpStatusCode::NotImplemented)
    }

    fn parse_update(_req_vec: &mut Vec<&str>) -> Result<HttpRequest, HttpStatusCode> {
        Err(HttpStatusCode::NotImplemented)
    }

    fn parse_delete(_req_vec: &mut Vec<&str>) -> Result<HttpRequest, HttpStatusCode> {
        Err(HttpStatusCode::NotImplemented)
    }

    fn parse_connect(_req_vec: &mut Vec<&str>) -> Result<HttpRequest, HttpStatusCode> {
        Err(HttpStatusCode::NotImplemented)
    }

    fn parse_trace(_req_vec: &mut Vec<&str>) -> Result<HttpRequest, HttpStatusCode> {
        Err(HttpStatusCode::NotImplemented)
    }

    fn parse_head(_req_vec: &mut Vec<&str>) -> Result<HttpRequest, HttpStatusCode> {
        Err(HttpStatusCode::NotImplemented)
    }

    fn parse_option(_req_vec: &mut  Vec<&str>) -> Result<HttpRequest, HttpStatusCode> {
        Err(HttpStatusCode::NotImplemented)
    }

}