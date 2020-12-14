//! Graph nodes

use sched::{
    binding::swap::BindingSwapSet,
    graph::{children::vec::Children, GraphNodeExec},
};

use crate::param::ParamHashMap;

use std::sync::Arc;

pub enum GraphItem {
    Node {
        exec: Arc<dyn GraphNodeExec>,
        params: ParamHashMap,
        children: Children<BindingSwapSet<usize>>,
    },
    Leaf {
        exec: Arc<dyn GraphNodeExec>,
        params: ParamHashMap,
    },
}

impl GraphItem {
    ///Get a reference to the parameters for this item.
    pub fn params(&self) -> &ParamHashMap {
        match self {
            Self::Node { params, .. } => params,
            Self::Leaf { params, .. } => params,
        }
    }

    ///Get a mut reference to the parameters for this item.
    pub fn params_mut(&mut self) -> &ParamHashMap {
        match self {
            Self::Node { params, .. } => params,
            Self::Leaf { params, .. } => params,
        }
    }
}
