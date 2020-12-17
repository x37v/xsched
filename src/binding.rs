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
pub enum Access {
    Get(Get),
    Set(Set),
    GetSet(Get, Set),
}

pub struct Binding {
    binding: Access,
    params: ParamHashMap,
    uuid: uuid::Uuid,
}

impl Binding {
    /// Create a new binding
    pub fn new<P: Into<ParamHashMap>>(binding: Access, params: P) -> Self {
        Self {
            binding,
            params: params.into(),
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
        let mut g = Arc::new(Binding::new(
            Access::Get(Get::USize(Arc::new(
                Arc::new(AtomicUsize::new(0)) as Arc<dyn ParamBindingGet<usize>>
            ))),
            HashMap::new(),
        ));
        assert_eq!("get", g.access_name());
        assert_eq!(Some("usize"), g.get_type_name());
        assert_eq!(None, g.set_type_name());

        let a = Arc::new(AtomicUsize::new(0));
        let mut s = Arc::new(Binding::new(
            Access::GetSet(
                Get::USize(Arc::new(a.clone() as Arc<dyn ParamBindingGet<usize>>)),
                Set::USize(Arc::new(a.clone() as Arc<dyn ParamBindingSet<usize>>)),
            ),
            HashMap::new(),
        ));
        assert_eq!("getset", s.access_name());
        assert_eq!(Some("usize"), s.get_type_name());
        assert_eq!(Some("usize"), s.set_type_name());

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

        let mut max = Binding::new(
            Access::Get(Get::USize(Arc::new(max as Arc<dyn ParamBindingGet<usize>>))),
            map,
        );
        assert_eq!(None, s.params().uuid(&"left"));
        assert_eq!(None, s.params().uuid(&"right"));
        assert_eq!(None, s.params().uuid(&"bill"));

        assert_eq!(Some("get"), max.params().access_name("left"));
        assert_eq!(Some("get"), max.params().access_name("right"));
        assert_eq!(None, max.params().access_name("bill"));

        assert_eq!(Some("usize"), max.params().get_type_name("left"));
        assert_eq!(Some("usize"), max.params().get_type_name("right"));
        assert_eq!(None, max.params().set_type_name("left"));
        assert_eq!(None, max.params().set_type_name("right"));

        assert_eq!(None, max.params().get_type_name("bill"));
        assert_eq!(None, max.params().set_type_name("bill"));

        let keys: Vec<&'static str> = max.params().keys().into_iter().map(|k| k.clone()).collect();

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
        let left = Arc::new(Binding::new(
            Access::GetSet(
                Get::USize(Arc::new(left.clone() as Arc<dyn ParamBindingGet<usize>>)),
                Set::USize(Arc::new(left.clone() as Arc<dyn ParamBindingSet<usize>>)),
            ),
            HashMap::new(),
        ));
        assert_eq!(None, max.params().uuid(&"left"));
        assert!(max.params().try_bind(&"left", left.clone()).is_ok());
        assert_eq!(Some(left.uuid()), max.params().uuid(&"left"));
        assert_eq!(1, get_max.get());

        let right = Arc::new(AtomicUsize::new(2));
        let right = Arc::new(Binding::new(
            Access::GetSet(
                Get::USize(Arc::new(right.clone() as Arc<dyn ParamBindingGet<usize>>)),
                Set::USize(Arc::new(right.clone() as Arc<dyn ParamBindingSet<usize>>)),
            ),
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
