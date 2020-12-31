//! Graph nodes

use crate::{
    children::{ALockChildren, SwapChildren, SwapChildrenContainer},
    param::{ParamHashMap, ParamMapGet},
};
use sched::{
    graph::{node_wrapper::GraphNodeWrapper, GraphLeafExec, GraphNodeContainer, GraphNodeExec},
    mutex::Mutex,
};

use std::sync::Arc;

/// An enum for holding and describing graph nodes.
pub enum GraphItem {
    /// Root is a root of a graph, it cannot be added as a child.
    Root {
        type_name: &'static str,
        uuid: uuid::Uuid,
        inner: GraphNodeContainer,
        params: ParamHashMap,
        children: Arc<Mutex<SwapChildren>>,
    },
    ///Node can have children.
    Node {
        type_name: &'static str,
        uuid: uuid::Uuid,
        inner: GraphNodeContainer,
        params: ParamHashMap,
        children: Arc<Mutex<SwapChildren>>,
    },
    ///Leaf is a terminal node, cannot have children.
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
    /// * `id` - a optional uuid to assign to this leaf, a value will be generated if `None`.
    pub fn new_leaf<P: Into<ParamHashMap>, N: GraphLeafExec + 'static>(
        type_name: &'static str,
        exec: N,
        params: P,
        id: Option<uuid::Uuid>,
    ) -> Self {
        Self::Leaf {
            type_name,
            uuid: id.unwrap_or_else(|| uuid::Uuid::new_v4()),
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
    /// * `id` - a optional uuid to assign to this node, a value will be generated if `None`.
    ///
    /// # Remarks
    ///
    /// * a `child_exec_index` Set(USize) item will be added to `params`, it must not collide.
    pub fn new_node<P: Into<ParamHashMap>, N: GraphNodeExec + 'static>(
        type_name: &'static str,
        exec: N,
        params: P,
        id: Option<uuid::Uuid>,
    ) -> Self {
        let children: Arc<Mutex<SwapChildren>> = Default::default();
        //add child_exec_index to the parameters
        let mut params = params.into();
        params.insert_unbound(
            &"child_exec_index",
            crate::param::ParamAccess::Set {
                set: crate::param::ParamSet::USize(children.lock().index_binding()),
                binding: Default::default(),
            },
        );
        Self::Node {
            type_name,
            uuid: id.unwrap_or_else(|| uuid::Uuid::new_v4()),
            inner: GraphNodeWrapper::new(exec, SwapChildrenContainer::new(children.clone())).into(),
            params,
            children,
        }
    }

    /// Create a new root with the given id.
    ///
    /// # Arguments
    ///
    /// * `type_name` - the name of the graph `exec` type, used to describe this node.
    /// * `exec` - the executor for this node.
    /// * `params` - a map of the parameters for this node.
    /// * `id` - a optional uuid to assign to this root, a value will be generated if `None`.
    ///
    /// # Remarks
    ///
    /// * a `child_exec_index` Set(USize) item will be added to `params`, it must not collide.
    pub fn new_root<P: Into<ParamHashMap>, N: GraphNodeExec + 'static>(
        type_name: &'static str,
        exec: N,
        params: P,
        id: Option<uuid::Uuid>,
    ) -> Self {
        let children: Arc<Mutex<SwapChildren>> = Default::default();
        //add child_exec_index to the parameters
        let mut params = params.into();
        params.insert_unbound(
            &"child_exec_index",
            crate::param::ParamAccess::Set {
                set: crate::param::ParamSet::USize(children.lock().index_binding()),
                binding: Default::default(),
            },
        );
        Self::Root {
            type_name,
            uuid: id.unwrap_or_else(|| uuid::Uuid::new_v4()),
            inner: GraphNodeWrapper::new(exec, SwapChildrenContainer::new(children.clone())).into(),
            params,
            children,
        }
    }

    pub fn get_node(&self) -> GraphNodeContainer {
        match self {
            Self::Root { inner, .. } => inner.clone(),
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
            Self::Root { children, .. } => Ok(children.lock().swap(new_children)),
            Self::Node { children, .. } => Ok(children.lock().swap(new_children)),
            Self::Leaf { .. } => Err(new_children),
        }
    }

    ///Get the type name for this item.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Root { type_name, .. } => type_name,
            Self::Node { type_name, .. } => type_name,
            Self::Leaf { type_name, .. } => type_name,
        }
    }
}

impl ParamMapGet for GraphItem {
    ///Get a reference to the parameters for this item.
    fn params(&self) -> &ParamHashMap {
        match self {
            Self::Root { params, .. } => params,
            Self::Node { params, .. } => params,
            Self::Leaf { params, .. } => params,
        }
    }
}
