use crate::{binding::Binding, graph::GraphItem, sched::Sched};
use oscquery::{root::NodeHandle, OscQueryServer};
use std::{
    collections::HashMap,
    net::SocketAddr,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

pub struct OSCQueryHandler {
    bindings: std::sync::Mutex<HashMap<&'static str, Binding>>,
    graph: std::sync::Mutex<HashMap<&'static str, GraphItem>>,
    server: OscQueryServer,
    xsched_handle: NodeHandle,
    bindings_handle: NodeHandle,
    graph_handle: NodeHandle,
    handles: Vec<NodeHandle>,
}

impl OSCQueryHandler {
    pub fn new(
        bindings: HashMap<&'static str, Binding>,
        graph: HashMap<&'static str, GraphItem>,
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
        Ok(Self {
            server,
            xsched_handle,
            bindings_handle,
            graph_handle,
            bindings: Mutex::new(bindings),
            graph: Mutex::new(graph),
            handles,
        })
    }
}
