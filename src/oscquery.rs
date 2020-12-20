use crate::{binding::Instance, graph::GraphItem, param::ParamMapGet};
use oscquery::{
    func_wrap::GetFunc,
    param::ParamGet,
    root::NodeHandle,
    value::{Get, ValueBuilder},
    OscQueryServer,
};
use std::{
    collections::HashMap,
    net::SocketAddr,
    str::FromStr,
    sync::{
        mpsc::{Receiver, SyncSender},
        Arc, Weak,
    },
};

enum Command {
    BindParam {
        binding_id: uuid::Uuid,
        param_name: &'static str,
        param_id: uuid::Uuid,
    },
}

//
struct ParamOSCQueryGetSet {
    command_sender: SyncSender<Command>,
    key: &'static str,
    map: Weak<dyn ParamMapGet + Send + Sync>,
}

pub struct OSCQueryHandler {
    bindings: std::sync::Mutex<HashMap<String, Arc<Instance>>>,
    graph: std::sync::Mutex<HashMap<String, Arc<GraphItem>>>,
    command_sender: SyncSender<Command>,
    command_receiver: Receiver<Command>,
    server: OscQueryServer,
    xsched_handle: NodeHandle,
    bindings_handle: NodeHandle,
    graph_handle: NodeHandle,
    handles: Vec<NodeHandle>,
}

impl ParamOSCQueryGetSet {
    fn new(
        command_sender: SyncSender<Command>,
        key: &'static str,
        map: &Arc<dyn ParamMapGet + Send + Sync>,
    ) -> Self {
        Self {
            command_sender,
            key,
            map: Arc::downgrade(map),
        }
    }
}

fn map_uuid(uuid: &uuid::Uuid) -> String {
    uuid.to_hyphenated().to_string()
}

impl ::oscquery::value::Get<String> for ParamOSCQueryGetSet {
    fn get(&self) -> String {
        self.map.upgrade().map_or("".into(), |m| {
            m.params()
                .uuid(self.key)
                .map_or("".into(), |u| map_uuid(&u))
        })
    }
}

impl ::oscquery::value::Set<String> for ParamOSCQueryGetSet {
    fn set(&self, value: String) {
        //XXX
    }
}

impl OSCQueryHandler {
    pub fn new(
        _bindings: HashMap<String, Arc<Instance>>,
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
                .unwrap()
                .into(),
                None,
            )
            .unwrap();
        let bindings_base = server
            .add_node(
                oscquery::node::Container::new(
                    "bindings".into(),
                    Some("xsched scheduler bindings".into()),
                )
                .unwrap()
                .into(),
                Some(xsched_handle),
            )
            .unwrap();
        handles.push(bindings_base.clone());
        let bindings_handle = server
            .add_node(
                oscquery::node::Container::new("uuids".into(), Some("bindings by uuid".into()))
                    .unwrap()
                    .into(),
                Some(bindings_base),
            )
            .unwrap();
        //TODO aliases
        let graph_handle = server
            .add_node(
                oscquery::node::Container::new(
                    "graph".into(),
                    Some("xsched scheduler graph".into()),
                )
                .unwrap()
                .into(),
                Some(xsched_handle),
            )
            .unwrap();
        let (command_sender, command_receiver) = std::sync::mpsc::sync_channel(256);
        let s = Self {
            server,
            xsched_handle,
            bindings_handle,
            graph_handle,
            bindings: Default::default(),
            graph: Default::default(),
            handles,
            command_sender,
            command_receiver,
        };

        //TODO add bindings and graph
        Ok(s)
    }

    pub fn add_binding(&self, binding: Arc<Instance>) {
        if let Ok(mut guard) = self.bindings.lock() {
            let uuids = map_uuid(&binding.uuid());
            guard.insert(uuids.clone(), binding.clone());

            //XXX do we need to keep track of the handle?
            let handle = self
                .server
                .add_node(
                    oscquery::node::Container::new(uuids, None).unwrap().into(),
                    Some(self.bindings_handle),
                )
                .unwrap();
            //type nodes
            {
                let weak = Arc::downgrade(&binding);
                let type_name = Arc::new(GetFunc::new(move || {
                    weak.upgrade().map_or("", |b| b.type_name()).to_string()
                })) as Arc<dyn Get<String>>;
                let weak = Arc::downgrade(&binding);
                let access_name = Arc::new(GetFunc::new(move || {
                    weak.upgrade().map_or("", |b| b.access_name()).to_string()
                })) as Arc<dyn Get<String>>;
                let weak = Arc::downgrade(&binding);
                let data_type_name = Arc::new(GetFunc::new(move || {
                    weak.upgrade()
                        .map_or("", |b| b.data_type_name())
                        .to_string()
                })) as Arc<dyn Get<String>>;
                let _ = self
                    .server
                    .add_node(
                        oscquery::node::Get::new(
                            "type".to_string(),
                            Some("type_name, access_name, data_type_name".into()),
                            vec![type_name, access_name, data_type_name]
                                .into_iter()
                                .map(|v| ParamGet::String(ValueBuilder::new(v as _).build())),
                        )
                        .unwrap()
                        .into(),
                        Some(handle),
                    )
                    .unwrap();
            }
            self.add_params(binding.clone() as _, handle.clone());
        }
    }

    fn add_params(
        &self,
        item: ::std::sync::Arc<dyn ParamMapGet + Send + Sync>,
        handle: ::oscquery::root::NodeHandle,
    ) {
        let phandle = self
            .server
            .add_node(
                ::oscquery::node::Container::new("params".to_string(), None)
                    .unwrap()
                    .into(),
                Some(handle.clone()),
            )
            .unwrap();
        let keys: Vec<_> = item.params().keys().into_iter().cloned().collect();
        for key in keys {
            let wrapper = Arc::new(ParamOSCQueryGetSet::new(
                self.command_sender.clone(),
                key,
                &item,
            ));
            let _ = self
                .server
                .add_node(
                    ::oscquery::node::GetSet::new(
                        key.to_string(),
                        Some("binding_id".into()),
                        vec![::oscquery::param::ParamGetSet::String(
                            ValueBuilder::new(wrapper as _).build(),
                        )],
                        None,
                    )
                    .unwrap()
                    .into(),
                    Some(phandle),
                )
                .unwrap();
        }
    }
}

//pull in the codegen
include!(concat!(env!("OUT_DIR"), "/oscquery.rs"));
