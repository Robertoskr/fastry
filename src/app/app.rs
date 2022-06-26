use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Debug)]
struct RouteNode {
    path: Option<String>,
    handler: Option<String>,
    childrens: HashMap<String, Box<RouteNode>>,
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
#[derive(Debug)]
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

    pub fn register_routes(&mut self, raw_routes: HashMap<String, String>) {
        //the dict consists of keys: route paths and values: handler paths
        //create a tree to resolve the paths in linear time
        //load the route_tree
        self.load_route_tree(&raw_routes);
        self.raw_routes = raw_routes;
    }

    fn load_route_tree(&mut self, raw_routes: &HashMap<String, String>) {
        //load the route tree, for later being used to resolve the request handlers
        for (raw_route, raw_path) in raw_routes.iter() {
            Self::recursive_insert(raw_route, raw_path, &mut self.routes_tree);
        }

        println!("{:?}", self.routes_tree);
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
                tree.handler = Some(raw_path.to_string());
            }
        }
    }
}
