#![deny(warnings)]

extern crate time;
extern crate tokio;
use crate::fs::DirEntry;
use pyo3::prelude::*;
use std::env;
use std::fs;
use std::io;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;

pub mod app;
pub mod request;
use app::App;
use request::Request;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::thread;
use std::io::Read;
use std::net::{TcpListener, TcpStream};

fn main() {
    let mut app = App::new();

    //prepare python threads
    pyo3::prepare_freethreaded_python();
    //acquire the gil, that will be used in the threads!
    let _ = Python::acquire_gil();

    //ensure python path is set to the correct value

    //get all the routes of the project
    let project_path = "/Users/robertoskr/personalthings/opensource/fastry";
    let raw_routes: Vec<(String, String)> = get_routes(project_path);
    //register all the routes
    app.register_routes(raw_routes);

    //start the tcp server
    let addr = env::args().nth(1).unwrap_or("127.0.0.1:8080".to_string());
    let addr = addr.parse::<SocketAddr>().unwrap();
    let listener = TcpListener::bind(&addr).unwrap();

    let mut workers = Vec::new();

    for _ in 0..10 { 
        let mut worker = app.clone();
        let (tx, rx): (Sender<(TcpStream, String)>, Receiver<(TcpStream, String)>) = mpsc::channel();
        workers.push(tx);
        thread::spawn(move || {
            worker.start(rx); 
        });
    } 

    let mut worker_id = 0;
    
    for stream in listener.incoming() {
        let mut socket = stream.unwrap();
        //clone the app to pass it to the thread
        let worker = workers[worker_id].to_owned();

        let mut buffer = [0; 16384];
        //read the request data into the buffer
        let bytes_read = socket.read(&mut buffer).unwrap();
        let request_str = match std::str::from_utf8(&buffer[..bytes_read]) { 
            Ok(s) => s, 
            Err(_) => "",
        };

        //process the request
        worker.send((socket, request_str.to_string())).unwrap();
        //println!("Sent request to worker: {}", worker_id);
        worker_id = worker_id + 1;
        if worker_id >= workers.len() { 
            worker_id = 0; 
        } 
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
