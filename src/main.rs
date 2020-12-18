use sched::binding::ParamBindingGet;
use xsched::{
    binding::Instance, graph::GraphItem, jack::Jack, oscquery::OSCQueryHandler, sched::Sched,
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
    server.add_binding(Instance::new(
        &"value",
        xsched::binding::Access::Get(xsched::binding::Get::USize(Arc::new(Arc::new(
            std::sync::atomic::AtomicUsize::new(0),
        )
            as Arc<dyn ParamBindingGet<usize>>))),
        HashMap::new(),
    ));

    server.add_binding(Instance::new(
        &"value",
        xsched::binding::Access::Get(xsched::binding::Get::ISize(Arc::new(Arc::new(
            std::sync::atomic::AtomicIsize::new(-2),
        )
            as Arc<dyn ParamBindingGet<isize>>))),
        HashMap::new(),
    ));

    while run.load(Ordering::Acquire) {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    Ok(())
}
