//! Parameters: named typed bindings.

use crate::binding::Instance;
use sched::{
    binding::{
        bpm::ClockData,
        swap::{BindingSwapGet, BindingSwapSet},
    },
    mutex::Mutex,
    tick::{TickResched, TickSched},
    Float,
};
use std::{
    collections::{hash_map::Keys, HashMap},
    sync::Arc,
};

use sched::binding::{
    ParamBinding, ParamBindingGet, ParamBindingKeyValue, ParamBindingKeyValueGet,
    ParamBindingKeyValueSet, ParamBindingSet,
};

pub enum ParamDataAccess {
    Get(ParamDataGet),
    Set(ParamDataSet),
    GetSet(ParamDataGetSet),
    KeyValueGet(ParamDataKeyValueGet),
    KeyValueSet(ParamDataKeyValueSet),
    KeyValueGetSet(ParamDataKeyValueGetSet),
}

/// A trait to access an items parameters.
pub trait ParamMapGet {
    /// Get a reference to the parameters for this object.
    fn params(&self) -> &ParamHashMap;
}

/// Parameters with their access.
pub enum ParamAccess {
    Get {
        get: ParamGet,
        binding: Mutex<Option<Arc<Instance>>>,
    },
    Set {
        set: ParamSet,
        binding: Mutex<Option<Arc<Instance>>>,
    },
}

/// Errors in binding parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingError {
    /// No parameter with the given name
    KeyMissing,
    /// Input didn't have needed Get
    NoGet,
    /// Input didn't have needed Set
    NoSet,
}

#[derive(Default)]
pub struct ParamHashMap {
    inner: HashMap<&'static str, ParamAccess>,
}

pub struct Param {
    //a value to use only in the scheduler thread
    data: ParamDataAccess,
    //a value that you can safely get or set from non scheduler threads
    shadow: Option<ParamDataAccess>,
    //parameters that would alter this value
    params: ParamHashMap,
    uuid: uuid::Uuid,
    type_name: &'static str,
}

impl ParamHashMap {
    /// See if the key exists in this map.
    pub fn contains_key(&self, key: &str) -> bool {
        self.inner.contains_key(key)
    }

    /// Get the param keys.
    pub fn keys(&self) -> Keys<'_, &'static str, ParamAccess> {
        self.inner.keys()
    }

    /// See if there is anything in the map.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get the access name for the parameter with `name`.
    pub fn access_name(&self, name: &str) -> Option<&str> {
        if let Some(param) = self.inner.get(name) {
            Some(param.access_name())
        } else {
            None
        }
    }

    /// Get the daat type name for the parameter with `name`, if there is a param.
    pub fn data_type_name(&self, name: &str) -> Option<&'static str> {
        if let Some(param) = self.inner.get(name) {
            Some(param.data_type_name())
        } else {
            None
        }
    }

    /// Get the uuid of the bound parameter
    pub fn uuid(&self, name: &str) -> Option<uuid::Uuid> {
        if let Some(param) = self.inner.get(name) {
            param.uuid()
        } else {
            None
        }
    }

    pub fn unbind(&self, name: &str) -> Option<Arc<Instance>> {
        if let Some(param) = self.inner.get(name) {
            match param {
                ParamAccess::Get { get: g, binding: b } => {
                    let mut l = b.lock();
                    g.unbind();
                    l.take()
                }
                ParamAccess::Set { set: s, binding: b } => {
                    let mut l = b.lock();
                    s.unbind();
                    l.take()
                }
            }
        } else {
            None
        }
    }

    ///Bind the parameter with the give `name` to the given `binding`.
    pub fn try_bind(&self, name: &str, binding: Arc<Instance>) -> Result<(), BindingError> {
        if let Some(param) = self.inner.get(name) {
            param.try_bind(binding)
        } else {
            Err(BindingError::KeyMissing)
        }
    }

    ///Insert a parameter into the mapping, it should be unbound.
    pub(crate) fn insert_unbound(&mut self, name: &'static str, param: ParamAccess) {
        assert!(!self.inner.contains_key(name));
        self.inner.insert(name, param);
    }
}

impl From<HashMap<&'static str, ParamAccess>> for ParamHashMap {
    fn from(params: HashMap<&'static str, ParamAccess>) -> Self {
        Self { inner: params }
    }
}

impl ParamAccess {
    ///Create a new unbound `Get`.
    pub fn new_get(get: ParamGet) -> Self {
        Self::Get {
            get,
            binding: Default::default(),
        }
    }

    ///Create a new unbound `Set`.
    pub fn new_set(set: ParamSet) -> Self {
        Self::Set {
            set,
            binding: Default::default(),
        }
    }
}

impl Param {
    /// Create a new param instance.
    pub fn new<P, D, S>(type_name: &'static str, data: D, params: P, shadow: Option<S>) -> Self
    where
        P: Into<ParamHashMap>,
        D: Into<ParamDataAccess>,
        S: Into<ParamDataAccess>,
    {
        let id = uuid::Uuid::new_v4();
        Self::new_with_id(type_name, data, params, shadow, &id)
    }

    /// Create a new param instance, with the given id.
    pub fn new_with_id<P, D, S>(
        type_name: &'static str,
        data: D,
        params: P,
        shadow: Option<S>,
        id: &uuid::Uuid,
    ) -> Self
    where
        P: Into<ParamHashMap>,
        D: Into<ParamDataAccess>,
        S: Into<ParamDataAccess>,
    {
        Self {
            data: data.into(),
            shadow: shadow.map(|s| s.into()),
            params: params.into(),
            uuid: id.clone(),
            type_name,
        }
    }

    /// Get the unique identifier for this param instance.
    pub fn uuid(&self) -> uuid::Uuid {
        self.uuid
    }

    /// Get the type name for this param instance, for example `&"cast"` or `&"const"`.
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// Get the data for this param, to use in scheduler thread.
    pub fn data(&self) -> &ParamDataAccess {
        &self.data
    }

    /// Get the data type name for this param instance, for example `&"usize"` or `&"Float"`.
    pub fn data_type_name(&self) -> &'static str {
        self.data.data_type_name()
    }

    /// Get the shadow for this param, if there is one.
    pub fn shadow(&self) -> &Option<ParamDataAccess> {
        &self.shadow
    }
}

//pull in the codegen
include!(concat!(env!("OUT_DIR"), "/param.rs"));
