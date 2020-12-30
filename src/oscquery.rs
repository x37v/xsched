use crate::{
    binding::{Access, Instance},
    graph::GraphItem,
    param::ParamMapGet,
};
use oscquery::{
    func_wrap::{GetFunc, GetSetFuncs, OscUpdateFunc, SetFunc},
    node::{Container as _, Get as _, GetSet as _, Set as _},
    param::{ParamGet, ParamGetSet, ParamSet},
    root::{NodeHandle, OscQueryGraph, OscWriteCallback},
    value::{ClipMode, Get, Range, Value, ValueBuilder},
    OscQueryServer,
};
use sched::{
    binding::{
        bpm::{Clock, ClockData},
        last::BindingLast,
        ParamBindingGet, ParamBindingSet,
    },
    tick::TickResched,
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

#[derive(Clone, Debug)]
enum ParamOwner {
    Binding(uuid::Uuid),
}

enum Command {
    BindParam {
        owner: ParamOwner,
        handle: NodeHandle,
        param_name: &'static str,
        param_id: String,
    },
}

//wrapper to impl get
struct ParamOSCQueryGetSet {
    key: &'static str,
    map: Weak<dyn ParamMapGet + Send + Sync>,
}

//wrapper to impl OscUpdate
struct ParamOSCQueryOscUpdate {
    owner: ParamOwner,
    command_sender: SyncSender<Command>,
    key: &'static str,
}

pub struct OSCQueryHandler {
    bindings: std::sync::Mutex<HashMap<uuid::Uuid, Arc<Instance>>>,
    graph: std::sync::Mutex<HashMap<String, Arc<GraphItem>>>,
    command_sender: SyncSender<Command>,
    server: OscQueryServer,
    xsched_handle: NodeHandle,
    bindings_handle: NodeHandle,
    graph_handle: NodeHandle,
    command_receiver: Receiver<Command>,
}

impl ParamOSCQueryGetSet {
    fn new(key: &'static str, map: &Arc<dyn ParamMapGet + Send + Sync>) -> Self {
        Self {
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
    fn set(&self, _value: String) {
        //use the oscquery handler
    }
}

impl ::oscquery::node::OscUpdate for ParamOSCQueryOscUpdate {
    fn osc_update(
        &self,
        args: &Vec<oscquery::osc::OscType>,
        _addr: Option<SocketAddr>,
        _time: Option<(u32, u32)>,
        handle: &NodeHandle,
    ) -> Option<oscquery::root::OscWriteCallback> {
        match args.first() {
            Some(::oscquery::osc::OscType::String(v)) => {
                //println!("to bind {:?}, {} {}", self.owner, self.key, v);
                //TODO use 2nd arg as uuid for command response?
                //use time?
                self.command_sender
                    .send(Command::BindParam {
                        owner: self.owner.clone(),
                        handle: handle.clone(),
                        param_name: self.key,
                        param_id: v.clone(),
                    })
                    .expect("to send command");
            }
            _ => (),
        }
        None
    }
}

impl ParamOSCQueryOscUpdate {
    pub fn new(owner: ParamOwner, command_sender: SyncSender<Command>, key: &'static str) -> Self {
        Self {
            owner,
            command_sender,
            key,
        }
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
        let xsched_handle = server
            .add_node(
                oscquery::node::Container::new("xsched", Some("xsched scheduler root")).unwrap(),
                None,
            )
            .unwrap();
        let bindings_base = server
            .add_node(
                oscquery::node::Container::new("bindings", Some("xsched scheduler bindings"))
                    .unwrap(),
                Some(xsched_handle),
            )
            .unwrap();
        let bindings_handle = server
            .add_node(
                oscquery::node::Container::new("uuids", Some("bindings by uuid")).unwrap(),
                Some(bindings_base),
            )
            .unwrap();
        //TODO aliases
        let graph_handle = server
            .add_node(
                oscquery::node::Container::new("graph", Some("xsched scheduler graph")).unwrap(),
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
            command_sender,
            command_receiver,
        };

        //TODO add bindings and graph
        Ok(s)
    }

    pub fn add_binding(&self, binding: Arc<Instance>) {
        if let Ok(mut guard) = self.bindings.lock() {
            guard.insert(binding.uuid(), binding.clone());
            let handle = self
                .server
                .add_node(
                    oscquery::node::Container::new(map_uuid(&binding.uuid()), None).unwrap(),
                    Some(self.bindings_handle),
                )
                .unwrap();
            //value
            self.add_binding_value(&binding, handle);
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
                            "type",
                            Some("type_name, access_name, data_type_name"),
                            vec![type_name, access_name, data_type_name]
                                .into_iter()
                                .map(|v| ParamGet::String(ValueBuilder::new(v as _).build())),
                        )
                        .unwrap(),
                        Some(handle),
                    )
                    .unwrap();
            }
            //parameters
            self.add_params(
                ParamOwner::Binding(binding.uuid().clone()),
                binding.clone() as _,
                handle.clone(),
            );
        }
    }

    fn add_params(
        &self,
        owner: ParamOwner,
        item: ::std::sync::Arc<dyn ParamMapGet + Send + Sync>,
        handle: ::oscquery::root::NodeHandle,
    ) {
        let phandle = self
            .server
            .add_node(
                ::oscquery::node::Container::new("params", None).unwrap(),
                Some(handle.clone()),
            )
            .unwrap();
        let keys: Vec<_> = item.params().keys().into_iter().cloned().collect();
        for key in keys {
            let wrapper = Arc::new(ParamOSCQueryGetSet::new(key, &item));
            let handler = Box::new(ParamOSCQueryOscUpdate::new(
                owner.clone(),
                self.command_sender.clone(),
                key,
            ));
            let _ = self
                .server
                .add_node(
                    ::oscquery::node::GetSet::new(
                        key,
                        Some("binding_id"),
                        vec![::oscquery::param::ParamGetSet::String(
                            ValueBuilder::new(wrapper as _).build(),
                        )],
                        Some(handler),
                    )
                    .unwrap(),
                    Some(phandle),
                )
                .unwrap();
        }
    }

    fn bind_param(
        &self,
        owner: ParamOwner,
        handle: NodeHandle,
        param_name: &'static str,
        param_id: String,
    ) {
        if let Ok(guard) = self.bindings.lock() {
            match owner {
                //bind parameters
                ParamOwner::Binding(binding_id) => {
                    if let Some(binding) = guard.get(&binding_id) {
                        if param_id.is_empty() {
                            binding.params().unbind(param_name);
                        } else {
                            if let Ok(param_id) = ::uuid::Uuid::from_str(&param_id) {
                                //TODO cycle detection
                                //TODO error handling
                                if let Some(param) = guard.get(&param_id) {
                                    let _r = binding.params().try_bind(param_name, param.clone());
                                }
                            }
                        }
                        self.server.trigger(handle);
                    }
                }
            }
        }
    }

    fn handle_command(&self, cmd: Command) {
        match cmd {
            Command::BindParam {
                owner,
                handle,
                param_name,
                param_id,
            } => self.bind_param(owner, handle, param_name, param_id),
        }
    }

    //TODO timeout?
    pub fn process(&mut self) {
        while let Ok(cmd) = self.command_receiver.try_recv() {
            self.handle_command(cmd);
        }
    }
}

//pull in the codegen
include!(concat!(env!("OUT_DIR"), "/oscquery.rs"));
