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
use std::{collections::HashMap, sync::Arc};

pub fn create_instance(
    uuid: uuid::Uuid,
    type_name: &str,
    _args: &str,
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
        //TODO bind values
        let map = HashMap::new();
        let note = ::sched::graph::midi::MidiNote::new(
            &0,
            64,
            &TickResched::ContextRelative(1),
            &127,
            &127,
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
