use sched::{
    binding::{
        bpm::ClockData,
        swap::{BindingSwapGet, BindingSwapSet},
        ParamBindingGet, ParamBindingSet,
    },
    Float,
};
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};

include!(concat!(env!("OUT_DIR"), "/binding.rs"));

/// Strong, "owned", or weak, "unowned" bindings.
pub enum Owner<T: ?Sized> {
    Owned(Arc<T>),
    Unowned(Weak<T>),
}

/// Get bindings.
pub enum Get {
    Bool(Owner<dyn ParamBindingGet<bool>>),
    U8(Owner<dyn ParamBindingGet<u8>>),
    USize(Owner<dyn ParamBindingGet<usize>>),
    ISize(Owner<dyn ParamBindingGet<isize>>),
    Float(Owner<dyn ParamBindingGet<Float>>),
    ClockData(Owner<dyn ParamBindingGet<ClockData>>),
}

/// Set bindings.
pub enum Set {
    Bool(Owner<dyn ParamBindingSet<bool>>),
    U8(Owner<dyn ParamBindingSet<u8>>),
    USize(Owner<dyn ParamBindingSet<usize>>),
    ISize(Owner<dyn ParamBindingSet<isize>>),
    Float(Owner<dyn ParamBindingSet<Float>>),
    ClockData(Owner<dyn ParamBindingSet<ClockData>>),
}

/// Parameters that you can get values from.
pub enum ParamGet {
    Bool(Arc<BindingSwapGet<bool>>),
    U8(Arc<BindingSwapGet<u8>>),
    USize(Arc<BindingSwapGet<usize>>),
    ISize(Arc<BindingSwapGet<isize>>),
    Float(Arc<BindingSwapGet<Float>>),
    ClockData(Arc<BindingSwapGet<ClockData>>),
}

/// Parameters that you can set to a value.
pub enum ParamSet {
    Bool(Arc<BindingSwapSet<bool>>),
    U8(Arc<BindingSwapSet<u8>>),
    USize(Arc<BindingSwapSet<usize>>),
    ISize(Arc<BindingSwapSet<isize>>),
    Float(Arc<BindingSwapSet<Float>>),
    ClockData(Arc<BindingSwapSet<ClockData>>),
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

pub struct Binding {
    binding: Access,
    params: HashMap<String, ParamAccess>,
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

macro_rules! impl_get_set {
    ($t:ty, $variant:ident, $name:ident) => {
        paste::item! {
            pub fn [<as_ $name _get>](&self) -> Option<Arc<dyn ParamBindingGet<$t>>> {
                match self.as_get() {
                    Some(Get::$variant(o)) => o.as_arc(),
                    _ => None,
                }
            }
            pub fn [<as_ $name _set>](&self) -> Option<Arc<dyn ParamBindingSet<$t>>> {
                match self.as_set() {
                    Some(Set::$variant(o)) => o.as_arc(),
                    _ => None,
                }
            }
        }
    };
}

impl Binding {
    /// Create a new binding
    pub fn new(binding: Access, params: HashMap<String, ParamAccess>) -> Self {
        Self { binding, params }
    }

    ///Get a `&str` representing the type of access: `"get", "set" or "getset"`
    pub fn access_name(&self) -> &str {
        match &self.binding {
            Access::Get(_) => "get",
            Access::Set(_) => "set",
            Access::GetSet(_, _) => "getset",
        }
    }

    ///Get the type name for the contained `Get` value, if there is one.
    pub fn get_type_name(&self) -> Option<&str> {
        if let Some(g) = self.as_get() {
            Some(match g {
                Get::Bool(_) => "bool",
                Get::U8(_) => "u8",
                Get::USize(_) => "usize",
                Get::ISize(_) => "isize",
                Get::Float(_) => "float",
                Get::ClockData(_) => "clock_data",
            })
        } else {
            None
        }
    }

    ///Get the type name for the contained `Set` value, if there is one.
    pub fn set_type_name(&self) -> Option<&str> {
        if let Some(s) = self.as_set() {
            Some(match s {
                Set::Bool(_) => "bool",
                Set::U8(_) => "u8",
                Set::USize(_) => "usize",
                Set::ISize(_) => "isize",
                Set::Float(_) => "float",
                Set::ClockData(_) => "clock_data",
            })
        } else {
            None
        }
    }

    pub fn param_unbind(&mut self, name: &str) {
        if let Some(param) = self.params.get_mut(name) {
            match param {
                ParamAccess::Get(g) => g.unbind(),
                ParamAccess::Set(s) => s.unbind(),
                ParamAccess::GetSet(g, s) => {
                    g.unbind();
                    s.unbind();
                }
            }
        }
    }

    ///Bind the parameter with the give `name` to the given `binding`.
    ///
    pub fn param_bind(&mut self, name: &str, binding: &Self) -> Result<(), BindingError> {
        if let Some(param) = self.params.get_mut(name) {
            param.bind(binding)
        } else {
            Err(BindingError::KeyMissing)
        }
    }

    fn as_get(&self) -> Option<&Get> {
        match &self.binding {
            Access::Get(m) => Some(m),
            Access::Set(_) => None,
            Access::GetSet(m, _) => Some(m),
        }
    }
    fn as_set(&self) -> Option<&Set> {
        match &self.binding {
            Access::Get(_) => None,
            Access::Set(m) => Some(m),
            Access::GetSet(_, m) => Some(m),
        }
    }

    //impl getter and setter for the given type, with the variant and the function name ident
    impl_get_set!(bool, Bool, bool);
    impl_get_set!(u8, U8, u8);
    impl_get_set!(usize, USize, usize);
    impl_get_set!(isize, ISize, isize);
    impl_get_set!(Float, Float, float);
    impl_get_set!(ClockData, ClockData, clock_data);
}

impl ParamGet {
    //TODO transform and return output?
    pub fn unbind(&mut self) {
        match self {
            Self::Bool(b) => {
                b.unbind();
            }
            Self::U8(b) => {
                b.unbind();
            }
            Self::USize(b) => {
                b.unbind();
            }
            Self::ISize(b) => {
                b.unbind();
            }
            Self::Float(b) => {
                b.unbind();
            }
            Self::ClockData(b) => {
                b.unbind();
            }
        }
    }
}

impl ParamSet {
    //TODO transform and return output?
    pub fn unbind(&mut self) {
        match self {
            Self::Bool(b) => {
                b.unbind();
            }
            Self::U8(b) => {
                b.unbind();
            }
            Self::USize(b) => {
                b.unbind();
            }
            Self::ISize(b) => {
                b.unbind();
            }
            Self::Float(b) => {
                b.unbind();
            }
            Self::ClockData(b) => {
                b.unbind();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

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

        assert_eq!(Err(BindingError::KeyMissing), s.param_bind(&"soda", &g));
        assert_eq!(Err(BindingError::KeyMissing), g.param_bind(&"foo", &s));
    }
}
