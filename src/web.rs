use rand::random;
use std::cell::Cell;
use std::cell::RefCell;
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
    temp_charge: Cell<f64>,
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
    start_node: Rc<RefCell<Node>>,
    end_node: Rc<RefCell<Node>>,
}

pub struct MorassWeb {
    nodes: Vec<Rc<RefCell<Node>>>,
    edges: Vec<Rc<RefCell<Edge>>>,
    op_counter: usize,
}

pub fn step(web: &mut MorassWeb, verbose: bool) {
    // Process all node threshold outputs to connected nodes and store the output in a temporary variable in the destination node
    for edge in &web.edges {
        let edge = edge.borrow();
        let start_node = edge.start_node.borrow();
        let end_node = edge.end_node.borrow_mut();

        let pulse = if start_node.charge.get() >= start_node.threshold {
            start_node.charge.get() * edge.out_percentage + edge.out_fixed
        } else {
            0.0
        };
        if pulse > 0.0 {
            web.op_counter += 1;
        }
        end_node.temp_charge.set(end_node.temp_charge.get() + pulse);
        if verbose {
            println!(
                "Node {} fired on edge {}->{} with pulse {}",
                start_node.id, start_node.id, end_node.id, pulse
            );
        }
    }

    // Subtraction of charge from nodes if fired
    for node in &web.nodes {
        let node = node.borrow_mut();
        if node.charge.get() >= node.threshold {
            node.charge.set(
                node.charge.get()
                    - node.charge.get() * node.charge_consumption_percentage
                    - node.charge_consumption_fixed,
            );
        }
    }

    // Decay all the nodes by some percentage and fixed amount
    for node in &web.nodes {
        let node = node.borrow_mut();
        node.charge
            .set(node.charge.get() - node.charge.get() * node.decay_percentage - node.decay_fixed);
    }

    // Assimilate the temporary charge from step 1 with the current charge
    for node in &web.nodes {
        let node = node.borrow_mut();
        node.charge.set(node.charge.get() + node.temp_charge.get());
        node.temp_charge.set(0.0);
    }
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
                temp_charge: Cell::new(0.0),
                charge_consumption_percentage: random::<f64>() * 20.0,
                charge_consumption_fixed: random::<f64>() * 3.0,
                decay_percentage: random::<f64>() * 0.05,
                decay_fixed: random::<f64>() * 0.2,
            };
            nodes.push(Rc::new(RefCell::new(node)));
        }

        // Make n random pairs of integers where n=num_edges and each integer is in the range [0, num_nodes)
        // Each pair is unique, (i, i) is not allowed, and (i, j) is the same as (j, i)
        let mut pairs = Vec::<(usize, usize)>::new();
        for _ in 0..num_edges {
            let mut pair = (random::<usize>() % num_nodes, random::<usize>() % num_nodes);
            while pair.0 == pair.1 || pairs.contains(&pair) {
                pair = (random::<usize>() % num_nodes, random::<usize>() % num_nodes);
            }
            pairs.push(pair);
        }
        pairs[0].0;

        for pair in pairs {
            // Create edge with random parameters
            let edge = Edge {
                out_percentage: random::<f64>(),
                out_fixed: random::<f64>() * 5.0,
                start_node: Rc::clone(&nodes[pair.0]),
                end_node: Rc::clone(&nodes[pair.1]),
            };
            edges.push(Rc::new(RefCell::new(edge)));
        }

        Self {
            nodes,
            edges,
            op_counter: 0,
        }
    }

    pub fn inject_node_index(&self, index: usize, input: f64) {
        let node = &self.nodes[index].borrow_mut();
        node.charge.set(node.charge.get() + input);
    }

    fn inject_node(node: &Node, input: f64) {
        node.charge.set(node.charge.get() + input);
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
}
