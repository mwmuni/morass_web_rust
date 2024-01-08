use rand::random;
use std::cmp::max;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::ops::{Add, AddAssign, DerefMut};
use rayon::prelude::*;
use std::sync::{Arc, RwLock};
use rand::distributions::uniform::SampleBorrow;

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
#[derive(PartialEq)]
pub struct Node {
    id: usize,
    threshold: f64,
    charge: f64,
    cooldown: usize,
    cooldown_remaining: usize,
    since_last_fire: usize,
    charge_consumption_percentage: f64,
    charge_consumption_fixed: f64,
    decay_percentage: f64,
    decay_fixed: f64,
}

impl Hash for Node {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Eq for Node {}



// An edge is defined with the following parameters:
//
// Op: Out percent; when a threshold is triggered, Op is multiplied by the pulse value of the start node and sent to the end node
// Of: Out fixed; when a threshold is triggered, Of is added to Op and sent to the end node
// start_node: the start node of the directed edge
// end_node: the end node of the directed edge
pub struct Edge {
    out_percentage: f64,
    out_fixed: f64,
    edge_health: usize, // Number of failed steps before the edge is removed
    last_fire: usize, // Number of steps since the edge last fired
    fire_within: usize, // Number of steps that the start_node needs to fire within otherwise edge_health is decremented
    end_node_fire_within: usize, // Number of steps that the end_node needs to fire within otherwise edge_health is decremented
    start_node: Arc<RwLock<Node>>,
    end_node: Arc<RwLock<Node>>,
}

pub struct MorassWeb {
    nodes: Vec<Arc<RwLock<Node>>>,
    edges: Vec<Arc<RwLock<Edge>>>,
    node_temp_charges: Vec<Arc<RwLock<f64>>>,
    node_last_fired: Vec<Arc<RwLock<usize>>>,
    pairs: Arc<RwLock<HashSet<(usize, usize)>>>, // usize representation of edges
    op_counter: Arc<RwLock<usize>>,
    edges_added_counter: Arc<RwLock<usize>>,
}



impl MorassWeb {
    pub fn make_random_web(num_nodes: usize, num_edges: usize) -> Self {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut node_temp_charges = Vec::new();
        let mut node_last_fired = Vec::new();

        for n in 0..num_nodes {
            // Create node with random parameters
            let node = Node {
                id: n + 1,
                threshold: random::<f64>() * 10.0,
                charge: random::<f64>() * 5.0,
                cooldown: random::<usize>() % 5 + 1,
                cooldown_remaining: 0,
                since_last_fire: 0,
                charge_consumption_percentage: random::<f64>() * 20.0,
                charge_consumption_fixed: random::<f64>() * 3.0,
                decay_percentage: random::<f64>() * 0.05,
                decay_fixed: random::<f64>() * 0.2,
            };
            let rc_node = Arc::new(RwLock::new(node));
            nodes.push(Arc::clone(&rc_node));
        }

        for _ in 0..num_nodes {
            node_temp_charges.push(Arc::new(RwLock::new(0.0)));
            node_last_fired.push(Arc::new(RwLock::new(0)));
        }

        // Make n random pairs of integers where n=num_edges and each integer is in the range [0, num_nodes)
        // Each pair is unique, (i, i) is not allowed, and (i, j) is the same as (j, i)
        let pairs: Arc<RwLock<HashSet<(usize, usize)>>> = Arc::new(RwLock::new(HashSet::new()));
        let mut tries = 0;
        for _ in 0..num_edges {
            let pair = loop {
                tries += 1;
                let ret = (random::<usize>() % num_nodes, random::<usize>() % num_nodes);
                if ret.0 != ret.1 && !pairs.read().unwrap().contains(&ret) && !pairs.read().unwrap().contains(&(ret.1, ret.0)) {
                    break ret;
                };
                if tries > 1000 {
                    println!("Could not find {} unique pairs", num_edges);
                    println!("Found {} unique pairs", pairs.read().unwrap().len());
                    return Self {
                        nodes,
                        edges,
                        node_temp_charges,
                        node_last_fired,
                        pairs,
                        op_counter: Arc::new(RwLock::new(0)),
                        edges_added_counter: Arc::new(RwLock::new(0)),
                    };
                }
            };
            pairs.write().unwrap().insert(pair);
        }

        // Create edge with random parameters
        for pair in pairs.read().unwrap().iter() {
            // Create edge with random parameters
            let edge = MorassWeb::default_edge(nodes.get(pair.0).unwrap(),
                                               nodes.get(pair.1).unwrap());
            edges.push(Arc::new(RwLock::new(edge)));
        }

        Self {
            nodes,
            edges,
            node_temp_charges,
            node_last_fired,
            pairs,
            op_counter: Arc::new(RwLock::new(0)),
            edges_added_counter: Arc::new(RwLock::new(0)),
        }
    }

    fn assimilate(&mut self, node: &Arc<RwLock<Node>>) {
        let mut node = node.write().unwrap();
        node.charge = self.node_temp_charges[node.id-1].read().unwrap().add(node.charge);
        self.node_temp_charges[node.id-1].write().unwrap().clone_from(&0.0);
    }

    fn pulse(&mut self, edge: &Arc<RwLock<Edge>>, verbose: bool) -> bool {
        // Read phase
        let (start_node_charge, start_node_threshold, start_node_cooldown, out_percentage, out_fixed, end_node_cooldown, end_node_since_last_fire, last_fire) = {
            let edge_read = edge.read().unwrap();
            let start_node_read = edge_read.start_node.read().unwrap();

            // If the start node or end node is on cooldown, skip it
            if start_node_read.cooldown_remaining > 0 {
                return false;
            }
            let end_node_read = edge_read.end_node.read().unwrap();

            (
                start_node_read.charge,
                start_node_read.threshold,
                start_node_read.cooldown,
                edge_read.out_percentage,
                edge_read.out_fixed,
                end_node_read.cooldown,
                end_node_read.since_last_fire,
                edge_read.last_fire,
            )
        };

        // Compute pulse
        let pulse = if start_node_charge >= start_node_threshold {
            start_node_charge * out_percentage + out_fixed
        } else {
            0.0
        };

        // Write phase
        if pulse > 0.0 {
            'startnode: loop {
                let edge_read = edge.read().unwrap();
                let start_node_lock = edge_read.start_node.read();
                if start_node_lock.is_err() {
                    continue 'startnode;
                }
                let read_start_node = start_node_lock.unwrap();
                self.node_temp_charges[read_start_node.id-1].write().unwrap().add_assign(&pulse);
                self.node_last_fired[read_start_node.id-1].write().unwrap().clone_from(&0);
                break 'startnode;
            }
            'endnode: loop {
                let edge_read = edge.read().unwrap();
                let end_node_lock = edge_read.end_node.read();
                if end_node_lock.is_err() {
                    continue 'endnode;
                }
                let read_end_node = end_node_lock.unwrap();
                self.node_temp_charges[read_end_node.id-1].write().unwrap().add_assign(&pulse);
                self.node_last_fired[read_end_node.id-1].write().unwrap().clone_from(&0);
                break 'endnode;
            }
            {
                let mut write_edge = edge.write().unwrap();
                write_edge.last_fire = 0;
            }

            if verbose {
                println!(
                    "Node {} fired on edge {}->{} with pulse {}",
                    edge.read().unwrap().start_node.read().unwrap().id,
                    edge.read().unwrap().start_node.read().unwrap().id,
                    edge.read().unwrap().end_node.read().unwrap().id,
                    pulse
                );
            }

            true
        } else {
            let mut write_edge = edge.write().unwrap();
            write_edge.last_fire = last_fire + 1;
            false
        }
    }


    pub fn step(&mut self, verbose: bool) {
        let op_counter: usize = self.edges.par_iter()
            .map(|edge| {
                if self.pulse(edge, verbose) { 1 } else { 0 }
            })
            .sum();

        self.nodes.par_iter().for_each(|node| {
            // println!("About to subtract charge");
            MorassWeb::subtract_charge(node);
            // println!("About to decay");
            MorassWeb::decay(node);
            // println!("About to assimilate");
            self.assimilate(node);
            // println!("About to cooldown");
            MorassWeb::cooldown_step(node);
        });

        self.edges.par_iter().for_each(|edge| {
            // println!("About to penalise");
            MorassWeb::penalise(edge);
        });
        // println!("About to retain");
        self.pairs.write().unwrap().retain(|pair| {
            let start_node = self.nodes.get(pair.0).unwrap();
            let end_node = self.nodes.get(pair.1).unwrap();
            let start_node_read = start_node.read().unwrap();
            let end_node_read = end_node.read().unwrap();
            start_node_read.cooldown_remaining <= 0 && end_node_read.cooldown_remaining <= 0
        });

        // Handling self.edges.retain in parallel might be complex due to mutable references
        self.edges.retain(|edge| edge.read().unwrap().edge_health > 0);
        let mut op_lock = self.op_counter.write().unwrap();
        *op_lock += op_counter;
        // println!("finished step");
    }

    fn cooldown_step(node: &Arc<RwLock<Node>>) {
        let mut node = node.write().unwrap();
        if node.cooldown_remaining > 0 {
            node.cooldown_remaining = node.cooldown_remaining - 1;
        }
    }

    fn penalise(edge: &Arc<RwLock<Edge>>) {
        let mut edge = edge.write().unwrap();
        if edge.end_node.read().unwrap().since_last_fire == edge.end_node_fire_within { // Only penalise once
            edge.edge_health = max(edge.edge_health, 1) - 1;
        }
        if edge.last_fire % edge.fire_within == edge.fire_within-1 {
            edge.edge_health = max(edge.edge_health, 1) - 1;
        }
    }



    fn decay(node: &Arc<RwLock<Node>>) {
        let mut node = node.write().unwrap();
        node.charge = node.charge - node.charge * node.decay_percentage - node.decay_fixed;
    }

    fn subtract_charge(node: &Arc<RwLock<Node>>) {
        // If the node is on cooldown, skip it
        let mut node = node.write().unwrap();
        if node.cooldown_remaining <= 0 {
            if node.charge >= node.threshold {
                node.charge =
                    node.charge
                        - node.charge * node.charge_consumption_percentage
                        - node.charge_consumption_fixed;
            }
        }
    }



    pub fn default_edge(start_node: &Arc<RwLock<Node>>, end_node: &Arc<RwLock<Node>>) -> Edge {
        Edge {
            out_percentage: random::<f64>(),
            out_fixed: random::<f64>() * 5.0,
            edge_health: 3,
            last_fire: 0,
            fire_within: 5,
            end_node_fire_within: 3,
            start_node: Arc::clone(start_node),
            end_node: Arc::clone(end_node),
        }
    }

    pub fn inject_node_index(&self, index: usize, input: f64) {
        let mut node = self.nodes[index].write().unwrap();
        node.charge += input;
    }


    // Show the current charge of all nodes
    pub fn show_nodes(&self) {
        for node in &self.nodes {
            let node = node.read().unwrap();
            println!("Node {} has charge {}", node.id, node.charge);
        }
    }

    // Show the topology of the network
    pub fn show_edges(&self) {
        for edge in &self.edges {
            let edge = edge.read().unwrap();
            println!(
                "Edge {}->{} has out_percentage {} and out_fixed {}",
                edge.start_node.read().unwrap().id,
                edge.end_node.read().unwrap().id,
                edge.out_percentage,
                edge.out_fixed
            );
        }
    }

    pub fn get_op_counter(&self) -> usize {
        self.op_counter.read().unwrap().clone()
    }

    pub fn get_added_edges(&self) -> usize {
        self.edges_added_counter.read().unwrap().clone()
    }

    pub fn add_edges_to_random_node(&mut self, num_edges: usize, max_tries: usize) {
        let mut tries = 0;
        let prior_total_edges = self.edges.len();
        // Tally the number of outgoing edges for each node
        let mut arr_count = vec![0; self.nodes.len()];
        for edge in &self.edges {
            let edge = edge.read().unwrap();
            arr_count[edge.start_node.read().unwrap().id - 1] += 1;
        }

        // Identify the nodes that can have edges added
        let available_nodes: Vec<usize> =
            arr_count.iter().enumerate().filter_map(|(i, &x)| if x < self.nodes.len() - 1 {
                Some(i)
            } else {
                None
            }).collect();
        if available_nodes.len() == 0 {
            // println!("No nodes can have any more edges");
            return;
        }
        'main: while tries < max_tries {
            // Randomly pick from the available nodes
            let _available_target_node_index = random::<usize>() % available_nodes.len();
            let target_node_index = available_nodes[_available_target_node_index];
            let mut loop_tries = 0;
            // Calculate the number of existing edges
            let existing_edges = arr_count[target_node_index];

            let unconnected_nodes: Vec<usize> = (0..self.nodes.len()).filter_map(|i| {
                if !self.pairs.read().unwrap().contains(&(target_node_index, i+1)) {
                    Some(i)
                } else {
                    None
                }
            }).collect();

            // Calculate the maximum number of new edges that can be added
            let max_new_edges = unconnected_nodes.len();
            let edges_to_add = std::cmp::min(num_edges, max_new_edges);

            if edges_to_add == 0 {
                // This node cannot have any more edges, try another node
                tries += 1;
                continue 'main;
            }

            for i in 0..edges_to_add {
                let end_node_index = unconnected_nodes[i];

                let edge = MorassWeb::default_edge(
                    &self.nodes[target_node_index],
                    &self.nodes[end_node_index]
                );
                self.edges.push(Arc::new(RwLock::new(edge)));
                self.pairs.write().unwrap().insert((target_node_index, end_node_index));
                let mut edge_count_lock = self.edges_added_counter.write().unwrap();
                *edge_count_lock += 1;
            }

            if self.edges.len() == prior_total_edges {
                // No edges were added, try again
                tries += 1;
                continue 'main;
            }

            // Successfully added edges, no need to try more
            break;
        }
    }

    pub fn show_node_counter(&self) -> usize {
        self.nodes.len()
    }

    pub fn show_edge_counter(&self) -> usize {
        self.edges.len()
    }

}
