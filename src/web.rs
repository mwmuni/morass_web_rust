use rand::random;
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

// Prototype neural network that focuses on facilitating an all-node-input-all-node-output network. The idea is that all neurons in the brain are interconnected and are used as both input and output simultaneously.
//
// Main steps for the network are:
//
// Process all node threshold outputs to connected nodes and store the output in a temporary variable in the destination node
// Decay all the nodes by some percentage and fixed amount
// Assimilate the temporary charge from step 1 with the current charge
// These three steps are repeated continuously to simulate signals passing through a neural network. To run these three steps, run the 'step()' method in the MorassNetwork class.
//
// Since all neurons are inputs, any node can have its charge injected (or subtracted) at any time. To do this, simply call the 'inject_node(int node_id, double input)' method.

// A node is defined with the following parameters:
//
// T: Threshold; when the current charge meets or exceeds this value, the node fires
// Cp: Charge consumption percentage; when the threshold is triggered, this percentage of the current charge is removed
// Cf: Charge consumption fixed; when the threshold is triggered, this fixed value is subtracted from the current charge (before Cp)
// Dp: Decay percentage; this is how much the current charge decays every step current_chg*=(1-Dp)
// Df: Decay fixed; this value is subtracted from the current charge every step
pub struct Node {
    id: usize,
    threshold: f64,
    charge: Cell<f64>,
    cooldown: usize,
    cooldown_remaining: Cell<usize>,
    since_last_fire: Cell<usize>,
    temp_charge: Cell<f64>,
    num_outgoing_edges: Cell<usize>,
    charge_consumption_percentage: f64,
    charge_consumption_fixed: f64,
    decay_percentage: f64,
    decay_fixed: f64,
}

// An edge is defined with the following parameters:
//
// Op: Out percent; when a threshold is triggered, Op is multiplied by the pulse value of the start node and sent to the end node
// Of: Out fixed; when a threshold is triggered, Of is added to Op and sent to the end node
// start_node: the start node of the directed edge
// end_node: the end node of the directed edge
pub struct Edge {
    out_percentage: f64,
    out_fixed: f64,
    edge_health: Cell<usize>, // Number of failed steps before the edge is removed
    last_fire: Cell<usize>, // Number of steps since the edge last fired
    fire_within: usize, // Number of steps that the start_node needs to fire within otherwise edge_health is decremented
    end_node_fire_within: usize, // Number of steps that the end_node needs to fire within otherwise edge_health is decremented
    start_node: Rc<RefCell<Node>>,
    end_node: Rc<RefCell<Node>>,
}

pub struct MorassWeb {
    nodes: Vec<Rc<RefCell<Node>>>,
    edges: Vec<Rc<RefCell<Edge>>>,
    pairs: HashSet<(usize, usize)>, // usize representation of edges
    op_counter: usize,
}



impl MorassWeb {
    pub fn make_random_web(num_nodes: usize, num_edges: usize) -> Self {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        for n in 0..num_nodes {
            // Create node with random parameters
            let node = Node {
                id: n + 1,
                threshold: random::<f64>() * 10.0,
                charge: Cell::new(random::<f64>() * 5.0),
                cooldown: random::<usize>() % 5 + 1,
                cooldown_remaining: Cell::new(0),
                since_last_fire: Cell::new(0),
                temp_charge: Cell::new(0.0),
                num_outgoing_edges: Cell::new(0),
                charge_consumption_percentage: random::<f64>() * 20.0,
                charge_consumption_fixed: random::<f64>() * 3.0,
                decay_percentage: random::<f64>() * 0.05,
                decay_fixed: random::<f64>() * 0.2,
            };
            nodes.push(Rc::new(RefCell::new(node)));
        }

        // Make n random pairs of integers where n=num_edges and each integer is in the range [0, num_nodes)
        // Each pair is unique, (i, i) is not allowed, and (i, j) is the same as (j, i)
        let mut pairs = HashSet::<(usize, usize)>::new();
        let mut tries = 0;
        for _ in 0..num_edges {
            let mut pair = (random::<usize>() % num_nodes, random::<usize>() % num_nodes);
            while pair.0 == pair.1 || pairs.contains(&pair) || pairs.contains(&(pair.1, pair.0)) {
                pair = (random::<usize>() % num_nodes, random::<usize>() % num_nodes);
                tries += 1;
                if tries > 1000 {
                    println!("Could not find {} unique pairs", num_edges);
                    println!("Found {} unique pairs", pairs.len());
                    return Self {
                        nodes,
                        edges,
                        pairs,
                        op_counter: 0,
                    };
                }
            }
            pairs.insert(pair);
        }

        // Create edge with random parameters
        for pair in pairs.iter() {
            // Create edge with random parameters
            let edge = MorassWeb::default_edge(nodes.get(pair.0).unwrap(),
                                               nodes.get(pair.1).unwrap());
            edges.push(Rc::new(RefCell::new(edge)));
        }

        Self {
            nodes,
            edges,
            pairs,
            op_counter: 0,
        }
    }

    pub fn step(&mut self, verbose: bool) {
        // Process all node threshold outputs to connected nodes and store the output in a temporary variable in the destination node
        self.pulse(verbose);

        // Subtraction of charge from nodes if fired
        self.subtract_charge();

        // Decay all the nodes by some percentage and fixed amount
        self.decay();

        // Assimilate the temporary charge from step 1 with the current charge
        self.assimilate();

        // Decrement cooldowns
        self.cooldown_step();
    }

    fn cooldown_step(&mut self) {
        for node in &self.nodes {
            let node = node.borrow();
            if node.cooldown_remaining.get() > 0 {
                node.cooldown_remaining.set(node.cooldown_remaining.get()-1);
            }
        }
        for edge in &self.edges {
            let edge = edge.borrow();
            if edge.end_node.borrow().since_last_fire.get() == edge.end_node_fire_within { // Only penalise once
                edge.edge_health.set(edge.edge_health.get()-1);
            }
            if edge.last_fire.get() % edge.fire_within == edge.fire_within-1 {
                edge.edge_health.set(edge.edge_health.get()-1);
            }
        }
        // Delete any edges with edge_health == 0 and their corresponding pairs
        let mut to_remove = Vec::<usize>::new();
        for (i, edge) in self.edges.iter().enumerate() {
            if edge.borrow().edge_health.get() == 0 {
                to_remove.push(i);
            }
        }
        for i in to_remove.iter().rev() {
            let edge = self.edges.remove(*i);
            let pair = (edge.borrow().start_node.borrow().id, edge.borrow().end_node.borrow().id);
            self.pairs.remove(&pair);
        }
    }

    fn assimilate(&self) {
        for node in &self.nodes {
            let node = node.borrow();
            node.charge.set(node.charge.get() + node.temp_charge.get());
            node.temp_charge.set(0.0);
        }
    }

    fn decay(&self) {
        for node in &self.nodes {
            let node = node.borrow();
            node.charge
                .set(node.charge.get() - node.charge.get() * node.decay_percentage - node.decay_fixed);
        }
    }

    fn subtract_charge(&self) {
        for node in &self.nodes {
            // If the node is on cooldown, skip it
            if node.borrow().cooldown_remaining.get() > 0 {
                continue;
            }
            let node = node.borrow_mut();
            if node.charge.get() >= node.threshold {
                node.charge.set(
                    node.charge.get()
                        - node.charge.get() * node.charge_consumption_percentage
                        - node.charge_consumption_fixed,
                );
            }
        }
    }

    fn pulse(&mut self, verbose: bool) {
        for _edge in &self.edges {
            let edge = _edge.borrow();
            let start_node = edge.start_node.borrow();
            let end_node = edge.end_node.borrow();

            // If the start node or end node is on cooldown, skip it
            if start_node.cooldown_remaining.get() > 0 {
                continue;
            }

            let pulse = if start_node.charge.get() >= start_node.threshold {
                start_node.charge.get() * edge.out_percentage + edge.out_fixed
            } else {
                0.0
            };
            if pulse > 0.0 {
                self.op_counter += 1;
                start_node.cooldown_remaining.set(start_node.cooldown);
                end_node.temp_charge.set(end_node.temp_charge.get() + pulse);
                start_node.since_last_fire.set(0);
                edge.last_fire.set(0);
            } else {
                edge.last_fire.set(edge.last_fire.get()+1);
            }
            if verbose {
                println!(
                    "Node {} fired on edge {}->{} with pulse {}",
                    start_node.id, start_node.id, end_node.id, pulse
                );
            }
        }
    }

    pub fn default_edge(start_node: &Rc<RefCell<Node>>, end_node: &Rc<RefCell<Node>>) -> Edge {
        Edge {
            out_percentage: random::<f64>(),
            out_fixed: random::<f64>() * 5.0,
            edge_health: Cell::new(3),
            last_fire: Cell::new(0),
            fire_within: 5,
            end_node_fire_within: 3,
            start_node: Rc::clone(start_node),
            end_node: Rc::clone(end_node),
        }
    }

    fn inject_node(node: &Node, input: f64) {
        node.charge.set(node.charge.get() + input);
    }

    pub fn inject_node_index(&self, index: usize, input: f64) {
        let node = &self.nodes[index].borrow_mut();
        MorassWeb::inject_node(&node, input);
    }


    // Show the current charge of all nodes
    pub fn show_nodes(&self) {
        for node in &self.nodes {
            let node = node.borrow();
            println!("Node {} has charge {}", node.id, node.charge.get());
        }
    }

    // Show the topology of the network
    pub fn show_edges(&self) {
        for edge in &self.edges {
            let edge = edge.borrow();
            println!(
                "Edge {}->{} has out_percentage {} and out_fixed {}",
                edge.start_node.borrow().id,
                edge.end_node.borrow().id,
                edge.out_percentage,
                edge.out_fixed
            );
        }
    }

    pub fn show_op_counter(&self) -> usize {
        self.op_counter
    }

    pub fn add_edges_to_random_node(&mut self, num_edges: usize, max_tries: usize) {
        let mut target_node = random::<usize>() % self.nodes.len();
        let mut tries = 1;
        while self.nodes[target_node].borrow().num_outgoing_edges.get() + num_edges >= self.nodes.len() {
            // If the target node has too many outgoing edges, pick a new target node
            target_node = random::<usize>() % self.nodes.len();
        }
        let mut new_pairs = HashSet::<(usize, usize)>::new();
        for _ in 0..num_edges {
            let mut pair = (random::<usize>() % self.nodes.len(), random::<usize>() % self.nodes.len());
            while pair.0 == pair.1 ||
                self.pairs.contains(&pair) ||
                self.pairs.contains(&(pair.1, pair.0)) {
                pair = (target_node, random::<usize>() % self.nodes.len());
                tries += 1;
                if tries > max_tries {
                    return;
                }
            }
            self.pairs.insert(pair);
            new_pairs.insert(pair);
        }
        for pair in new_pairs.iter() {
            // Create edge with random parameters
            let edge = MorassWeb::default_edge(self.nodes.get(pair.0).unwrap(),
                                               self.nodes.get(pair.1).unwrap());
            self.edges.push(Rc::new(RefCell::new(edge)));
        }
    }

    pub fn show_node_counter(&self) -> usize {
        self.nodes.len()
    }

    pub fn show_edge_counter(&self) -> usize {
        self.edges.len()
    }
}
