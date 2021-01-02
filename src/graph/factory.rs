use crate::{
    error::CreateError,
    graph::GraphItem,
    param::{ParamAccess, ParamGet},
};
use sched::{
    binding::{
        bpm::{Clock, ClockData},
        swap::BindingSwapGet,
        ParamBindingGet,
    },
    graph::root_clock::RootClock,
    Float,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

pub fn create_instance(
    uuid: uuid::Uuid,
    type_name: &str,
    _args: &str,
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
    } else {
        Err(CreateError::TypeNotFound)
    }
}
