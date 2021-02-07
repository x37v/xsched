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
    graph::root_clock::RootClock,
    tick::TickResched,
    Float,
};
use serde_json::value::Value as JsonValue;
use std::{collections::HashMap, sync::Arc};

pub fn create_instance(
    uuid: uuid::Uuid,
    type_name: &str,
    _args: Option<JsonValue>,
    queue_sources: &Arc<dyn QueueSource>,
) -> Result<GraphItem, CreateError> {
    if type_name == "root::clock" {
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
            RootClock::new(micros.clone() as Arc<dyn ParamBindingGet<Float>>),
            map,
            Some(uuid),
        ))
    } else if type_name == "leaf::midi::note" {
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

        let note = ::sched::graph::midi::MidiNote::new(
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
    } else {
        Err(CreateError::TypeNotFound)
    }
}
