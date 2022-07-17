#![deny(warnings)]

extern crate time;
extern crate tokio;
use crate::fs::DirEntry;
use crate::tokio::io::AsyncReadExt;
use crate::tokio::io::AsyncWriteExt;
use pyo3::{prelude::*, types::PyModule};
use std::env;
use std::fs;
use std::io;
use std::io::BufReader;
use std::net::SocketAddr;
use std::path::Path;

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

    //get all the routes of the project
    let project_path = "YOUR_ROUTE_PROJECT_HERE";
    let raw_routes: Vec<(String, String)> = get_routes(project_path);
    //register all the routes
    app.register_routes(raw_routes);

    //start the tcp server
    let addr = env::args().nth(1).unwrap_or("127.0.0.1:8080".to_string());
    let addr = addr.parse::<SocketAddr>().unwrap();
    let listener = TcpListener::bind(&addr).await.unwrap();
    println!("Listening on: {}", addr);

    loop {
        let (mut socket, _) = listener.accept().await.unwrap();
        //clone the app to pass it to the thread
        let mut app = app.clone();
        tokio::spawn(async move {
            let mut buf = vec![0; 16364]; //16 bytes, should make this dynamic ?

            // In a loop, read data from the socket and write the data back.
            let n = socket.read(&mut buf).await.expect("");
            // no input
            if n == 0 {
                return;
            }

            //process the request
            let result: String = app.process_request(&buf[..n]);

            //write the reponse
            socket
                .write_all(result.as_bytes())
                .await
                .expect("failed to write data to socket");
        });
    }
}

fn visit_dirs(
    dir: &Path,
    cb: &dyn Fn(&DirEntry, &mut Vec<(String, String)>),
    container: &mut Vec<(String, String)>,
) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let path_str = path.to_str().unwrap();
            if path_str.find("venv").is_some() || path_str.find("target").is_some() {
                continue;
            }
            if path.is_dir() {
                visit_dirs(&path, cb, container)?;
            } else {
                cb(&entry, container);
            }
        }
    }
    Ok(())
}

fn visit_python_file(entry: &DirEntry, container: &mut Vec<(String, String)>) {
    let path = entry.path();
    let path = path.to_str().unwrap();
    if path.find(".py").is_none() {
        return;
    }

    let routes = get_routes_for_file(path);
    for (path, fn_name) in routes {
        container.push((path, fn_name));
    }
}

fn get_routes(project_path: &str) -> Vec<(String, String)> {
    let mut container: Vec<(String, String)> = Vec::new();
    _ = visit_dirs(Path::new(project_path), &visit_python_file, &mut container);

    container
}

pub fn get_routes_for_file(path: &str) -> Vec<(String, String)> {
    //get all the routes from the app base file
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let mut last_path: Option<String> = None;
    let mut routes: Vec<(String, String)> = Vec::new();
    for line in reader.lines() {
        let line = line.unwrap_or("".to_string());
        if line.find("#->r").is_some() {
            //this is a route function
            let route_start = line.find('/').unwrap();
            last_path = Some(line[route_start..].trim().to_string());
            continue;
        }
        if line.trim() != "" && last_path.is_some() {
            //get the name of the function
            let (_, name) = line.split_once("def").unwrap();
            routes.push((
                last_path.unwrap(),
                format!("{}::{}", path, name.split_once('(').unwrap().0.trim()).to_string(),
            ));
            last_path = None;
            continue;
        }
    }
    routes
}
