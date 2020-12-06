use xsched::jack::Jack;
use xsched::sched::Sched;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() {
    let run = Arc::new(AtomicBool::new(true));
    //gracefully handle control-c
    {
        let run = run.clone();
        ctrlc::set_handler(move || {
            run.store(false, Ordering::Release);
        })
        .expect("Error setting Ctrl-C handler");
    }

    let sched = Sched::new();
    let jack = Jack::new(sched);
    while run.load(Ordering::Acquire) {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
