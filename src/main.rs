#![deny(warnings)]

extern crate time;
extern crate tokio;
use crate::tokio::io::AsyncReadExt;
use crate::tokio::io::AsyncWriteExt;
use pyo3::{prelude::*, types::PyModule};
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;

use tokio::net::TcpListener;
pub mod app;
pub mod request;
use app::App;
use request::Request;
use std::fs::File;
use std::io::prelude::*;

#[tokio::main]
async fn main() {
    let mut app = App::new();

    //prepare python threads
    pyo3::prepare_freethreaded_python();

    //ensure python path is set to the correct value

    //get all the routes of the app (from python app.py file)
    let raw_routes: HashMap<String, String> =
        get_routes("/Users/robertoskr/personalthings/opensource/fastry/test.py");
    //register all the routes
    app.register_routes(raw_routes);

    //start the tcp server
    let addr = env::args().nth(1).unwrap_or("127.0.0.1:8080".to_string());
    let addr = addr.parse::<SocketAddr>().unwrap();
    let listener = TcpListener::bind(&addr).await.unwrap();
    println!("Listening on: {}", addr);

    loop {
        let (mut socket, _) = listener.accept().await.unwrap();

        tokio::spawn(async move {
            let mut buf = vec![0; 1024];

            // In a loop, read data from the socket and write the data back.
            let n = socket.read(&mut buf).await.expect("");
            // no input
            if n == 0 {
                return;
            }

            //process the request
            let response = get_response(&buf[..n]);

            //write the reponse
            socket
                .write_all(response.as_bytes())
                .await
                .expect("failed to write data to socket");
        });
    }
}

//this is one of the most important functions, it handles incoming requests and return valid http reponses
pub fn get_response(buf: &[u8]) -> String {
    //process the request and return a response

    //parse the request to an structure
    let _ = Request::from_bytes(buf);

    //now that we determine the type of the request, get the handler,
    //and determine if we want to do something with it

    String::from("something")
}

pub fn get_handler(_: Python, _: String) -> Option<PyObject> {
    //load the code
    None
    /*let module = PyModule::from_code(py, code.as_str(), "test.py", "test").unwrap();
    let handler: PyObject = module.getattr("hello_world").unwrap().into();
    let result: String = function
        .call1(py, ("roberto",))
        .unwrap()
        .extract(py)
        .unwrap();
    println!("{}", result);
    None*/
}

pub fn get_routes(path: &str) -> HashMap<String, String> {
    //get all the routes from the app base file
    let mut file = File::open(path).unwrap();
    let mut code = String::new();
    _ = file.read_to_string(&mut code);

    Python::with_gil(|py| {
        let module = PyModule::from_code(py, code.as_str(), "app.py", "app").unwrap();
        let app: PyObject = module.getattr("app").unwrap().into();
        let result: PyObject = app.call_method0(py, "get_routes").unwrap();

        let result = result.extract::<HashMap<String, String>>(py).unwrap();
        return result;
    })
}
