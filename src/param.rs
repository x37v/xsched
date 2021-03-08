//! Parameters: named typed bindings.

use sched::{
    binding::swap::{BindingSwapGet, BindingSwapSet},
    mutex::Mutex,
};
use std::{
    collections::{hash_map::Keys, HashMap},
    sync::Arc,
};

use sched::binding::{
    ParamBindingGet, ParamBindingKeyValueGet, ParamBindingKeyValueSet, ParamBindingSet,
};

pub mod factory;

pub type ParamBindingGetSet<T> =
    ::sched::binding::ParamBindingGetSet<T, Arc<dyn ::sched::binding::ParamBinding<T>>>;

pub type ParamBindingKeyValueGetSet<T> = ::sched::binding::ParamBindingKeyValueGetSet<
    T,
    Arc<dyn ::sched::binding::ParamBindingKeyValue<T>>,
>;

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

pub trait AsParamGet<T> {
    fn as_get(&self) -> Option<::std::sync::Arc<dyn ParamBindingGet<T>>>;
}

pub trait AsParamSet<T> {
    fn as_set(&self) -> Option<::std::sync::Arc<dyn ParamBindingSet<T>>>;
}

pub trait AsParamKeyValueGet<T> {
    fn as_key_value_get(&self) -> Option<::std::sync::Arc<dyn ParamBindingKeyValueGet<T>>>;
}

pub trait AsParamKeyValueSet<T> {
    fn as_key_value_set(&self) -> Option<::std::sync::Arc<dyn ParamBindingKeyValueSet<T>>>;
}

/// Parameters with their access.
pub enum ParamAccess {
    Get {
        get: ParamGet,
        binding: Mutex<Option<Arc<Param>>>,
    },
    Set {
        set: ParamSet,
        binding: Mutex<Option<Arc<Param>>>,
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

    pub fn unbind(&self, name: &str) -> Option<Arc<Param>> {
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
    pub fn try_bind(&self, name: &str, binding: Arc<Param>) -> Result<(), BindingError> {
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
    pub fn new<P, D>(
        type_name: &'static str,
        data: D,
        params: P,
        shadow: Option<ParamDataAccess>,
    ) -> Self
    where
        P: Into<ParamHashMap>,
        D: Into<ParamDataAccess>,
    {
        let id = uuid::Uuid::new_v4();
        Self::new_with_id(type_name, data, params, shadow, &id)
    }

    /// Create a new param instance, with the given id.
    pub fn new_with_id<P, D>(
        type_name: &'static str,
        data: D,
        params: P,
        shadow: Option<ParamDataAccess>,
        id: &uuid::Uuid,
    ) -> Self
    where
        P: Into<ParamHashMap>,
        D: Into<ParamDataAccess>,
    {
        Self {
            data: data.into(),
            shadow,
            params: params.into(),
            uuid: id.clone(),
            type_name,
        }
    }

    /// Get the unique identifier for this param instance.
    pub fn uuid(&self) -> uuid::Uuid {
        self.uuid
    }

    /// Get the data for this param, to use in scheduler thread.
    pub fn data(&self) -> &ParamDataAccess {
        &self.data
    }

    /// Get the type name for this param instance, for example `&"cast"` or `&"const"`.
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// Get the access name for this param instance: `"get", "set" or "getset"`
    pub fn access_name(&self) -> &'static str {
        self.data.access_name()
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

impl ParamMapGet for Param {
    fn params(&self) -> &ParamHashMap {
        &self.params
    }
}

//pull in the codegen
include!(concat!(env!("OUT_DIR"), "/param.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use ::sched::binding::ParamBindingGet;
    use std::collections::HashMap;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn can_create() {
        let g = Arc::new(Param::new(
            &"value",
            Arc::new(AtomicUsize::new(0)) as Arc<dyn ParamBindingGet<usize>>,
            HashMap::new(),
            None,
        ));
        assert_eq!("get", g.access_name());
        assert_eq!("value", g.type_name());
        assert_eq!("usize", g.data_type_name());

        let a = Arc::new(AtomicUsize::new(0));
        let s = Arc::new(Param::new(
            &"value",
            Arc::new(ParamBindingGetSet::new(a.clone())),
            HashMap::new(),
            None,
        ));
        assert_eq!("getset", s.access_name());
        assert_eq!("usize", s.data_type_name());

        assert_eq!(
            Err(BindingError::KeyMissing),
            s.params().try_bind(&"soda", g.clone())
        );
        assert_eq!(
            Err(BindingError::KeyMissing),
            g.params().try_bind(&"foo", s.clone())
        );
        assert_eq!(None, s.params().uuid(&"soda"));
        assert_eq!(None, s.params().uuid(&"foo"));

        let lswap = Arc::new(sched::binding::swap::BindingSwapGet::default());
        let rswap = Arc::new(sched::binding::swap::BindingSwapGet::default());
        let max = Arc::new(sched::binding::ops::GetBinaryOp::new(
            core::cmp::max,
            lswap.clone() as Arc<dyn ParamBindingGet<usize>>,
            rswap.clone() as Arc<dyn ParamBindingGet<usize>>,
        ));

        let mut map = HashMap::new();
        map.insert("left", ParamAccess::new_get(ParamGet::USize(lswap)));
        map.insert("right", ParamAccess::new_get(ParamGet::USize(rswap)));

        let max = Param::new(&"value", max as Arc<dyn ParamBindingGet<usize>>, map, None);
        assert_eq!(None, s.params().uuid(&"left"));
        assert_eq!(None, s.params().uuid(&"right"));
        assert_eq!(None, s.params().uuid(&"bill"));

        assert_eq!(Some("get"), max.params().access_name("left"));
        assert_eq!(Some("get"), max.params().access_name("right"));
        assert_eq!(None, max.params().access_name("bill"));

        assert_eq!(Some("usize"), max.params().data_type_name("left"));
        assert_eq!(Some("usize"), max.params().data_type_name("right"));

        assert_eq!(None, max.params().data_type_name("bill"));
        assert_eq!(None, max.params().data_type_name("bill"));

        let keys: Vec<&'static str> = max.params().keys().into_iter().map(|k| k.clone()).collect();

        assert_eq!(2, keys.len());
        assert!(keys.contains(&"left"));
        assert!(keys.contains(&"right"));

        assert_eq!("get", max.access_name());
        assert_eq!("usize", max.data_type_name());

        let get_max: Option<Arc<dyn ParamBindingGet<usize>>> = max.as_get();
        let get_bool: Option<Arc<dyn ParamBindingGet<bool>>> = max.as_get();
        assert!(get_bool.is_none());
        assert!(get_max.is_some());

        let get_max = get_max.unwrap();
        assert_eq!(0, get_max.get());

        let left = Arc::new(AtomicUsize::new(1));
        let left = Arc::new(Param::new(
            &"value",
            Arc::new(ParamBindingGetSet::new(left.clone())),
            HashMap::new(),
            None,
        ));
        assert_eq!(None, max.params().uuid(&"left"));
        assert!(max.params().try_bind(&"left", left.clone()).is_ok());
        assert_eq!(Some(left.uuid()), max.params().uuid(&"left"));
        assert_eq!(1, get_max.get());

        let right = Arc::new(AtomicUsize::new(2));
        let right = Arc::new(Param::new(
            &"value",
            right.clone() as Arc<dyn ParamBindingGet<usize>>,
            HashMap::new(),
            None,
        ));
        assert_eq!(None, max.params().uuid(&"right"));
        assert!(max.params().try_bind(&"right", right.clone()).is_ok());
        assert_eq!(Some(left.uuid()), max.params().uuid(&"left"));
        assert_eq!(Some(right.uuid()), max.params().uuid(&"right"));
        assert_eq!(2, get_max.get());
        max.params().unbind(&"right");
        assert_eq!(None, max.params().uuid(&"right"));
        assert_eq!(1, get_max.get());

        assert!(max.params().try_bind(&"right", left.clone()).is_ok());
        assert!(max.params().try_bind(&"left", right.clone()).is_ok());
        assert_eq!(Some(left.uuid()), max.params().uuid(&"right"));
        assert_eq!(Some(right.uuid()), max.params().uuid(&"left"));
        assert_eq!(2, get_max.get());

        assert!(max.params().try_bind(&"left", left.clone()).is_ok());
        assert!(max.params().try_bind(&"right", right.clone()).is_ok());
        assert_eq!(Some(left.uuid()), max.params().uuid(&"left"));
        assert_eq!(Some(right.uuid()), max.params().uuid(&"right"));
        max.params().unbind(&"left");
        assert_eq!(Some(right.uuid()), max.params().uuid(&"right"));
        assert_eq!(None, max.params().uuid(&"left"));
        assert_eq!(2, get_max.get());

        max.params().unbind(&"right");
        assert_eq!(None, max.params().uuid(&"right"));
        assert_eq!(None, max.params().uuid(&"left"));
        assert_eq!(0, get_max.get());

        assert!(max.params().try_bind(&"left", left.clone()).is_ok());
        assert!(max.params().try_bind(&"right", right.clone()).is_ok());
        assert_eq!(Some(left.uuid()), max.params().uuid(&"left"));
        assert_eq!(Some(right.uuid()), max.params().uuid(&"right"));
        assert_eq!(2, get_max.get());

        let sleft = AsParamSet::<usize>::as_set(left.as_ref());
        assert!(sleft.is_some());
        sleft.unwrap().set(2084);
        assert_eq!(2084, get_max.get());
    }
}
