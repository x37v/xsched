use sched::{
    binding::{
        bpm::ClockData,
        swap::{BindingSwapGet, BindingSwapSet},
        ParamBindingGet, ParamBindingSet,
    },
    tick::{TickResched, TickSched},
    Float,
};
use std::{
    collections::{hash_map::Keys, HashMap},
    sync::{Arc, Weak},
};

include!(concat!(env!("OUT_DIR"), "/binding.rs"));

/// Strong, "owned", or weak, "unowned" bindings.
pub enum Owner<T: ?Sized> {
    Owned(Arc<T>),
    Unowned(Weak<T>),
}

/// Bindings with their access.
pub enum Access {
    Get(Get),
    Set(Set),
    GetSet(Get, Set),
}

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

pub struct Binding {
    binding: Access,
    params: HashMap<&'static str, ParamAccessWithUUID>,
    uuid: uuid::Uuid,
}

impl<T> Owner<T>
where
    T: ?Sized,
{
    /// Get an Arc if possible.
    pub fn as_arc(&self) -> Option<Arc<T>> {
        match self {
            Owner::Owned(a) => Some(a.clone()),
            Owner::Unowned(w) => w.upgrade(),
        }
    }
}

impl Binding {
    /// Create a new binding
    pub fn new(binding: Access, params: HashMap<&'static str, ParamAccess>) -> Self {
        Self {
            binding,
            params: params
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
            uuid: uuid::Uuid::new_v4(),
        }
    }

    pub fn uuid(&self) -> uuid::Uuid {
        self.uuid
    }

    ///Get a `&str` representing the type of access: `"get", "set" or "getset"`
    pub fn access_name(&self) -> &str {
        match &self.binding {
            Access::Get(_) => "get",
            Access::Set(_) => "set",
            Access::GetSet(_, _) => "getset",
        }
    }

    ///Get the param keys.
    pub fn param_keys(&self) -> Keys<'_, &'static str, ParamAccessWithUUID> {
        self.params.keys()
    }

    /// Get the access name for the parameter with `name`.
    pub fn param_access_name(&self, name: &str) -> Option<&str> {
        if let Some(param) = self.params.get(name) {
            Some(param.access_name())
        } else {
            None
        }
    }

    /// Get the 'get' type name for the parameter with `name`, if there is a param and if it has a
    /// get.
    pub fn param_get_type_name(&self, name: &str) -> Option<&str> {
        if let Some(param) = self.params.get(name) {
            param.get_type_name()
        } else {
            None
        }
    }

    /// Get the 'set' type name for the parameter with `name`, if there is a param and if it has a
    /// set.
    pub fn param_set_type_name(&self, name: &str) -> Option<&str> {
        if let Some(param) = self.params.get(name) {
            param.set_type_name()
        } else {
            None
        }
    }

    /// Get the uuid of the bound parameter
    pub fn param_uuid(&self, name: &str) -> Option<uuid::Uuid> {
        if let Some(param) = self.params.get(name) {
            param.uuid()
        } else {
            None
        }
    }

    pub fn param_unbind(&mut self, name: &str) {
        if let Some(param) = self.params.get_mut(name) {
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
    ///
    pub fn param_try_bind(&mut self, name: &str, binding: &Self) -> Result<(), BindingError> {
        if let Some(param) = self.params.get_mut(name) {
            param.try_bind(binding)
        } else {
            Err(BindingError::KeyMissing)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    use sched::binding::ParamBindingGet;

    #[test]
    fn can_create() {
        let mut g = Binding::new(
            Access::Get(Get::USize(Owner::Owned(Arc::new(AtomicUsize::new(0)) as _))),
            HashMap::new(),
        );
        assert_eq!("get", g.access_name());
        assert_eq!(Some("usize"), g.get_type_name());
        assert_eq!(None, g.set_type_name());

        let a = Arc::new(AtomicUsize::new(0));
        let mut s = Binding::new(
            Access::GetSet(
                Get::USize(Owner::Owned(a.clone() as _)),
                Set::USize(Owner::Owned(a.clone() as _)),
            ),
            HashMap::new(),
        );
        assert_eq!("getset", s.access_name());
        assert_eq!(Some("usize"), s.get_type_name());
        assert_eq!(Some("usize"), s.set_type_name());

        assert_eq!(Err(BindingError::KeyMissing), s.param_try_bind(&"soda", &g));
        assert_eq!(Err(BindingError::KeyMissing), g.param_try_bind(&"foo", &s));
        assert_eq!(None, s.param_uuid(&"soda"));
        assert_eq!(None, s.param_uuid(&"foo"));

        let lswap = Arc::new(sched::binding::swap::BindingSwapGet::default());
        let rswap = Arc::new(sched::binding::swap::BindingSwapGet::default());
        let max = Arc::new(sched::binding::ops::GetBinaryOp::new(
            core::cmp::max,
            lswap.clone() as Arc<dyn ParamBindingGet<usize>>,
            rswap.clone() as Arc<dyn ParamBindingGet<usize>>,
        ));

        let mut map = HashMap::new();
        map.insert("left", ParamAccess::Get(ParamGet::USize(lswap)));
        map.insert("right", ParamAccess::Get(ParamGet::USize(rswap)));

        let mut max = Binding::new(
            Access::Get(Get::USize(Owner::Owned(
                max as Arc<dyn ParamBindingGet<usize>>,
            ))),
            map,
        );
        assert_eq!(None, s.param_uuid(&"left"));
        assert_eq!(None, s.param_uuid(&"right"));
        assert_eq!(None, s.param_uuid(&"bill"));

        assert_eq!(Some("get"), max.param_access_name("left"));
        assert_eq!(Some("get"), max.param_access_name("right"));
        assert_eq!(None, max.param_access_name("bill"));

        assert_eq!(Some("usize"), max.param_get_type_name("left"));
        assert_eq!(Some("usize"), max.param_get_type_name("right"));
        assert_eq!(None, max.param_set_type_name("left"));
        assert_eq!(None, max.param_set_type_name("right"));

        assert_eq!(None, max.param_get_type_name("bill"));
        assert_eq!(None, max.param_set_type_name("bill"));

        let keys: Vec<&'static str> = max.param_keys().into_iter().map(|k| k.clone()).collect();

        assert_eq!(2, keys.len());
        assert!(keys.contains(&"left"));
        assert!(keys.contains(&"right"));

        assert_eq!("get", max.access_name());
        assert_eq!(Some("usize"), max.get_type_name());
        assert_eq!(None, max.set_type_name());

        let get_max = max.as_usize_get();
        assert!(max.as_bool_get().is_none());
        assert!(get_max.is_some());

        let get_max = get_max.unwrap();
        assert_eq!(0, get_max.get());

        let left = Arc::new(AtomicUsize::new(1));
        let left = Binding::new(
            Access::GetSet(
                Get::USize(Owner::Owned(left.clone() as _)),
                Set::USize(Owner::Owned(left.clone() as _)),
            ),
            HashMap::new(),
        );
        assert_eq!(None, max.param_uuid(&"left"));
        assert!(max.param_try_bind(&"left", &left).is_ok());
        assert_eq!(Some(left.uuid()), max.param_uuid(&"left"));
        assert_eq!(1, get_max.get());

        let right = Arc::new(AtomicUsize::new(2));
        let right = Binding::new(
            Access::GetSet(
                Get::USize(Owner::Owned(right.clone() as _)),
                Set::USize(Owner::Owned(right.clone() as _)),
            ),
            HashMap::new(),
        );
        assert_eq!(None, max.param_uuid(&"right"));
        assert!(max.param_try_bind(&"right", &right).is_ok());
        assert_eq!(Some(left.uuid()), max.param_uuid(&"left"));
        assert_eq!(Some(right.uuid()), max.param_uuid(&"right"));
        assert_eq!(2, get_max.get());
        max.param_unbind(&"right");
        assert_eq!(None, max.param_uuid(&"right"));
        assert_eq!(1, get_max.get());

        assert!(max.param_try_bind(&"right", &left).is_ok());
        assert!(max.param_try_bind(&"left", &right).is_ok());
        assert_eq!(Some(left.uuid()), max.param_uuid(&"right"));
        assert_eq!(Some(right.uuid()), max.param_uuid(&"left"));
        assert_eq!(2, get_max.get());

        assert!(max.param_try_bind(&"left", &left).is_ok());
        assert!(max.param_try_bind(&"right", &right).is_ok());
        assert_eq!(Some(left.uuid()), max.param_uuid(&"left"));
        assert_eq!(Some(right.uuid()), max.param_uuid(&"right"));
        max.param_unbind(&"left");
        assert_eq!(Some(right.uuid()), max.param_uuid(&"right"));
        assert_eq!(None, max.param_uuid(&"left"));
        assert_eq!(2, get_max.get());

        max.param_unbind(&"right");
        assert_eq!(None, max.param_uuid(&"right"));
        assert_eq!(None, max.param_uuid(&"left"));
        assert_eq!(0, get_max.get());

        assert!(max.param_try_bind(&"left", &left).is_ok());
        assert!(max.param_try_bind(&"right", &right).is_ok());
        assert_eq!(Some(left.uuid()), max.param_uuid(&"left"));
        assert_eq!(Some(right.uuid()), max.param_uuid(&"right"));
        assert_eq!(2, get_max.get());

        let sleft = left.as_usize_set();
        assert!(sleft.is_some());
        sleft.unwrap().set(2084);
        assert_eq!(2084, get_max.get());
    }
}
