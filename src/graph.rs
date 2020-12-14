//! Graph nodes

use crate::{
    children::{ALockChildren, SwapChildren, SwapChildrenContainer},
    param::ParamHashMap,
};
use sched::{
    binding::swap::BindingSwapSet,
    graph::{node_wrapper::GraphNodeWrapper, GraphLeafExec, GraphNodeContainer, GraphNodeExec},
    mutex::Mutex,
};

use std::sync::Arc;

/// An enum for holding and describing graph nodes.
pub enum GraphItem {
    Node {
        type_name: &'static str,
        uuid: uuid::Uuid,
        inner: GraphNodeContainer,
        params: ParamHashMap,
        children: Arc<Mutex<SwapChildren>>,
    },
    Leaf {
        type_name: &'static str,
        uuid: uuid::Uuid,
        inner: GraphNodeContainer,
        params: ParamHashMap,
    },
}

impl GraphItem {
    /// Create a new leaf.
    ///
    /// # Arguments
    ///
    /// * `type_name` - the name of the graph `exec` type, used to describe this leaf.
    /// * `exec` - the executor for this leaf.
    /// * `params` - a map of the parameters for this leaf.
    pub fn new_leaf<P: Into<ParamHashMap>, N: GraphLeafExec + 'static>(
        type_name: &'static str,
        exec: N,
        params: P,
    ) -> Self {
        Self::Leaf {
            type_name,
            uuid: uuid::Uuid::new_v4(),
            inner: GraphNodeWrapper::new(exec, sched::graph::children::empty::Children).into(),
            params: params.into(),
        }
    }

    /// Create a new node.
    ///
    /// # Arguments
    ///
    /// * `type_name` - the name of the graph `exec` type, used to describe this node.
    /// * `exec` - the executor for this node.
    /// * `params` - a map of the parameters for this node.
    pub fn new_node<P: Into<ParamHashMap>, N: GraphNodeExec + 'static>(
        type_name: &'static str,
        exec: N,
        params: P,
    ) -> Self {
        let children: Arc<Mutex<SwapChildren>> = Default::default();
        //add child_exec_index to the parameters
        let mut params = params.into();
        params.insert_unbound(
            &"child_exec_index",
            crate::param::ParamAccess::Set(crate::param::ParamSet::USize(
                children.lock().index_binding(),
            )),
        );
        Self::Node {
            type_name,
            uuid: uuid::Uuid::new_v4(),
            inner: GraphNodeWrapper::new(exec, SwapChildrenContainer::new(children.clone())).into(),
            params,
            children,
        }
    }

    pub fn get_node(&self) -> GraphNodeContainer {
        match self {
            Self::Node { inner, .. } => inner.clone(),
            Self::Leaf { inner, .. } => inner.clone(),
        }
    }

    /// Swap children
    pub fn swap_children(
        &mut self,
        new_children: ALockChildren,
    ) -> Result<ALockChildren, ALockChildren> {
        match self {
            Self::Node { children, .. } => Ok(children.lock().swap(new_children)),
            Self::Leaf { .. } => Err(new_children),
        }
    }

    ///Get the type name for this item.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Node { type_name, .. } => type_name,
            Self::Leaf { type_name, .. } => type_name,
        }
    }

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
