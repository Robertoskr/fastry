use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Serialize, Debug, Clone)]
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
    pub raw_headers: String,
    pub raw_body: String,
    pub path: String,
    pub raw_request: String,
    pub json: Option<Value>,
    pub headers: Option<HashMap<String, String>>,
    pub path_variables: Option<HashMap<String, String>>,
}

#[derive(Serialize)]
pub struct ProcessedRequest {
    pub method: RequestMethod,
    pub http_version: String,
    pub json: Value,
    pub headers: HashMap<String, String>,
    pub path_variables: HashMap<String, String>,
    pub text: String,
}

impl ProcessedRequest {
    pub fn from_request(mut request: Request) -> Self {
        request.process();
        Self {
            method: request.method,
            http_version: request.http_version,
            json: request.json.unwrap_or(Value::Null),
            headers: request.headers.unwrap(),
            text: request.raw_body,
            path_variables: request.path_variables.unwrap_or(HashMap::new())
        }
    }
}

impl Request {
    pub fn from_string(string: String)-> Self {
        let (method, path, http_version) = Self::get_request_core_info(&string);
        Self {
            method: method,
            http_version: http_version,
            raw_headers: Self::get_raw_headers(&string),
            raw_body: Self::get_raw_body(&string),
            path: path,
            raw_request: String::new(), //as_str.to_string(),
            json: None,
            headers: None,
            path_variables: None, 
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

    //parses the raw request to get the raw raw_headers
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

    pub fn process(&mut self) {
        //process the request for being passed to python
        //first process the headers, and see the body type and lenght
        let headers: HashMap<String, String>;

        if self.headers.is_some() {
            return;
        }

        headers = self.headers();

        match headers
            .get(&String::from("Content-Type"))
            .unwrap_or(&String::from(""))
            .as_str()
        {
            "application/json" => {
                //parse the body with serde
                let value: Value =
                    serde_json::from_str(self.raw_body.as_str()).expect("Wrong json");
                self.json = Some(value);
            }
            "application/xml" => {
                //TODO: support xml parsing
            }
            _ => {}
        }

        self.headers = Some(headers);
    }

    //TODO: get json and create json struct
    //todo get the json and return it
    //preferably do it in async way
    //async fn json() -> () {}

    //TODO: get raw_headers and create headers struct
    //todo get the headers of the request and return it
    //preferably do it in async way
    pub fn headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::<String, String>::new();
        for line in self.raw_headers.split("\r\n") {
            let (key, value) = line.split_once(':').unwrap();
            let key = key.trim();
            let value = value.trim();
            headers.insert(key.to_string(), value.to_string());
        }
        headers
    }
}
