use xsched::{graph::GraphItem, jack::Jack, oscquery::OSCQueryHandler, param::Param, sched::Sched};

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

    let bindings: HashMap<String, Arc<Param>> = Default::default();
    let graph: HashMap<String, GraphItem> = Default::default();

    let sched = Sched::new();
    let queue_sources = sched.queue_sources();
    let _jack = Jack::new(sched);
    let mut server = OSCQueryHandler::new(queue_sources, bindings, graph)?;

    /*
    server.add_binding(Arc::new(Param::new(
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

    server.add_binding(Arc::new(Param::new(
        &"max",
        Access::USizeGet(Arc::new(BindingLastGet::new(max))),
        map,
    )));

    server.add_binding(Arc::new(Param::new(
        &"value",
        sched::binding::bpm::ClockData::default(),
        HashMap::new(),
    )));

    server.add_binding(Arc::new(Param::new(
        &"value",
        sched::tick::TickResched::Relative(20),
        HashMap::new(),
    )));
    */

    let help = xsched::param::factory::help().to_string();
    println!("param help {}", help);

    while run.load(Ordering::Acquire) {
        server.process();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    Ok(())
}
