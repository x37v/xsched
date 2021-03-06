use sched::{
    binding::{swap::BindingSwapSet, ParamBindingSet},
    event::EventEvalContext,
    graph::{ChildCount, GraphChildExec, GraphNode, GraphNodeContainer},
    mutex::Mutex,
};
use std::{ops::DerefMut, sync::Arc};

/// Graphi hcild
pub enum Children {
    /// None, yet
    None,
    /// A single child that acts like many.
    NChild {
        child: GraphNodeContainer,
    },
    Indexed {
        children: Vec<GraphNodeContainer>,
    },
}

/// A wrapper for children that has swappable members.
#[derive(Default)]
pub struct SwapChildren {
    children: Mutex<Arc<Children>>,
    index_binding: Arc<BindingSwapSet<usize>>,
}

/// A new type so we can implement GraphChildExec
pub struct SwapChildrenContainer(Mutex<Arc<SwapChildren>>);

impl SwapChildren {
    pub fn replace(&self, children: Arc<Children>) -> Arc<Children> {
        let mut g = self.children.lock();
        std::mem::replace(g.deref_mut(), children)
    }

    pub fn type_name(&self) -> &'static str {
        let g = self.children.lock();
        match g.as_ref() {
            Children::None => &"None",
            Children::NChild { .. } => &"NChild",
            Children::Indexed { .. } => &"Indexed",
        }
    }

    pub fn index_binding(&self) -> Arc<BindingSwapSet<usize>> {
        self.index_binding.clone()
    }
}

impl GraphChildExec for SwapChildren {
    fn child_count(&self) -> ChildCount {
        match self.children.lock().as_ref() {
            Children::None => ChildCount::None,
            Children::NChild { .. } => ChildCount::Inf,
            Children::Indexed { ref children, .. } => ChildCount::Some(children.len()),
        }
    }

    fn child_exec_range(&self, context: &mut dyn EventEvalContext, range: core::ops::Range<usize>) {
        match self.children.lock().as_ref() {
            Children::None => (),
            Children::NChild { child } => {
                for i in range {
                    self.index_binding.set(i);
                    child.node_exec(context);
                }
            }
            Children::Indexed { children } => {
                let (_, r) = children.split_at(range.start);
                let (r, _) = r.split_at(range.end - range.start);
                for (i, c) in r.iter().enumerate() {
                    self.index_binding.set(i + range.start);
                    c.node_exec(context);
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
        Self(Mutex::new(inner))
    }
}

impl GraphChildExec for SwapChildrenContainer {
    fn child_count(&self) -> ChildCount {
        self.0.lock().child_count()
    }

    fn child_exec_range(&self, context: &mut dyn EventEvalContext, range: core::ops::Range<usize>) {
        self.0.lock().child_exec_range(context, range)
    }
}
