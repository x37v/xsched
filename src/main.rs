use sched::binding::{
    last::{BindingLastGet},
    ParamBindingGet,
};
use xsched::{
    binding::{Access, Instance},
    graph::GraphItem,
    jack::Jack,
    oscquery::OSCQueryHandler,
    param::{ParamAccess, ParamGet},
    sched::Sched,
};

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

fn main() -> Result<(), std::io::Error> {
    let run = Arc::new(AtomicBool::new(true));
    //gracefully handle control-c
    {
        let run = run.clone();
        ctrlc::set_handler(move || {
            run.store(false, Ordering::Release);
        })
        .expect("Error setting Ctrl-C handler");
    }

    let bindings: HashMap<String, Arc<Instance>> = Default::default();
    let graph: HashMap<String, GraphItem> = Default::default();

    let sched = Sched::new();
    let _jack = Jack::new(sched);
    let server = OSCQueryHandler::new(bindings, graph)?;
    server.add_binding(Arc::new(Instance::new(
        &"value",
        std::sync::atomic::AtomicUsize::new(2084),
        HashMap::new(),
    )));

    let lswap = Arc::new(sched::binding::swap::BindingSwapGet::default());
    let rswap = Arc::new(sched::binding::swap::BindingSwapGet::default());
    let max = sched::binding::ops::GetBinaryOp::new(
        core::cmp::max,
        lswap.clone() as Arc<dyn ParamBindingGet<usize>>,
        rswap.clone() as Arc<dyn ParamBindingGet<usize>>,
    );

    let mut map = HashMap::new();
    map.insert("left", ParamAccess::new_get(ParamGet::USize(lswap)));
    map.insert("right", ParamAccess::new_get(ParamGet::USize(rswap)));

    server.add_binding(Arc::new(Instance::new(
        &"max",
        Access::USizeGet(Arc::new(BindingLastGet::new(max))),
        map,
    )));

    server.add_binding(Arc::new(Instance::new(
        &"value",
        sched::binding::bpm::ClockData::default(),
        HashMap::new(),
    )));

    server.add_binding(Arc::new(Instance::new(
        &"value",
        sched::tick::TickResched::Relative(20),
        HashMap::new(),
    )));

    while run.load(Ordering::Acquire) {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    Ok(())
}
