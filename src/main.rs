mod jack;
mod sched;
use crate::jack::Jack;
use crate::sched::Sched;

fn main() {
    let sched = Sched::new();
    let jack = Jack::new(sched);
}
