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

/// Strong, "owned", or weak, "unowned" bindings.
pub enum Owner<T: ?Sized> {
    Owned(Arc<T>),
    Unowned(Weak<T>),
}

pub enum Get {
    Bool(Owner<dyn ParamBindingGet<bool>>),
    U8(Owner<dyn ParamBindingGet<u8>>),
    USize(Owner<dyn ParamBindingGet<usize>>),
    ISize(Owner<dyn ParamBindingGet<isize>>),
    Float(Owner<dyn ParamBindingGet<Float>>),
    ClockData(Owner<dyn ParamBindingGet<ClockData>>),
}

pub enum Set {
    Bool(Owner<dyn ParamBindingSet<bool>>),
    U8(Owner<dyn ParamBindingSet<u8>>),
    USize(Owner<dyn ParamBindingSet<usize>>),
    ISize(Owner<dyn ParamBindingSet<isize>>),
    Float(Owner<dyn ParamBindingSet<Float>>),
    ClockData(Owner<dyn ParamBindingSet<ClockData>>),
}

pub enum ParamGet {
    Bool(Arc<BindingSwapGet<bool>>),
    U8(Arc<BindingSwapGet<u8>>),
    USize(Arc<BindingSwapGet<usize>>),
    ISize(Arc<BindingSwapGet<isize>>),
    Float(Arc<BindingSwapGet<Float>>),
    ClockData(Arc<BindingSwapGet<ClockData>>),
}

pub enum ParamSet {
    Bool(Arc<BindingSwapSet<bool>>),
    U8(Arc<BindingSwapSet<u8>>),
    USize(Arc<BindingSwapSet<usize>>),
    ISize(Arc<BindingSwapSet<isize>>),
    Float(Arc<BindingSwapSet<Float>>),
    ClockData(Arc<BindingSwapSet<ClockData>>),
}

pub enum Access {
    Get(Get),
    Set(Set),
    GetSet(Get, Set),
}

pub enum ParamAccess {
    Get(ParamGet),
    Set(ParamSet),
    GetSet(ParamGet, ParamSet),
}

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
    pub fn param_bind(&mut self, name: &str, binding: &Self) -> Result<(), BindingError> {
        if let Some(param) = self.params.get_mut(name) {
            match param {
                ParamAccess::Get(g) => {
                    //XXX
                }
                ParamAccess::Set(s) => {
                    //XXX
                }
                ParamAccess::GetSet(g, s) => {
                    //XXX
                }
            }
            Ok(())
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

    /*
    pub fn as_bool_get(&self) -> Option<Arc<dyn ParamBindingGet<bool>>> {
        match self.as_get() {
            Some(Get::Bool(o)) => o.as_arc(),
            _ => None,
        }
    }
    pub fn as_bool_set(&self) -> Option<Arc<dyn ParamBindingSet<bool>>> {
        match self.as_set() {
            Some(Set::Bool(o)) => o.as_arc(),
            _ => None,
        }
    }
    */
    impl_get_set!(bool, Bool, bool);
    impl_get_set!(u8, U8, bool);
    impl_get_set!(usize, USize, usize);
    impl_get_set!(isize, ISize, usize);
    impl_get_set!(Float, Float, float);
    impl_get_set!(ClockData, ClockData, clock_data);
}

impl ParamGet {
    pub fn bind(&mut self, binding: &ParamAccess) -> Result<(), BindingError> {
        match self {
            Self::Bool(b) => {
                //XXX
            }
            Self::U8(b) => {
                //XXX
            }
            Self::USize(b) => {
                //XXX
            }
            Self::ISize(b) => {
                //XXX
            }
            Self::Float(b) => {
                //XXX
            }
            Self::ClockData(b) => {
                //XXX
            }
        };
        Ok(())
    }

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
    pub fn bind(&mut self, binding: &ParamAccess) -> Result<(), BindingError> {
        match self {
            Self::Bool(b) => {
                //XXX
            }
            Self::U8(b) => {
                //XXX
            }
            Self::USize(b) => {
                //XXX
            }
            Self::ISize(b) => {
                //XXX
            }
            Self::Float(b) => {
                //XXX
            }
            Self::ClockData(b) => {
                //XXX
            }
        };
        Ok(())
    }

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
