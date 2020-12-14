use sched::{
    binding::ParamBindingSet,
    event::EventEvalContext,
    graph::{ChildCount, GraphChildExec, GraphNode, GraphNodeContainer},
};
use std::sync::Arc;

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
        index_binding: Arc<dyn ParamBindingSet<usize>>,
    },
    Indexed {
        children: Vec<ChildWithUUID>,
        index_binding: Arc<dyn ParamBindingSet<usize>>,
    },
}

/// A wrapper for children that has swappable members.
#[derive(Default)]
pub struct SwapChildren {
    children: sched::mutex::Mutex<Children>,
    //XXX can we move the index_binding in here?
}

/// A new type so we can implement GraphChildExec
pub struct SwapChildrenContainer(Arc<SwapChildren>);

impl GraphChildExec for Children {
    fn child_count(&self) -> ChildCount {
        match self {
            Self::None => ChildCount::None,
            Self::NChild { .. } => ChildCount::Inf,
            Self::Indexed { children, .. } => ChildCount::Some(children.len()),
        }
    }

    fn child_exec_range(
        &mut self,
        context: &mut dyn EventEvalContext,
        range: core::ops::Range<usize>,
    ) {
        match self {
            Self::None => (),
            Self::NChild {
                child,
                index_binding,
            } => {
                for i in range {
                    index_binding.set(i);
                    child.inner.node_exec(context);
                }
            }
            Self::Indexed {
                children,
                index_binding,
            } => {
                let (_, r) = children.split_at_mut(range.start);
                let (r, _) = r.split_at_mut(range.end - range.start);
                for (i, c) in r.iter_mut().enumerate() {
                    index_binding.set(i + range.start);
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
    pub fn new(inner: Arc<SwapChildren>) -> Self {
        Self(inner)
    }
}

impl GraphChildExec for SwapChildrenContainer {
    fn child_count(&self) -> ChildCount {
        self.0.children.lock().child_count()
    }

    fn child_exec_range(
        &mut self,
        context: &mut dyn EventEvalContext,
        range: core::ops::Range<usize>,
    ) {
        self.0.children.lock().child_exec_range(context, range)
    }
}
