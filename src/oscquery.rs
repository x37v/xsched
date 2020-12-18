use crate::{binding::Binding, graph::GraphItem};
use oscquery::{root::NodeHandle, OscQueryServer};
use std::{collections::HashMap, net::SocketAddr, str::FromStr, sync::Arc};

pub struct OSCQueryHandler {
    bindings: std::sync::Mutex<HashMap<String, Arc<Binding>>>,
    graph: std::sync::Mutex<HashMap<String, Arc<GraphItem>>>,
    server: OscQueryServer,
    xsched_handle: NodeHandle,
    bindings_handle: NodeHandle,
    graph_handle: NodeHandle,
    handles: Vec<NodeHandle>,
}

impl OSCQueryHandler {
    pub fn new(
        _bindings: HashMap<String, Binding>,
        _graph: HashMap<String, GraphItem>,
    ) -> Result<Self, std::io::Error> {
        let server = OscQueryServer::new(
            Some("xsched".into()),
            &SocketAddr::from_str("0.0.0.0:3000").expect("failed to bind for http"),
            "0.0.0.0:3010",
            "0.0.0.0:3001",
        )?;
        let mut handles = Vec::new();
        let xsched_handle = server
            .add_node(
                oscquery::node::Container::new(
                    "xsched".into(),
                    Some("xsched scheduler root".into()),
                )
                .expect("to construct xsched")
                .into(),
                None,
            )
            .expect("to add handle");
        let bindings_base = server
            .add_node(
                oscquery::node::Container::new(
                    "bindings".into(),
                    Some("xsched scheduler bindings".into()),
                )
                .expect("to construct bindings")
                .into(),
                Some(xsched_handle),
            )
            .expect("to add handle");
        handles.push(bindings_base.clone());
        let bindings_handle = server
            .add_node(
                oscquery::node::Container::new("uuids".into(), Some("bindings by uuid".into()))
                    .expect("to construct bindings")
                    .into(),
                Some(bindings_base),
            )
            .expect("to add handle");
        //TODO aliases
        let graph_handle = server
            .add_node(
                oscquery::node::Container::new(
                    "graph".into(),
                    Some("xsched scheduler graph".into()),
                )
                .expect("to construct graph")
                .into(),
                Some(xsched_handle),
            )
            .expect("to add handle");
        let s = Self {
            server,
            xsched_handle,
            bindings_handle,
            graph_handle,
            bindings: Default::default(),
            graph: Default::default(),
            handles,
        };

        //TODO add bindings and graph
        Ok(s)
    }

    fn add_binding(&mut self, binding: Binding) {
        if let Ok(mut guard) = self.bindings.lock() {
            let uuids = binding.uuid().to_hyphenated().to_string();
            let binding = Arc::new(binding);
            guard.insert(uuids.clone(), binding.clone());

            //XXX do we need to keep track of the handle?
            let _handle = self
                .server
                .add_node(
                    oscquery::node::Container::new(uuids, None)
                        .expect("to construct binding")
                        .into(),
                    Some(self.bindings_handle),
                )
                .expect("to add node");
            /*
            let typ = self
                .server
                .add_node(
                    oscquery::node::Get::new(uuids, Some("xsched scheduler graph".into()))
                        .expect("to construct binding")
                        .into(),
                    Some(self.bindings_handle),
                )
                .expect("to add node");
            */
        }
    }
}
