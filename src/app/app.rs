extern crate pyo3;

use pyo3::{prelude::*};
use crate::request::ProcessedRequest;
use crate::File;
use crate::Python;
use crate::Request;
use pythonize::pythonize;
use serde::Serialize;
use std::collections::HashMap;
use std::io::Read;
use std::time::Instant;
use std::sync::mpsc::Receiver;
use std::net::TcpStream;
use std::io::Write;

#[allow(dead_code)]
#[derive(Serialize, Debug, Clone)]
struct RouteNode {
    path: Option<String>,
    handler: Option<String>,
    childrens: HashMap<String, Box<RouteNode>>,
    //TODO: add field for handling paths with custom variables
}

impl RouteNode {
    pub fn default() -> Self {
        Self {
            path: None,
            handler: None,
            childrens: HashMap::new(),
        }
    }
}

#[allow(dead_code)]
#[derive(Serialize, Clone)]
pub struct App {
    raw_routes: HashMap<String, String>,
    routes_tree: Box<RouteNode>,
}

impl App {
    pub fn new() -> Self {
        Self {
            raw_routes: HashMap::new(),
            routes_tree: Box::new(RouteNode::default()),
        }
    }

    pub fn register_routes(&mut self, raw_routes: Vec<(String, String)>) {
        //the dict consists of keys: route paths and values: handler paths
        //create a tree to resolve the paths in linear time
        //load the route_tree
        self.load_route_tree(&raw_routes);
    }

    fn load_route_tree(&mut self, raw_routes: &Vec<(String, String)>) {
        //load the route tree, for later being used to resolve the request handlers
        for (raw_route, raw_path) in raw_routes {
            println!("Registering: {} -> {}", raw_route, raw_path);
            match raw_route.find('<') {
                Some(_) => Self::recursive_insert(raw_route, raw_path, &mut self.routes_tree),
                None => {
                    //as this route does not have any param, we can safely store it in one time
                    let mut node = RouteNode::default();
                    node.path = Some(raw_route.to_string());
                    node.handler = Some(raw_path.to_string());
                    self.routes_tree
                        .childrens
                        .insert(raw_route.to_string(), Box::new(node));
                }
            }
        }
    }

    fn recursive_insert(raw_route: &str, raw_path: &str, tree: &mut Box<RouteNode>) {
        match raw_route.split_once('/') {
            Some((left, right)) => match tree.childrens.get(left) {
                Some(_) => {
                    Self::recursive_insert(right, raw_path, tree.childrens.get_mut(left).unwrap());
                }
                None => {
                    let mut node = RouteNode::default();
                    node.path = Some(left.to_string());
                    tree.childrens.insert(left.to_string(), Box::new(node));
                    Self::recursive_insert(right, raw_path, tree.childrens.get_mut(left).unwrap());
                }
            },
            None => {
                //add the handler to the tree
                //we are in the end of the path
                match tree.childrens.get_mut(raw_route) {
                    Some(node) => {
                        node.handler = Some(raw_path.to_string());
                    }
                    None => {
                        let mut node = RouteNode::default();
                        node.path = Some(raw_route.to_string());
                        node.handler = Some(raw_path.to_string());
                        tree.childrens.insert(raw_route.to_string(), Box::new(node));
                    }
                }
            }
        }
    }

    pub fn resolve_route(&self, route: &str) -> Option<String> {
        //resolve the route, returning the the path of the handler
        //try to resolve the whole route first
        match self.routes_tree.childrens.get(route) {
            Some(node) => return node.handler.clone(),
            None => (),
        };

        //try to resolve the route one by one
        let mut actual_node = &self.routes_tree;
        let as_list: Vec<String> = route.split('/').map(|p| p.to_string()).collect();
        for (i, p) in as_list.iter().enumerate() {
            match actual_node.childrens.get(p) {
                Some(node) => {
                    if i == as_list.len() - 1 {
                        return node.handler.clone();
                    }
                    actual_node = node;
                }
                None => {
                    break;
                }
            }
        }
        None
    }

    pub fn start(&mut self, receiver: Receiver<(TcpStream, String)>){ 
        loop { 
            let (mut socket, raw_request) = receiver.recv().unwrap();
            unsafe { 
                Python::with_gil_unchecked(|py| {
                    let result = self.process_request(py, raw_request); 

                    //write the reponse
                    socket.write_all(result.as_bytes()).unwrap();
                    socket.flush().unwrap();
                });
            }
        } 
    } 

    pub fn process_request(&mut self, py: Python, raw_request: String)-> String {
        //process a request
        let request = Request::from_string(raw_request);

        //get the handler path
        let handler_path = self.resolve_route(request.path.as_str());

        match handler_path {
            Some(path) => {
                //send the request to the handler and get the response
                //handler not found yet, create find it.
                let (file_name, fn_name) = path.split_once("::").unwrap();
                let mut file = File::open(file_name).unwrap();
                let mut code = String::new();
                _ = file.read_to_string(&mut code);
                let module = PyModule::from_code(py, code.as_str(), "app.py", "app").unwrap();
                let handler: PyObject = module.getattr(fn_name).unwrap().into();
                self.execute_request(&py, &handler, request).unwrap()
            }
            None => {
                "".to_string()
            }
        }
    }

    fn execute_request(&self, py: &Python, handler: &PyObject, request: Request) -> PyResult<String> {
        //process the headers and body
        let processed_request = ProcessedRequest::from_request(request);

        //send this request to the python handler
        let res = handler.call1(*py,(
            pythonize(*py, self).unwrap(),
            pythonize(*py, &processed_request).unwrap()))?;
        
        let code: i32 = res.getattr(*py, "code")?.extract(*py)?;        
        let _type: String = res.getattr(*py, "type")?.extract(*py)?;        
        let body: String = res.getattr(*py, "body")?.extract(*py)?;        
        
        Ok(format!(
            "HTTP/1.1 {} OK\r\nDate: {:?}\r\nServer: Someserver\r\nContent-Length: {}\r\nContent-Type: {}\r\nConnection: close\r\n\r\n\r\n{}", 
            code, Instant::now(), body.len() + 2, _type, body, 
        )) 
    }
}

