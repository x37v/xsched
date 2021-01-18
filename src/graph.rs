//! Graph nodes

use crate::{
    graph::children::{SwapChildren, SwapChildrenContainer},
    param::{ParamHashMap, ParamMapGet},
};
use sched::{
    atomic::{Atomic, Ordering},
    binding::{swap::BindingSwapSet, ParamBindingGet},
    event::{
        gate::{ArcMutexEvent, GateEvent},
        EventContainer,
    },
    graph::{
        node_wrapper::GraphNodeWrapper, root_wrapper::GraphRootWrapper, GraphLeafExec,
        GraphNodeContainer, GraphNodeExec, GraphRootExec,
    },
    mutex::Mutex,
};

use std::sync::Arc;

pub mod children;
pub mod factory;

pub type ChildrenUUIDs = (Arc<crate::graph::children::Children>, Vec<uuid::Uuid>);

#[derive(Default)]
pub struct SwapChildrenWithUUIDs {
    children: Arc<SwapChildren>,
    uuids: Vec<uuid::Uuid>,
}

/// An enum for holding and describing graph nodes.
pub enum GraphItem {
    /// Root is the start of a graph, it cannot be added as a child.
    Root {
        type_name: &'static str,
        uuid: uuid::Uuid,
        inner: ArcMutexEvent,
        params: ParamHashMap,
        children: Mutex<SwapChildrenWithUUIDs>,
        active_gate: Mutex<Option<Arc<Atomic<bool>>>>,
    },
    ///Node can have children.
    Node {
        type_name: &'static str,
        uuid: uuid::Uuid,
        inner: GraphNodeContainer,
        params: ParamHashMap,
        children: Mutex<SwapChildrenWithUUIDs>,
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
    //helper to get children and create params
    fn children_params<P: Into<ParamHashMap>>(params: P) -> (SwapChildrenWithUUIDs, ParamHashMap) {
        let children: SwapChildrenWithUUIDs = Default::default();
        let mut params = params.into();
        //add child_exec_index to the parameters
        params.insert_unbound(
            &"child_exec_index",
            crate::param::ParamAccess::Set {
                set: crate::param::ParamSet::USize(children.index_binding()),
                binding: Default::default(),
            },
        );
        (children, params)
    }

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
        let (children, params) = Self::children_params(params);
        Self::Node {
            type_name,
            uuid: id.unwrap_or_else(|| uuid::Uuid::new_v4()),
            inner: GraphNodeWrapper::new(exec, SwapChildrenContainer::new(children.children()))
                .into(),
            params,
            children: Mutex::new(children),
        }
    }

    /// Create a new root with the given id.
    ///
    /// # Arguments
    ///
    /// * `type_name` - the name of the graph `exec` type, used to describe this root.
    /// * `exec` - the executor for this root.
    /// * `params` - a map of the parameters for this root.
    /// * `id` - a optional uuid to assign to this root, a value will be generated if `None`.
    ///
    /// # Remarks
    ///
    /// * a `child_exec_index` Set(USize) item will be added to `params`, it must not collide.
    pub fn new_root<P: Into<ParamHashMap>, N: GraphRootExec + 'static>(
        type_name: &'static str,
        exec: N,
        params: P,
        id: Option<uuid::Uuid>,
    ) -> Self {
        let (children, params) = Self::children_params(params);
        Self::Root {
            type_name,
            uuid: id.unwrap_or_else(|| uuid::Uuid::new_v4()),
            inner: Arc::new(Mutex::new(GraphRootWrapper::new(
                exec,
                SwapChildrenContainer::new(children.children()),
            ))),
            params,
            children: Mutex::new(children),
            active_gate: Mutex::new(None),
        }
    }

    /// Get an `EventContainer` for this node, if it is a Root node.
    ///
    /// # Remarks
    /// * If this root is already active, this will halt its processing.
    pub fn root_event(&self) -> Option<EventContainer> {
        match self {
            Self::Root {
                ref active_gate,
                inner,
                ..
            } => {
                let g: Arc<Atomic<bool>> = Arc::new(Atomic::new(true));
                let v = EventContainer::new(GateEvent::new(
                    g.clone() as Arc<dyn ParamBindingGet<bool>>,
                    inner.clone(),
                ));
                if let Some(g) = active_gate.lock().replace(g) {
                    g.store(false, Ordering::Release);
                }
                Some(v)
            }
            Self::Node { .. } | Self::Leaf { .. } => None,
        }
    }

    /// Is this a root, and is it active?
    pub fn root_active(&self) -> Option<bool> {
        match self {
            Self::Root {
                ref active_gate, ..
            } => Some(active_gate.lock().is_some()),
            Self::Node { .. } | Self::Leaf { .. } => None,
        }
    }

    /// Deactivate this node, if it is a root.
    pub fn root_deactivate(&self) {
        match self {
            Self::Root {
                ref active_gate, ..
            } => {
                if let Some(g) = active_gate.lock().take() {
                    g.store(false, Ordering::Release);
                }
            }
            Self::Node { .. } | Self::Leaf { .. } => (),
        }
    }

    pub fn get_node(&self) -> Option<GraphNodeContainer> {
        match self {
            Self::Root { .. } => None,
            Self::Node { inner, .. } => Some(inner.clone()),
            Self::Leaf { inner, .. } => Some(inner.clone()),
        }
    }

    /// Get the uuids of the children.
    pub fn children_uuids(&self) -> Option<Vec<uuid::Uuid>> {
        match self {
            Self::Root { ref children, .. } | Self::Node { ref children, .. } => {
                Some(children.lock().uuids.clone())
            }
            Self::Leaf { .. } => None,
        }
    }

    /// Swap the children.
    pub fn children_swap(
        &self,
        new_children: ChildrenUUIDs,
    ) -> Result<ChildrenUUIDs, ChildrenUUIDs> {
        match self {
            Self::Root { ref children, .. } | Self::Node { ref children, .. } => {
                Ok(children.lock().replace(new_children))
            }
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

    ///Get the uuid for this item.
    pub fn uuid(&self) -> uuid::Uuid {
        match self {
            Self::Root { uuid, .. } => uuid.clone(),
            Self::Node { uuid, .. } => uuid.clone(),
            Self::Leaf { uuid, .. } => uuid.clone(),
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

impl SwapChildrenWithUUIDs {
    pub fn replace(&mut self, children: ChildrenUUIDs) -> ChildrenUUIDs {
        (
            self.children.replace(children.0),
            std::mem::replace(&mut self.uuids, children.1),
        )
    }

    pub fn children(&self) -> Arc<SwapChildren> {
        self.children.clone()
    }

    pub fn index_binding(&self) -> Arc<BindingSwapSet<usize>> {
        self.children.index_binding()
    }
}

impl Drop for GraphItem {
    fn drop(&mut self) {
        self.root_deactivate()
    }
}
