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
    //sometimes, we change the path of the node, 
    //so here we store the original path
    original_path: Option<String>,
    handler: Option<String>,
    childrens: HashMap<String, Box<RouteNode>>,
    //TODO: add field for handling paths with custom variables
}

impl RouteNode {
    pub fn default() -> Self {
        Self {
            path: None,
            original_path: None,
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
    #[serde(skip)]
    handlers: HashMap<(String, String), PyObject>,
    #[serde(skip)] 
    python_app: Option<PyObject>
}

impl App {
    pub fn new() -> Self {
        Self {
            raw_routes: HashMap::new(),
            routes_tree: Box::new(RouteNode::default()),
            handlers: HashMap::new(), 
            python_app: None, 
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
                    node.original_path = Some(raw_route.to_string());
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
            Some((left, right)) => {
                let original_path = left.to_string();
                let left = if left.starts_with("<") { "CUSTOM_PARAM" } else { left } ;
                if left == "" { 
                    return Self::recursive_insert(right, raw_path, tree);
                } 

                match tree.childrens.get(left) {
                    Some(_) => {
                        Self::recursive_insert(right, raw_path, tree.childrens.get_mut(left).unwrap());
                    }
                    None => {
                        let mut node = RouteNode::default();
                        node.path = Some(left.to_string());
                        node.original_path = Some(original_path);
                        tree.childrens.insert(left.to_string(), Box::new(node));
                        Self::recursive_insert(right, raw_path, tree.childrens.get_mut(left).unwrap());
                    }
                }
            },
            None => {
                //add the handler to the tree
                //we are in the end of the path
                let original_path = raw_route.to_string();
                let raw_route = if raw_route.starts_with("<") { "CUSTOM_PARAM" } else { raw_route } ;
                match tree.childrens.get_mut(raw_route) {
                    Some(node) => {
                        node.handler = Some(raw_path.to_string());
                    }
                    None => {
                        let mut node = RouteNode::default();
                        node.path = Some(raw_route.to_string());
                        node.original_path = Some(original_path);
                        node.handler = Some(raw_path.to_string());
                        tree.childrens.insert(raw_route.to_string(), Box::new(node));
                    }
                }
            }
        }
    }

    pub fn resolve_route(&self, route: &str) -> (Option<String>, HashMap<String, String>) {
        //resolve the route, returning the the path of the handler
        //try to resolve the whole route first
        let mut path_variables = HashMap::new(); 

        match self.routes_tree.childrens.get(route) {
            Some(node) => return (node.handler.clone(), path_variables),
            None => (),
        };
        
        let (route, route_variables) = match route.split_once('?') { 
            Some(split) => split,
            None => (route, "") 
        };

        for route_var in route_variables.split('&') { 
            match route_var.split_once('=') { 
                Some((key, value)) => { 
                    path_variables.insert(key.to_string(), value.to_string());
                },
                None => ()
            } 
        } 

        //try to resolve the route one by one
        let mut actual_node = self.routes_tree.clone();
        let mut as_list: Vec<String> = route.split('/').map(|p| p.to_string()).collect();
        let _ =  as_list.remove(0);
        for (i, p) in as_list.iter().enumerate() {
            match self.next_item_while_resolving(p, &actual_node, true) { 
                Some(child_node) => { 
                    let child_clone = child_node.clone();
                    if child_clone.path.unwrap() == "CUSTOM_PARAM"{ 
                        path_variables.insert(
                            child_clone.original_path.unwrap(),
                            p.to_string(),
                        );
                    } 
                    if i == as_list.len() - 1 {
                        return (child_node.handler.clone(), path_variables);
                    }
                    actual_node = child_node;
                },
                None => (),
            } 
        }
        (None, path_variables) 
    }
    
    fn next_item_while_resolving(&self, p: &str, node: &Box<RouteNode>, fallback: bool) -> Option<Box<RouteNode>> { 
        match node.childrens.get(p) {
            Some(children_node) => {
                return Some(children_node.clone());
            }
            None => {
                if fallback { 
                    //this can be because the node is a custome thingy
                    let p = "CUSTOM_PARAM"; 
                    return self.next_item_while_resolving(p, node, false);
                } 
                None
           }
        }
    } 

    pub fn start(&mut self, project_path: &str, receiver: Receiver<(TcpStream, String)>){ 
        self.initialize_application(project_path);
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
        //parse the raw request string to a request
        let mut request = Request::from_string(raw_request);

        //get the handler path
        let (maybe_path, route_variables) = self.resolve_route(request.path.as_str());
        //get the handler (python function that is going to handle the request !
        match maybe_path{
            Some(path) => {
                request.path_variables = Some(route_variables);
                let handler = self.get_or_save_handler(py, path).unwrap();
                //send the request to the handler and get the response
                //handler not found yet, create find it.
                
                self.execute_request(&py, &handler, request).unwrap()
            }
            None => {
                "".to_string()
            }
        }
    }

    fn initialize_application(&mut self, path: &str) {
        let mut filename = path.to_string();
        filename.push_str("/fastry.py");
        let mut file = File::open(filename).unwrap();
        let mut code = String::new();
        file.read_to_string(&mut code).unwrap();
        unsafe { 
            Python::with_gil_unchecked(|py| {
                let module = PyModule::from_code(py, code.as_str(), "project/fastry.py", "fastry").unwrap();
                let application: PyObject = module.getattr("FastryApplication").unwrap().into();
                let python_application: PyObject = application.call0(py).unwrap();
                self.python_app = Some(python_application);
            });
        } 
    } 

    fn get_or_save_handler(&mut self, py: Python ,path: String) -> Option<PyObject> { 
        let (file_name, fn_name) = path.split_once("::").unwrap();
        match self.handlers.get(&(file_name.to_string(), fn_name.to_string())) { 
            Some(handler) => Some(handler.clone()), 
            None => { 
                let mut file = File::open(file_name).unwrap();
                let mut code = String::new();
                _ = file.read_to_string(&mut code);
                let module = PyModule::from_code(py, code.as_str(), "project/app.py", "app").unwrap();
                let handler: PyObject = module.getattr(fn_name).unwrap().into();
                self.handlers.insert((file_name.to_string(), fn_name.to_string()), handler.clone());
                Some(handler) 
            } 
        }  
    } 

    fn execute_request(&self, py: &Python, handler: &PyObject, request: Request) -> PyResult<String> {
        //process the headers and body
        let processed_request = ProcessedRequest::from_request(request);

        //send this request to the python handler
        let res = handler.call1(*py,(
            self.python_app.clone().unwrap(),
            //convert to dict, the processed request
            pythonize(*py, &processed_request).unwrap()
        ))?;
        
        let code: i32 = res.getattr(*py, "code")?.extract(*py)?;        
        let _type: String = res.getattr(*py, "type")?.extract(*py)?;        
        let body: String = res.getattr(*py, "body")?.extract(*py)?;        
        
        Ok(format!(
            "HTTP/1.1 {} OK\r\nDate: {:?}\r\nServer: Someserver\r\nContent-Length: {}\r\nContent-Type: {}\r\nConnection: close\r\n\r\n\r\n{}", 
            code, Instant::now(), body.len() + 2, _type, body, 
        )) 
    }
}

