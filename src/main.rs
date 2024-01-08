mod web;

// Timer
use std::time::Instant;

fn main() {
    let start = Instant::now();
    let mut w = web::MorassWeb::make_random_web(10, 20);
    let mut last_ops = 0;
    let mut last_added_edges = 0;

    for step in 0..100_000 {
        // println!("about to step");
        w.step(false);
        // println!("stepped");
        if step % 1000 == 0 {
            println!("Step {} had {} operations and added {} edges", step,
                     w.get_op_counter()-last_ops,
                     w.get_added_edges()-last_added_edges);
            println!("The current web has {} nodes and {} edges", w.show_node_counter(), w.show_edge_counter());
        }
        last_ops = w.get_op_counter();
        last_added_edges = w.get_added_edges();
        // println!("About to add 5 edges to a random node");
        w.add_edges_to_random_node(5, 1000);
        // println!("Finished adding edges");
    }

    println!(
        "{} operations took {}ms",
        w.get_op_counter(),
        start.elapsed().as_millis()
    );
}
