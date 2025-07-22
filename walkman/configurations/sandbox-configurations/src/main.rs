use std::sync::Arc;
use tokio::task;

struct Worker;

impl Worker {
    async fn do_background(self: Arc<Self>) {
        for i in 0..3 {
            let this = Arc::clone(&self); // ✅ cheap, cloneable, 'static
            task::spawn(async move {
                this.do_work(i).await;
            });
        }
    }

    async fn do_work(&self, id: usize) {
        println!("Worker {} doing work", id);
    }
}

#[tokio::main]
async fn main() {
    let worker = Arc::new(Worker);
    worker.do_background().await;
}
