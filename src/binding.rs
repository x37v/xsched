//! Data binding.

use sched::{
    binding::{bpm::ClockData, ParamBindingGet, ParamBindingSet},
    tick::{TickResched, TickSched},
    Float,
};

use crate::param::ParamHashMap;
use std::sync::Arc;

//pull in the codegen
include!(concat!(env!("OUT_DIR"), "/binding.rs"));

/// Bindings with their access.

/// An instance of a typed datum or operation.
pub struct Instance {
    binding: Access,
    params: ParamHashMap,
    uuid: uuid::Uuid,
    type_name: &'static str,
}

impl Instance {
    /// Create a new binding
    pub fn new<P: Into<ParamHashMap>>(type_name: &'static str, binding: Access, params: P) -> Self {
        Self {
            binding,
            params: params.into(),
            uuid: uuid::Uuid::new_v4(),
            type_name,
        }
    }

    pub fn uuid(&self) -> uuid::Uuid {
        self.uuid
    }

    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    ///Get a `&str` representing the type of access: `"get", "set" or "getset"`
    pub fn access_name(&self) -> &'static str {
        //XXX
        &"TODO"
        //match &self.binding {
        //Access::Get(_) => "get",
        //Access::Set(_) => "set",
        //Access::GetSet(_) => "getset",
        //}
    }

    ///Get a reference to the parameters for this binding.
    pub fn params(&self) -> &ParamHashMap {
        &self.params
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::param::*;
    use sched::binding::ParamBindingGet;
    use std::collections::HashMap;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn can_create() {
        let g = Arc::new(Instance::new(
            &"value",
            Access::USizeGet(Arc::new(
                Arc::new(AtomicUsize::new(0)) as Arc<dyn ParamBindingGet<usize>>
            )),
            HashMap::new(),
        ));
        assert_eq!("get", g.access_name());
        assert_eq!("value", g.type_name());
        assert_eq!("usize", g.data_type_name());

        let a = Arc::new(AtomicUsize::new(0));
        let s = Arc::new(Instance::new(
            &"value",
            Access::USizeGetSet {
                get: Arc::new(a.clone() as Arc<dyn ParamBindingGet<usize>>),
                set: Arc::new(a.clone() as Arc<dyn ParamBindingSet<usize>>),
            },
            HashMap::new(),
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

        let max = Instance::new(
            &"value",
            Access::USizeGet(Arc::new(max as Arc<dyn ParamBindingGet<usize>>)),
            map,
        );
        assert_eq!(None, s.params().uuid(&"left"));
        assert_eq!(None, s.params().uuid(&"right"));
        assert_eq!(None, s.params().uuid(&"bill"));

        assert_eq!(Some("get"), max.params().access_name("left"));
        assert_eq!(Some("get"), max.params().access_name("right"));
        assert_eq!(None, max.params().access_name("bill"));

        assert_eq!(Some("usize"), max.params().data_type_name("left"));
        assert_eq!(Some("usize"), max.params().data_type_name("right"));

        assert_eq!(None, max.params().type_name("bill"));
        assert_eq!(None, max.params().type_name("bill"));

        let keys: Vec<&'static str> = max.params().keys().into_iter().map(|k| k.clone()).collect();

        assert_eq!(2, keys.len());
        assert!(keys.contains(&"left"));
        assert!(keys.contains(&"right"));

        assert_eq!("get", max.access_name());
        assert_eq!("usize", max.data_type_name());

        let get_max = max.as_usize_get();
        assert!(max.as_bool_get().is_none());
        assert!(get_max.is_some());

        let get_max = get_max.unwrap();
        assert_eq!(0, get_max.get());

        let left = Arc::new(AtomicUsize::new(1));
        let left = Arc::new(Instance::new(
            &"value",
            Access::USizeGetSet {
                get: Arc::new(left.clone() as Arc<dyn ParamBindingGet<usize>>),
                set: Arc::new(left.clone() as Arc<dyn ParamBindingSet<usize>>),
            },
            HashMap::new(),
        ));
        assert_eq!(None, max.params().uuid(&"left"));
        assert!(max.params().try_bind(&"left", left.clone()).is_ok());
        assert_eq!(Some(left.uuid()), max.params().uuid(&"left"));
        assert_eq!(1, get_max.get());

        let right = Arc::new(AtomicUsize::new(2));
        let right = Arc::new(Instance::new(
            &"value",
            Access::USizeGetSet {
                get: Arc::new(right.clone() as Arc<dyn ParamBindingGet<usize>>),
                set: Arc::new(right.clone() as Arc<dyn ParamBindingSet<usize>>),
            },
            HashMap::new(),
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

        let sleft = left.as_usize_set();
        assert!(sleft.is_some());
        sleft.unwrap().set(2084);
        assert_eq!(2084, get_max.get());
    }
}
