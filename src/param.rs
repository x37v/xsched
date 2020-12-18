//! Parameters
use crate::binding::Binding;
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

//pull in the codegen
include!(concat!(env!("OUT_DIR"), "/param.rs"));

/// Parameters with their access.
pub enum ParamAccess {
    Get {
        get: ParamGet,
        binding: Mutex<Option<Arc<Binding>>>,
    },
    Set {
        set: ParamSet,
        binding: Mutex<Option<Arc<Binding>>>,
    },
    GetSet {
        get: ParamGet,
        set: ParamSet,
        binding: Mutex<Option<Arc<Binding>>>,
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

impl ParamHashMap {
    ///Get the param keys.
    pub fn keys(&self) -> Keys<'_, &'static str, ParamAccess> {
        self.inner.keys()
    }

    /// Get the access name for the parameter with `name`.
    pub fn access_name(&self, name: &str) -> Option<&str> {
        if let Some(param) = self.inner.get(name) {
            Some(param.access_name())
        } else {
            None
        }
    }

    /// Get the 'get' type name for the parameter with `name`, if there is a param and if it has a
    /// get.
    pub fn get_type_name(&self, name: &str) -> Option<&str> {
        if let Some(param) = self.inner.get(name) {
            param.get_type_name()
        } else {
            None
        }
    }

    /// Get the 'set' type name for the parameter with `name`, if there is a param and if it has a
    /// set.
    pub fn set_type_name(&self, name: &str) -> Option<&str> {
        if let Some(param) = self.inner.get(name) {
            param.set_type_name()
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

    pub fn unbind(&self, name: &str) -> Option<Arc<Binding>> {
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
                ParamAccess::GetSet {
                    get: g,
                    set: s,
                    binding: b,
                } => {
                    let mut l = b.lock();
                    g.unbind();
                    s.unbind();
                    l.take()
                }
            }
        } else {
            None
        }
    }

    ///Bind the parameter with the give `name` to the given `binding`.
    pub fn try_bind(&self, name: &str, binding: Arc<Binding>) -> Result<(), BindingError> {
        if let Some(param) = self.inner.get(name) {
            param.try_bind(binding)
        } else {
            Err(BindingError::KeyMissing)
        }
    }

    ///Insert a parameter into the mapping, it should be unbound
    pub(crate) fn insert_unbound(&mut self, name: &'static str, param: ParamAccess) {
        //XXX assert unbound and no collision
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

    ///Create a new unbound `GetSet`.
    pub fn new_getset(get: ParamGet, set: ParamSet) -> Self {
        Self::GetSet {
            get,
            set,
            binding: Default::default(),
        }
    }
}
