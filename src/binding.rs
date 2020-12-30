//! Data binding.

use sched::{
    binding::{
        bpm::ClockData,
        last::{BindingLastGet, BindingLastGetSet, BindingLastSet},
        ParamBinding, ParamBindingGet, ParamBindingSet,
    },
    tick::{TickResched, TickSched},
    Float,
};

use crate::param::{ParamHashMap, ParamMapGet};
use std::sync::Arc;

/// Bindings with their access.

/// An instance of a typed datum or operation.
pub struct Instance {
    binding: Access,
    params: ParamHashMap,
    uuid: uuid::Uuid,
    type_name: &'static str,
}

impl Instance {
    /// Create a new binding instance.
    pub fn new<P, A>(type_name: &'static str, binding: A, params: P) -> Self
    where
        P: Into<ParamHashMap>,
        A: Into<Access>,
    {
        Self {
            binding: binding.into(),
            params: params.into(),
            uuid: uuid::Uuid::new_v4(),
            type_name,
        }
    }

    /// Get the binding for this instance.
    pub fn binding(&self) -> &Access {
        &self.binding
    }

    /// Get the unique identifier for this binding instance.
    pub fn uuid(&self) -> uuid::Uuid {
        self.uuid
    }

    /// Get the type name for this binding instance, for example `&"cast"` or `&"const"`.
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// Get the data type name for this binding instance, for example `&"usize"` or `&"Float"`.
    pub fn data_type_name(&self) -> &'static str {
        self.binding.data_type_name()
    }

    /// Get the access name for this binding instance: `"get", "set" or "getset"`
    pub fn access_name(&self) -> &'static str {
        self.binding.access_name()
    }
}

impl ParamMapGet for Instance {
    fn params(&self) -> &ParamHashMap {
        &self.params
    }
}

impl From<::std::sync::atomic::AtomicBool> for Access {
    fn from(binding: ::std::sync::atomic::AtomicBool) -> Self {
        Self::new_bool_get_set_init(binding)
    }
}

impl From<::std::sync::atomic::AtomicU8> for Access {
    fn from(binding: ::std::sync::atomic::AtomicU8) -> Self {
        Self::new_u8_get_set_init(binding)
    }
}

impl From<::std::sync::atomic::AtomicUsize> for Access {
    fn from(binding: ::std::sync::atomic::AtomicUsize) -> Self {
        Self::new_usize_get_set_init(binding)
    }
}

impl From<::std::sync::atomic::AtomicIsize> for Access {
    fn from(binding: ::std::sync::atomic::AtomicIsize) -> Self {
        Self::new_isize_get_set_init(binding)
    }
}

impl From<Float> for Access {
    fn from(data: Float) -> Self {
        Self::new_float_get_set_init(sched::binding::spinlock::SpinlockParamBinding::new(data))
    }
}

impl From<TickSched> for Access {
    fn from(data: TickSched) -> Self {
        Self::new_tick_sched_get_set_init(sched::binding::spinlock::SpinlockParamBinding::new(data))
    }
}

impl From<TickResched> for Access {
    fn from(data: TickResched) -> Self {
        Self::new_tick_resched_get_set_init(sched::binding::spinlock::SpinlockParamBinding::new(
            data,
        ))
    }
}

impl From<ClockData> for Access {
    fn from(data: ClockData) -> Self {
        Self::new_clock_data_get_set_init(sched::binding::spinlock::SpinlockParamBinding::new(data))
    }
}

//pull in the codegen
include!(concat!(env!("OUT_DIR"), "/binding.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use crate::param::*;
    use sched::binding::{ParamBinding, ParamBindingGet};
    use std::collections::HashMap;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn can_create() {
        let g = Arc::new(Instance::new(
            &"value",
            Arc::new(AtomicUsize::new(0)) as Arc<dyn ParamBindingGet<usize>>,
            HashMap::new(),
        ));
        assert_eq!("get", g.access_name());
        assert_eq!("value", g.type_name());
        assert_eq!("usize", g.data_type_name());

        let a = Arc::new(AtomicUsize::new(0));
        let s = Arc::new(Instance::new(
            &"value",
            a.clone() as Arc<dyn ParamBinding<usize>>,
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

        let max = Instance::new(&"value", max as Arc<dyn ParamBindingGet<usize>>, map);
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

        let get_max = max.as_usize_get();
        assert!(max.as_bool_get().is_none());
        assert!(get_max.is_some());

        let get_max = get_max.unwrap();
        assert_eq!(0, get_max.get());

        let left = Arc::new(AtomicUsize::new(1));
        let left = Arc::new(Instance::new(
            &"value",
            left.clone() as Arc<dyn ParamBinding<usize>>,
            HashMap::new(),
        ));
        assert_eq!(None, max.params().uuid(&"left"));
        assert!(max.params().try_bind(&"left", left.clone()).is_ok());
        assert_eq!(Some(left.uuid()), max.params().uuid(&"left"));
        assert_eq!(1, get_max.get());

        let right = Arc::new(AtomicUsize::new(2));
        let right = Arc::new(Instance::new(
            &"value",
            right.clone() as Arc<dyn ParamBindingGet<usize>>,
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
