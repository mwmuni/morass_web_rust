mod web;

// Timer
use std::time::Instant;

fn main() {
    let start = Instant::now();
    let mut w = web::MorassWeb::make_random_web(1000, 10_000);

    for _ in 0..1000 {
        web::step(&mut w, false);
    }

    println!(
        "{} operations took {}ms",
        w.show_op_counter(),
        start.elapsed().as_millis()
    );
}
