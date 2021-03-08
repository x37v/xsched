use crate::{
    error::CreateError,
    param::{Param, ParamDataAccess, ParamHashMap},
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

/// Result from attempt to create a param.
pub type ParamDataResult =
    Result<(ParamDataAccess, Option<ParamDataAccess>, ParamHashMap), CreateError>;

/// Param Factory Function type.
pub type ParamDataFn = dyn Fn(JsonValue) -> ParamDataResult + Sync;

/// Param Factory Item.
#[derive(Serialize)] //just for display
pub struct ParamFactItem {
    /// Factory function.
    #[serde(skip_serializing)]
    func: Box<ParamDataFn>,
    /// Description
    desc: String,
    /// Example Argument
    example_args: Option<String>,
}

impl ParamFactItem {
    pub fn new<D>(func: Box<ParamDataFn>, description: D, example_args: Option<String>) -> Self
    where
        D: ToString,
    {
        Self {
            func,
            desc: description.to_string(),
            example_args,
        }
    }

    pub fn create(&self, args: JsonValue) -> ParamDataResult {
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

pub fn create_param(
    uuid: &uuid::Uuid,
    type_name: &str,
    args: JsonValue,
) -> Result<Param, CreateError> {
    if let Some((key, f)) = PARAM_FACTORY_HASH.get_key_value(type_name) {
        match f.create(args) {
            Ok((access, shadow, map)) => Ok(Param::new_with_id(key, access, map, shadow, uuid)),
            Err(e) => Err(e),
        }
    } else {
        Err(CreateError::TypeNotFound)
    }
}

pub fn help() -> serde_json::Value {
    serde_json::to_value(&*PARAM_FACTORY_HASH).expect("failed to serialize")
}

//pull in the codegen
include!(concat!(env!("OUT_DIR"), "/instance_factory.rs"));
