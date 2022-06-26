#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum RequestMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
    HEAD,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Request {
    pub method: RequestMethod,
    pub http_version: String,
    pub headers: String,
    pub body: String,
    pub path: String,
    pub raw_request: String,
}

impl Request {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let as_str = std::str::from_utf8(&bytes).unwrap();
        let (method, path, http_version) = Self::get_request_core_info(&as_str);
        Self {
            method: method,
            http_version: http_version,
            headers: Self::get_raw_headers(&as_str),
            body: Self::get_raw_body(&as_str),
            path: path,
            raw_request: String::new(), //as_str.to_string(),
        }
    }

    fn get_request_core_info(request: &str) -> (RequestMethod, String, String) {
        //the method is ussually the first thing of the request
        let mut method_string = String::new();
        let mut path = String::new();
        let mut http_version = String::new();
        let mut state: u8 = 0;
        for ch in request.chars() {
            if ch == '\r' {
                break;
            }
            if ch == ' ' {
                if state == 2 {
                    break;
                }
                state += 1;
                continue;
            }
            if state == 0 {
                method_string.push(ch);
            } else if state == 1 {
                path.push(ch);
            } else if state == 2 {
                http_version.push(ch);
            }
        }
        let method = match method_string.as_str() {
            "GET" => RequestMethod::GET,
            "POST" => RequestMethod::POST,
            "DELETE" => RequestMethod::DELETE,
            "PATCH" => RequestMethod::PATCH,
            "PUT" => RequestMethod::PUT,
            "HEAD" => RequestMethod::HEAD,
            _ => unreachable!(),
        };

        (method, path, http_version)
    }

    //parses the raw request to get the raw headers
    fn get_raw_headers(request: &str) -> String {
        let start_idx = request.find("\r\n").unwrap();
        let end_idx = request.find("\r\n\r\n").unwrap();

        request[start_idx + 2..end_idx].to_string()
    }

    //parses the raw request to get the raw json content
    fn get_raw_body(request: &str) -> String {
        let start_idx = request.find("\r\n\r\n").unwrap() + 4;
        request[start_idx..].to_string()
    }

    //TODO: get json and create json struct
    //todo get the json and return it
    //preferably do it in async way
    //async fn json() -> () {}

    //TODO: get headers and create headers struct
    //todo get the headers of the request and return it
    //preferably do it in async way
    //pub fn headers() -> () {}
}
