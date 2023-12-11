mod web;

// Timer
use std::time::Instant;

fn main() {
    let start = Instant::now();
    let mut w = web::MorassWeb::make_random_web(100, 1000);
    let mut last_ops = 0;

    for step in 0..100_000 {
        w.step(false);
        if step % 100 == 0 {
            println!("Step {} had {} operations", step, w.show_op_counter()-last_ops);
            println!("The current web has {} nodes and {} edges", w.show_node_counter(), w.show_edge_counter());
        }
        last_ops = w.show_op_counter();
        w.add_edges_to_random_node(10, 10)
    }

    println!(
        "{} operations took {}ms",
        w.show_op_counter(),
        start.elapsed().as_millis()
    );
}
