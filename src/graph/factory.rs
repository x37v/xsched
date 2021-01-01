use crate::{error::CreateError, graph::GraphItem};
use sched::graph::root_clock::RootClock;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

pub fn create_instance(
    uuid: uuid::Uuid,
    type_name: &str,
    args: &str,
) -> Result<GraphItem, CreateError> {
    if type_name == "root::clock" {
        //XXX
    }
    Err(CreateError::TypeNotFound)
}
