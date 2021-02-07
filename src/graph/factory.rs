use crate::{
    error::CreateError,
    graph::GraphItem,
    param::{ParamAccess, ParamGet},
    sched::QueueSource,
};
use sched::{
    binding::{
        bpm::{Clock, ClockData},
        swap::BindingSwapGet,
        ParamBindingGet,
    },
    graph as sgraph,
    tick::TickResched,
    Float,
};
use serde_json::value::Value as JsonValue;
use std::{collections::HashMap, sync::Arc};

pub fn create_instance(
    uuid: &uuid::Uuid,
    type_name: &str,
    _args: Option<JsonValue>,
    queue_sources: &Arc<dyn QueueSource>,
) -> Result<GraphItem, CreateError> {
    let uuid = uuid.clone();
    //TODO build.rs BindStoreNode for all the binding types
    match type_name {
        "root::clock" => {
            let clock = ClockData::default();
            let micros: Arc<BindingSwapGet<Float>> =
                Arc::new(BindingSwapGet::new(clock.period_micros()));
            let mut map = HashMap::new();
            map.insert(
                "period_micros",
                ParamAccess::new_get(ParamGet::Float(micros.clone())),
            );
            Ok(GraphItem::new_root(
                &"root::clock",
                sgraph::root_clock::RootClock::new(
                    micros.clone() as Arc<dyn ParamBindingGet<Float>>
                ),
                map,
                Some(uuid),
            ))
        }
        "node::clock_ratio" => {
            let mul: Arc<BindingSwapGet<usize>> = Arc::new(BindingSwapGet::new(1));
            let div: Arc<BindingSwapGet<usize>> = Arc::new(BindingSwapGet::new(1));
            let mut map = HashMap::new();
            map.insert("mul", ParamAccess::new_get(ParamGet::USize(mul.clone())));
            map.insert("div", ParamAccess::new_get(ParamGet::USize(div.clone())));

            let node = sgraph::clock_ratio::ClockRatio::new(
                mul as Arc<dyn ParamBindingGet<usize>>,
                div as Arc<dyn ParamBindingGet<usize>>,
            );
            Ok(GraphItem::new_node(
                &"node::clock_ratio",
                node,
                map,
                Some(uuid),
            ))
        }
        "node::gate" => {
            let gate: Arc<BindingSwapGet<bool>> = Arc::new(BindingSwapGet::new(false));
            let mut map = HashMap::new();
            map.insert("gate", ParamAccess::new_get(ParamGet::Bool(gate.clone())));

            let node = sgraph::gate::Gate::new(gate as Arc<dyn ParamBindingGet<bool>>);
            Ok(GraphItem::new_node(&"node::gate", node, map, Some(uuid)))
        }
        "node::one_hot" => {
            let sel: Arc<BindingSwapGet<usize>> = Arc::new(BindingSwapGet::new(0));
            let mut map = HashMap::new();
            map.insert("sel", ParamAccess::new_get(ParamGet::USize(sel.clone())));
            let node = sgraph::one_hot::OneHot::new(sel as Arc<dyn ParamBindingGet<usize>>);
            Ok(GraphItem::new_node(&"node::one_hot", node, map, Some(uuid)))
        }
        "node::fanout" => {
            let node = sgraph::fanout::FanOut::new();
            Ok(GraphItem::new_node(
                &"node::fanout",
                node,
                HashMap::new(),
                Some(uuid),
            ))
        }
        "node::step_seq" => {
            let step_ticks: Arc<BindingSwapGet<usize>> = Arc::new(BindingSwapGet::new(16));
            let steps: Arc<BindingSwapGet<usize>> = Arc::new(BindingSwapGet::new(16));
            let mut map = HashMap::new();
            map.insert(
                "step_ticks",
                ParamAccess::new_get(ParamGet::USize(step_ticks.clone())),
            );
            map.insert(
                "steps",
                ParamAccess::new_get(ParamGet::USize(steps.clone())),
            );

            let node = sgraph::step_seq::StepSeq::new(
                step_ticks as Arc<dyn ParamBindingGet<usize>>,
                steps as Arc<dyn ParamBindingGet<usize>>,
            );
            Ok(GraphItem::new_node(
                &"node::step_seq",
                node,
                map,
                Some(uuid),
            ))
        }
        "leaf::midi::note" => {
            let chan: Arc<BindingSwapGet<u8>> = Arc::new(BindingSwapGet::new(0));
            let num: Arc<BindingSwapGet<u8>> = Arc::new(BindingSwapGet::new(64));
            let on_vel: Arc<BindingSwapGet<u8>> = Arc::new(BindingSwapGet::new(127));
            let off_vel: Arc<BindingSwapGet<u8>> = Arc::new(BindingSwapGet::new(127));
            let dur: Arc<BindingSwapGet<TickResched>> =
                Arc::new(BindingSwapGet::new(TickResched::ContextRelative(1)));

            //setup parameters
            let mut map = HashMap::new();
            map.insert("chan", ParamAccess::new_get(ParamGet::U8(chan.clone())));
            map.insert("num", ParamAccess::new_get(ParamGet::U8(num.clone())));
            map.insert("on_vel", ParamAccess::new_get(ParamGet::U8(on_vel.clone())));
            map.insert(
                "off_vel",
                ParamAccess::new_get(ParamGet::U8(off_vel.clone())),
            );
            map.insert(
                "dur",
                ParamAccess::new_get(ParamGet::TickResched(dur.clone())),
            );

            let note = sgraph::midi::MidiNote::new(
                chan as Arc<dyn ParamBindingGet<u8>>,
                num as Arc<dyn ParamBindingGet<u8>>,
                dur as Arc<dyn ParamBindingGet<TickResched>>,
                on_vel as Arc<dyn ParamBindingGet<u8>>,
                off_vel as Arc<dyn ParamBindingGet<u8>>,
                queue_sources.midi_event_source(),
                queue_sources.midi_queue() as _,
            );
            Ok(GraphItem::new_leaf(
                &"leaf::midi::note",
                note,
                map,
                Some(uuid),
            ))
        }
        _ => Err(CreateError::TypeNotFound),
    }
}
