//! Parameters
use crate::binding::Binding;
use sched::{
    binding::{
        bpm::ClockData,
        swap::{BindingSwapGet, BindingSwapSet},
    },
    tick::{TickResched, TickSched},
    Float,
};
use std::collections::{hash_map::Keys, HashMap};

//pull in the codegen
include!(concat!(env!("OUT_DIR"), "/param.rs"));

/// Parameters with their access.
pub enum ParamAccess {
    Get(ParamGet),
    Set(ParamSet),
    GetSet(ParamGet, ParamSet),
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

/// A helper struct that simply associates a param with a uuid
pub struct ParamAccessWithUUID {
    pub access: ParamAccess,
    pub uuid: Option<uuid::Uuid>,
}

#[derive(Default)]
pub struct ParamHashMap {
    inner: HashMap<&'static str, ParamAccessWithUUID>,
}

impl ParamHashMap {
    ///Get the param keys.
    pub fn keys(&self) -> Keys<'_, &'static str, ParamAccessWithUUID> {
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

    pub fn unbind(&mut self, name: &str) {
        if let Some(param) = self.inner.get_mut(name) {
            match &mut param.access {
                ParamAccess::Get(g) => g.unbind(),
                ParamAccess::Set(s) => s.unbind(),
                ParamAccess::GetSet(g, s) => {
                    g.unbind();
                    s.unbind();
                }
            }
            param.uuid = None;
        }
    }

    ///Bind the parameter with the give `name` to the given `binding`.
    pub fn try_bind(&mut self, name: &str, binding: &Binding) -> Result<(), BindingError> {
        if let Some(param) = self.inner.get_mut(name) {
            param.try_bind(binding)
        } else {
            Err(BindingError::KeyMissing)
        }
    }

    ///Insert a parameter into the mapping, it should be unbound
    pub(crate) fn insert_unbound(&mut self, name: &'static str, param: ParamAccess) {
        //XXX assert unbound and no collision
        self.inner.insert(
            name,
            crate::param::ParamAccessWithUUID {
                access: param,
                uuid: None,
            },
        );
    }
}

impl From<HashMap<&'static str, ParamAccess>> for ParamHashMap {
    fn from(params: HashMap<&'static str, ParamAccess>) -> Self {
        Self {
            inner: params
                .into_iter()
                .map(|(k, v)| {
                    (
                        k,
                        ParamAccessWithUUID {
                            access: v,
                            uuid: None,
                        },
                    )
                })
                .collect(),
        }
    }
}
