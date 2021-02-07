use crate::{
    binding::{Access, Instance},
    error::CreateError,
    param::ParamHashMap,
};
use sched::{
    atomic::Atomic,
    binding::{bpm::ClockData, ParamBinding, ParamBindingGet},
    tick::{TickResched, TickSched},
    Float,
};
use serde::Serialize;
use serde_json::value::Value as JsonValue;
use std::{collections::HashMap, sync::Arc};

/// Result from attempt to create an instance.
pub type InstDataResult = Result<(Access, ParamHashMap), CreateError>;

/// Instance Factory Function type.
pub type InstDataFn = dyn Fn(JsonValue) -> InstDataResult + Sync;

/// Instance Factory Item.
#[derive(Serialize)] //just for display
pub struct InstFactItem {
    /// Factory function.
    #[serde(skip_serializing)]
    func: Box<InstDataFn>,
    /// Description
    desc: String,
    /// Example Argument
    example_args: Option<String>,
}

impl InstFactItem {
    pub fn new<D>(func: Box<InstDataFn>, description: D, example_args: Option<String>) -> Self
    where
        D: ToString,
    {
        Self {
            func,
            desc: description.to_string(),
            example_args,
        }
    }

    pub fn create(&self, args: JsonValue) -> InstDataResult {
        (self.func)(args)
    }

    /// Get the description.
    pub fn description(&self) -> &str {
        &self.desc
    }

    /// Get the argument example.
    pub fn example_args(&self) -> Option<&str> {
        self.example_args.as_deref()
    }
}

pub fn create_instance(
    uuid: uuid::Uuid,
    type_name: &str,
    args: JsonValue,
) -> Result<Instance, CreateError> {
    if let Some((key, f)) = INSTANCE_FACTORY_HASH.get_key_value(type_name) {
        match f.create(args) {
            Ok((access, map)) => Ok(Instance::new_with_id(key, access, map, uuid)),
            Err(e) => Err(e),
        }
    } else {
        Err(CreateError::TypeNotFound)
    }
}

pub fn help() -> serde_json::Value {
    serde_json::to_value(&*INSTANCE_FACTORY_HASH).expect("failed to serialize")
}

//pull in the codegen
include!(concat!(env!("OUT_DIR"), "/instance_factory.rs"));
