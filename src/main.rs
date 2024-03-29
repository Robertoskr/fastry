#![deny(warnings)]

extern crate time;
extern crate tokio;
extern crate pyo3;
use crate::fs::DirEntry;
use pyo3::prelude::*;
use pyo3::PyErr;
use pyo3::types::PyList;
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
use std::time::Instant;
use std::net::{TcpListener, TcpStream};

fn main() {
    let mut app = App::new();
    //ensure python path is set to the correct value

    //get all the routes of the project
    //TODO: move this to an env variable or to the executable run arguments 
    let project_path = "/Users/robertoskr/personalthings/website";

    prepare_python_things(project_path).unwrap();

    let raw_routes: Vec<(String, String)> = get_routes(project_path);
    //register all the routes
    app.register_routes(raw_routes);

    //start the tcp server
    let addr = env::args().nth(1).unwrap_or("127.0.0.1:8080".to_string());
    let addr = addr.parse::<SocketAddr>().unwrap();
    let listener = TcpListener::bind(&addr).unwrap();

    let mut workers = Vec::new();

    for _ in 0..10 { 
        add_and_start_worker(&mut workers, project_path, &app); 
    } 

    let mut worker_id = 0;
    let mut request_counter = 0;
    let mut start_time = Instant::now();
    for stream in listener.incoming() {
        let mut socket = stream.unwrap();
        //clone the app to pass it to the thread
        let worker = workers[worker_id].to_owned();
        
        //TODO: 16 bytes, we should make this dinamic ofc!
        let mut buffer = [0; 16384];
        //read the request data into the buffer
        let bytes_read = socket.read(&mut buffer).unwrap();
        let request_str = match std::str::from_utf8(&buffer[..bytes_read]) { 
            Ok(s) => s, 
            Err(_) => "",
        };

        //process the request
        match worker.send((Some(socket), request_str.to_string())) { 
            Ok(_) => (), 
            Err(_) => { 
                //remove the worker from the worker list 
                remove_worker_try_stop(&mut workers, worker_id);
            } 
        }

        //now lets see if we should add remove workers based on the traffic of the application
        let now = Instant::now();
        if now.duration_since(start_time).as_secs() > 60 { 
            let n_workers = workers.len();
            let ratio = request_counter as f64 / 60.0 / n_workers as f64;
            if ratio > 5.0 { 
                //add more workers 
                add_and_start_worker(&mut workers, project_path, &app); 
            } else if ratio < 0.2 { 
                //remove some workers
                remove_worker_try_stop(&mut workers, n_workers - 1);
            }

            start_time = now;
            request_counter = 0;
        } 

        //advance the worker !
        worker_id = worker_id + 1;
        if worker_id >= workers.len() { 
            worker_id = 0; 
        } 
        request_counter += 1;
    } 
}


fn remove_worker_try_stop(workers: &mut Vec<Sender<(Option<TcpStream>, String)>>, idx: usize) { 
    let worker = workers.remove(idx);
    //TODO: we should send a signal to the tread an kill it
    worker.send((None, String::new())).unwrap();
} 

fn add_and_start_worker(workers: &mut Vec<Sender<(Option<TcpStream>, String)>>, project_path: &str, application: &App) { 
    let mut worker = application.clone();
    let (tx, rx): (Sender<(Option<TcpStream>, String)>, Receiver<(Option<TcpStream>, String)>) = mpsc::channel();
    let clone = project_path.to_owned();
    thread::spawn(move || {
        worker.start(&clone, rx); 
    });
    workers.push(tx);
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

fn prepare_python_things(path: &str) -> Result<(), PyErr> { 
    //prepare python threads
    pyo3::prepare_freethreaded_python();
    //acquire the gil, that will be used in the threads!
    {
        let gil = Python::acquire_gil();
        let python = gil.python();

        //lets change the current working directory for python 
        let os = python.import("os")?;
        os.call_method1("chdir", (path, ))?;

        let syspath: &PyList = python
            .import("sys")?
            .getattr("path")?
            .extract()?;

        syspath.append(path)?;

        //add the venv to the syspath 
        let mut venv_path = path.to_string();
        venv_path.push_str("/venv/lib/python3.10/site-packages/");
        syspath.append(venv_path)?;
    }
    Ok(()) 
} 
