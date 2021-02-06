use crate::{
    binding::{Access, Instance},
    graph::{children::Children, GraphItem},
    param::ParamMapGet,
    sched::{EventQueue, QueueSource},
};
use oscquery::{
    func_wrap::{GetFunc, GetSetFuncs, OscUpdateFunc, SetFunc},
    osc::{OscArray, OscType},
    param::{ParamGet, ParamGetSet, ParamSet},
    root::{NodeHandle, OscWriteCallback},
    value::{ClipMode, Range, ValueBuilder},
    OscQueryServer,
};
use sched::{
    binding::{
        bpm::{Clock, ClockData},
        last::BindingLast,
        ParamBindingSet,
    },
    graph::GraphNodeContainer,
    pqueue::TickPriorityEnqueue,
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
    GraphItem(uuid::Uuid),
}

enum Command {
    ParamBind {
        owner: ParamOwner,
        handle: NodeHandle,
        param_name: &'static str,
        param_id: String,
    },
    BindingCreate {
        id: Option<uuid::Uuid>,
        type_name: String,
        args: String,
    },
    GraphItemCreate {
        id: Option<uuid::Uuid>,
        type_name: String,
        args: String,
    },
    GraphNodeSetChildren {
        parent_id: uuid::Uuid,
        children_ids: Vec<uuid::Uuid>,
    },
}

//wrapper to impl Get
struct ParamOSCQueryGet {
    key: &'static str,
    map: Weak<dyn ParamMapGet + Send + Sync>,
}

//wrapper to impl OscUpdate
struct ParamOSCQueryOscUpdate {
    owner: ParamOwner,
    command_sender: SyncSender<Command>,
    key: &'static str,
}

//wrapper to impl GetSet
struct GraphChildrenParamGet {
    owner: Weak<GraphItem>,
}

pub struct OSCQueryHandler {
    bindings: std::sync::Mutex<HashMap<uuid::Uuid, Arc<Instance>>>,
    graph: std::sync::Mutex<HashMap<uuid::Uuid, Arc<GraphItem>>>,
    command_sender: SyncSender<Command>,
    server: OscQueryServer,
    _xsched_handle: NodeHandle,
    bindings_handle: NodeHandle,
    graph_handle: NodeHandle,
    command_receiver: Receiver<Command>,
    sched_queue: EventQueue,
    queue_sources: Arc<dyn QueueSource>,
}

impl ParamOSCQueryGet {
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

impl ::oscquery::value::Get<String> for ParamOSCQueryGet {
    fn get(&self) -> String {
        self.map.upgrade().map_or("".into(), |m| {
            m.params()
                .uuid(self.key)
                .map_or("".into(), |u| map_uuid(&u))
        })
    }
}

impl ::oscquery::node::OscUpdate for ParamOSCQueryOscUpdate {
    fn osc_update(
        &self,
        args: &Vec<OscType>,
        _addr: Option<SocketAddr>,
        _time: Option<(u32, u32)>,
        handle: &NodeHandle,
    ) -> Option<oscquery::root::OscWriteCallback> {
        match args.first() {
            Some(OscType::String(v)) => {
                //println!("to bind {:?}, {} {}", self.owner, self.key, v);
                //TODO use 2nd arg as uuid for command response?
                //use time?
                self.command_sender
                    .send(Command::ParamBind {
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

impl ::oscquery::value::Get<OscArray> for GraphChildrenParamGet {
    fn get(&self) -> OscArray {
        let mut children = Vec::new();
        if let Some(owner) = self.owner.upgrade() {
            if let Some(uuids) = owner.children_uuids() {
                for i in uuids {
                    children.push(OscType::String(map_uuid(&i)));
                }
            }
        }
        OscArray { content: children }
    }
}

impl OSCQueryHandler {
    pub fn new(
        queue_sources: Arc<dyn QueueSource>,
        _bindings: HashMap<String, Arc<Instance>>,
        _graph: HashMap<String, GraphItem>,
    ) -> Result<Self, std::io::Error> {
        let server = OscQueryServer::new(
            Some("xsched".into()),
            &SocketAddr::from_str("0.0.0.0:3000").expect("failed to bind for http"),
            "0.0.0.0:3010",
            "0.0.0.0:3001",
        )?;
        let (command_sender, command_receiver) = std::sync::mpsc::sync_channel(256);

        let xsched_handle = server
            .add_node(
                oscquery::node::Container::new("xsched", Some("xsched scheduler root")).unwrap(),
                None,
            )
            .unwrap();

        let _ = server
            .add_node(
                ::oscquery::node::Set::new(
                    "command",
                    Some("json formatted string command"),
                    vec![ParamSet::String(
                        //handle in callback
                        ValueBuilder::new(Arc::new(()) as _).build(),
                    )],
                    Some(Box::new(OscUpdateFunc::new(
                        move |args: &Vec<OscType>,
                              _addr: Option<SocketAddr>,
                              _time: Option<(u32, u32)>,
                              _handle: &NodeHandle|
                              -> Option<OscWriteCallback> {
                            //TODO
                            None
                        },
                    ))),
                )
                .unwrap(),
                Some(xsched_handle.clone()),
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
        let graph_base = server
            .add_node(
                oscquery::node::Container::new("graph", Some("xsched scheduler graph")).unwrap(),
                Some(xsched_handle),
            )
            .unwrap();

        let graph_handle = server
            .add_node(
                oscquery::node::Container::new("uuids", Some("xsched scheduler graph uuids"))
                    .unwrap(),
                Some(graph_base),
            )
            .unwrap();

        let s = Self {
            server,
            _xsched_handle: xsched_handle,
            bindings_handle,
            graph_handle,
            bindings: Default::default(),
            graph: Default::default(),
            command_sender,
            command_receiver,
            sched_queue: queue_sources.sched_queue(),
            queue_sources,
        };

        //TODO add bindings and graph
        Ok(s)
    }

    pub fn add_graph_item(&self, item: GraphItem) {
        let item = Arc::new(item);
        if let Ok(mut guard) = self.graph.lock() {
            let handle = self
                .server
                .add_node(
                    oscquery::node::Container::new(map_uuid(&item.uuid()), None).unwrap(),
                    Some(self.graph_handle),
                )
                .unwrap();
            //type node
            {
                let _ = self
                    .server
                    .add_node(
                        oscquery::node::Get::new(
                            "type",
                            Some("type_name"),
                            vec![ParamGet::String(
                                ValueBuilder::new(Arc::new(item.type_name()) as _).build(),
                            )],
                        )
                        .unwrap(),
                        Some(handle),
                    )
                    .unwrap();
            }
            //TODO activation control
            self.add_params(
                ParamOwner::GraphItem(item.uuid().clone()),
                item.clone() as _,
                handle.clone(),
            );
            guard.insert(item.uuid(), item.clone());

            //TODO use some config to decide if we should start the event immediately
            if let Some(e) = item.root_event() {
                self.sched_queue
                    .lock()
                    .enqueue(0, e)
                    .ok()
                    .expect("to be able to schedule root event");
            }

            {
                let parent_id = item.uuid();
                let wrapper = GraphChildrenParamGet {
                    owner: Arc::downgrade(&item),
                };
                //children
                match item.as_ref() {
                    GraphItem::Leaf { .. } => (),
                    GraphItem::Root { .. } | GraphItem::Node { .. } => {
                        let command_sender = self.command_sender.clone();
                        let _ = self
                            .server
                            .add_node(
                                ::oscquery::node::Get::new(
                                    "children",
                                    Some("list of child uuids"),
                                    vec![::oscquery::param::ParamGet::Array(
                                        ValueBuilder::new(Arc::new(wrapper) as _).build(),
                                    )],
                                )
                                .unwrap(),
                                Some(handle),
                            )
                            .unwrap();
                    }
                };
            }
        }
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
            //type node
            {
                let _ = self
                    .server
                    .add_node(
                        oscquery::node::Get::new(
                            "type",
                            Some("type_name, access_name, data_type_name"),
                            vec![
                                binding.type_name(),
                                binding.access_name(),
                                binding.data_type_name(),
                            ]
                            .into_iter()
                            .map(|v| ParamGet::String(ValueBuilder::new(Arc::new(v) as _).build())),
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
            let wrapper = Arc::new(ParamOSCQueryGet::new(key, &item));
            let _ = self
                .server
                .add_node(
                    ::oscquery::node::Get::new(
                        key,
                        Some("binding_id"),
                        vec![::oscquery::param::ParamGet::String(
                            ValueBuilder::new(wrapper as _).build(),
                        )],
                    )
                    .unwrap(),
                    Some(phandle),
                )
                .unwrap();
        }
    }

    fn param_bind(
        &self,
        owner: ParamOwner,
        handle: NodeHandle,
        param_name: &'static str,
        param_id: String,
    ) {
        if let Ok(bindings_guard) = self.bindings.lock() {
            match owner {
                //bind parameters
                ParamOwner::Binding(binding_id) => {
                    if let Some(binding) = bindings_guard.get(&binding_id) {
                        if param_id.is_empty() {
                            binding.params().unbind(param_name);
                        } else {
                            if let Ok(param_id) = ::uuid::Uuid::from_str(&param_id) {
                                //TODO cycle detection
                                //TODO error handling
                                if let Some(param) = bindings_guard.get(&param_id) {
                                    let _r = binding.params().try_bind(param_name, param.clone());
                                }
                            }
                        }
                        self.server.trigger(handle);
                    }
                }
                ParamOwner::GraphItem(item_id) => {
                    if let Ok(graph_guard) = self.graph.lock() {
                        if let Some(item) = graph_guard.get(&item_id) {
                            if param_id.is_empty() {
                                item.params().unbind(param_name);
                            } else {
                                if let Ok(param_id) = ::uuid::Uuid::from_str(&param_id) {
                                    //TODO error handling
                                    if let Some(param) = bindings_guard.get(&param_id) {
                                        let _r = item.params().try_bind(param_name, param.clone());
                                    }
                                }
                            }
                            self.server.trigger(handle);
                        }
                    }
                }
            }
        }
    }

    fn binding_create(&self, uuid: Option<uuid::Uuid>, type_name: String, args: String) {
        let uuid = uuid.unwrap_or_else(|| uuid::Uuid::new_v4());
        match crate::binding::factory::create_instance(uuid, &type_name, &args) {
            Ok(inst) => {
                self.add_binding(Arc::new(inst));
            }
            Err(e) => println!("error creating instance {}", e),
        }
    }

    fn graph_node_create(&self, uuid: Option<uuid::Uuid>, type_name: String, args: String) {
        let uuid = uuid.unwrap_or_else(|| uuid::Uuid::new_v4());
        match crate::graph::factory::create_instance(uuid, &type_name, &args, &self.queue_sources) {
            Ok(item) => {
                self.add_graph_item(item);
            }
            Err(e) => println!("error creating instance {}", e),
        }
    }

    //TODO set child type
    fn graph_node_set_children(&self, parent_id: uuid::Uuid, children_ids: Vec<uuid::Uuid>) {
        if let Ok(guard) = self.graph.lock() {
            if let Some(parent) = guard.get(&parent_id) {
                let children: Option<Vec<GraphNodeContainer>> = children_ids
                    .iter()
                    .map(|id| {
                        guard
                            .get(&id)
                            .map(|i| i.get_node().map(|n| n.clone()))
                            .flatten()
                    })
                    .collect();
                if let Some(children) = children {
                    let _ = parent.children_swap((
                        Arc::new(Children::Indexed { children }),
                        children_ids.clone(),
                    ));
                //TODO trigger handle, report error
                } else {
                    eprintln!("cannot find graph children with ids {:?}", children_ids);
                }
            } else {
                eprintln!("cannot find graph parent with id {}", parent_id);
            }
        }
    }

    fn handle_command(&self, cmd: Command) {
        match cmd {
            Command::ParamBind {
                owner,
                handle,
                param_name,
                param_id,
            } => self.param_bind(owner, handle, param_name, param_id),
            Command::BindingCreate {
                id,
                type_name,
                args,
            } => self.binding_create(id, type_name, args),
            Command::GraphItemCreate {
                id,
                type_name,
                args,
            } => self.graph_node_create(id, type_name, args),
            Command::GraphNodeSetChildren {
                parent_id,
                children_ids,
            } => self.graph_node_set_children(parent_id, children_ids),
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
