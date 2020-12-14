use sched::{
    binding::{swap::BindingSwapSet, ParamBindingSet},
    event::EventEvalContext,
    graph::{ChildCount, GraphChildExec, GraphNode, GraphNodeContainer},
    mutex::Mutex,
};
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

pub type ALockChildren = Arc<Mutex<Children>>;

pub struct ChildWithUUID {
    pub inner: GraphNodeContainer,
    pub uuid: uuid::Uuid,
}

pub enum Children {
    /// None, yet
    None,
    /// A single child that acts like many.
    NChild {
        child: ChildWithUUID,
    },
    Indexed {
        children: Vec<ChildWithUUID>,
    },
}

/// A wrapper for children that has swappable members.
#[derive(Default)]
pub struct SwapChildren {
    children: Arc<Mutex<Children>>,
    index_binding: Arc<BindingSwapSet<usize>>,
}

/// A new type so we can implement GraphChildExec
pub struct SwapChildrenContainer(Arc<Mutex<SwapChildren>>);

impl SwapChildren {
    pub fn swap(&mut self, children: ALockChildren) -> ALockChildren {
        std::mem::replace(&mut self.children, children)
    }

    pub fn index_binding(&self) -> Arc<BindingSwapSet<usize>> {
        self.index_binding.clone()
    }
}

impl GraphChildExec for SwapChildren {
    fn child_count(&self) -> ChildCount {
        match self.children.lock().deref() {
            Children::None => ChildCount::None,
            Children::NChild { .. } => ChildCount::Inf,
            Children::Indexed { children, .. } => ChildCount::Some(children.len()),
        }
    }

    fn child_exec_range(
        &mut self,
        context: &mut dyn EventEvalContext,
        range: core::ops::Range<usize>,
    ) {
        match self.children.lock().deref_mut() {
            Children::None => (),
            Children::NChild { child } => {
                for i in range {
                    self.index_binding.set(i);
                    child.inner.node_exec(context);
                }
            }
            Children::Indexed { children } => {
                let (_, r) = children.split_at_mut(range.start);
                let (r, _) = r.split_at_mut(range.end - range.start);
                for (i, c) in r.iter_mut().enumerate() {
                    self.index_binding.set(i + range.start);
                    c.inner.node_exec(context);
                }
            }
        }
    }
}

impl Default for Children {
    fn default() -> Self {
        Self::None
    }
}

impl SwapChildrenContainer {
    pub fn new(inner: Arc<Mutex<SwapChildren>>) -> Self {
        Self(inner)
    }
}

impl GraphChildExec for SwapChildrenContainer {
    fn child_count(&self) -> ChildCount {
        self.0.lock().child_count()
    }

    fn child_exec_range(
        &mut self,
        context: &mut dyn EventEvalContext,
        range: core::ops::Range<usize>,
    ) {
        self.0.lock().child_exec_range(context, range)
    }
}
