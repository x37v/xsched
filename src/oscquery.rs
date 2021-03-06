use crate::{
    graph::{children::Children, GraphItem},
    param::{Param, ParamDataAccess, ParamMapGet},
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
    binding::bpm::{Clock, ClockData},
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

use serde::{Deserialize, Serialize};
use serde_json::value::Value as JsonValue;

#[derive(Clone, Debug, Deserialize, Serialize)]
enum ParamOwner {
    Param(uuid::Uuid),
    GraphItem(uuid::Uuid),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
enum GraphNodeChildren {
    None,
    NChild(uuid::Uuid),
    Indexed(Vec<uuid::Uuid>),
}

#[derive(Deserialize, Serialize)]
enum Command {
    Batch(Box<Vec<Command>>),
    ParamBind {
        owner: ParamOwner,
        param_name: String,
        param_id: uuid::Uuid,
    },
    ParamUnbind {
        owner: ParamOwner,
        param_name: String,
    },
    ParamCreate {
        id: Option<uuid::Uuid>,
        type_name: String,
        args: JsonValue,
        params: Option<HashMap<String, uuid::Uuid>>,
    },
    GraphItemCreate {
        id: Option<uuid::Uuid>,
        type_name: String,
        args: Option<JsonValue>,
        children: Option<GraphNodeChildren>,
        params: Option<HashMap<String, uuid::Uuid>>,
    },
    GraphNodeSetChildren {
        parent_id: uuid::Uuid,
        children: GraphNodeChildren,
    },
}

//wrapper to impl Get
struct ParamOSCQueryGet {
    key: &'static str,
    map: Weak<dyn ParamMapGet + Send + Sync>,
}

//wrapper to impl Get
struct GraphChildrenParamGet {
    owner: Weak<GraphItem>,
}

struct GraphChildrenTypeNameParamGet {
    owner: Weak<GraphItem>,
}

pub struct OSCQueryHandler {
    params: std::sync::Mutex<HashMap<uuid::Uuid, Arc<Param>>>,
    graph: std::sync::Mutex<HashMap<uuid::Uuid, Arc<GraphItem>>>,
    command_sender: SyncSender<Command>,
    server: OscQueryServer,
    _xsched_handle: NodeHandle,
    params_handle: NodeHandle,
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

impl ::oscquery::value::Get<String> for GraphChildrenTypeNameParamGet {
    fn get(&self) -> String {
        self.owner
            .upgrade()
            .map(|o| o.children_type_name())
            .flatten()
            .unwrap_or(Default::default())
            .to_string()
    }
}

impl OSCQueryHandler {
    pub fn new(
        queue_sources: Arc<dyn QueueSource>,
        _params: HashMap<String, Arc<Param>>,
        _graph: HashMap<String, GraphItem>,
    ) -> Result<Self, std::io::Error> {
        println!(
            "example command {}",
            serde_json::to_string(&Command::ParamBind {
                owner: ParamOwner::Param(uuid::Uuid::new_v4()),
                param_name: "toast".into(),
                param_id: uuid::Uuid::new_v4(),
            })
            .unwrap()
        );

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

        {
            let command_sender = command_sender.clone();
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
                                match args.first() {
                                    Some(OscType::String(v)) => {
                                        println!("got command {}", v);
                                        let cmd: Result<Command, _> =
                                            serde_json::from_str(v.as_str());
                                        if let Ok(cmd) = cmd {
                                            if command_sender.send(cmd).is_err() {
                                                eprintln!("error sending command");
                                            }
                                        } else {
                                            eprintln!(
                                                "failed to deserialize command from str {}",
                                                v
                                            );
                                        }
                                    }
                                    _ => (),
                                }
                                None
                            },
                        ))),
                    )
                    .unwrap(),
                    Some(xsched_handle.clone()),
                )
                .unwrap();
        }

        let params_base = server
            .add_node(
                oscquery::node::Container::new("params", Some("xsched scheduler params")).unwrap(),
                Some(xsched_handle),
            )
            .unwrap();
        let params_handle = server
            .add_node(
                oscquery::node::Container::new("uuids", Some("params by uuid")).unwrap(),
                Some(params_base),
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
            params_handle,
            graph_handle,
            params: Default::default(),
            graph: Default::default(),
            command_sender,
            command_receiver,
            sched_queue: queue_sources.sched_queue(),
            queue_sources,
        };

        //TODO add params and graph
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
            self.add_params(item.clone() as _, handle.clone());
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
                //children
                match item.as_ref() {
                    GraphItem::Leaf { .. } => (),
                    GraphItem::Root { .. } | GraphItem::Node { .. } => {
                        let children_handle = self
                            .server
                            .add_node(
                                oscquery::node::Container::new("children", None).unwrap(),
                                Some(handle),
                            )
                            .unwrap();
                        {
                            let wrapper = GraphChildrenParamGet {
                                owner: Arc::downgrade(&item),
                            };
                            let _ = self
                                .server
                                .add_node(
                                    ::oscquery::node::Get::new(
                                        "ids",
                                        Some("list of child uuids"),
                                        vec![::oscquery::param::ParamGet::Array(
                                            ValueBuilder::new(Arc::new(wrapper) as _).build(),
                                        )],
                                    )
                                    .unwrap(),
                                    Some(children_handle.clone()),
                                )
                                .unwrap();
                        }
                        {
                            let wrapper = GraphChildrenTypeNameParamGet {
                                owner: Arc::downgrade(&item),
                            };
                            let _ = self
                                .server
                                .add_node(
                                    ::oscquery::node::Get::new(
                                        "type",
                                        Some("children type name"),
                                        vec![::oscquery::param::ParamGet::String(
                                            ValueBuilder::new(Arc::new(wrapper) as _).build(),
                                        )],
                                    )
                                    .unwrap(),
                                    Some(children_handle),
                                )
                                .unwrap();
                        }
                    }
                };
            }
        }
    }

    pub fn add_param(&self, param: Arc<Param>) {
        if let Ok(mut guard) = self.params.lock() {
            guard.insert(param.uuid(), param.clone());
            let handle = self
                .server
                .add_node(
                    oscquery::node::Container::new(map_uuid(&param.uuid()), None).unwrap(),
                    Some(self.params_handle),
                )
                .unwrap();
            //value
            if let Some(shadow) = param.shadow() {
                self.add_param_value(&shadow, handle);
            }
            //type node
            {
                let _ = self
                    .server
                    .add_node(
                        oscquery::node::Get::new(
                            "type",
                            Some("type_name, access_name, data_type_name"),
                            vec![
                                param.type_name(),
                                param.access_name(),
                                param.data_type_name(),
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
            if !param.params().is_empty() {
                self.add_params(param.clone() as _, handle.clone());
            }
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
                        Some("param_id"),
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

    fn param_unbind(&self, owner: &ParamOwner, param_name: &str) {
        if let Ok(params_guard) = self.params.lock() {
            match owner {
                //bind parameters
                ParamOwner::Param(param_id) => {
                    if let Some(param) = params_guard.get(&param_id) {
                        param.params().unbind(param_name);
                        //get handle and self.server.trigger(handle);
                    }
                }
                ParamOwner::GraphItem(item_id) => {
                    if let Ok(graph_guard) = self.graph.lock() {
                        if let Some(item) = graph_guard.get(&item_id) {
                            item.params().unbind(param_name);
                            //get handle and self.server.trigger(handle);
                        }
                    }
                }
            }
        }
    }

    fn param_bind(&self, owner: &ParamOwner, param_name: &str, param_id: &uuid::Uuid) {
        if let Ok(params_guard) = self.params.lock() {
            match owner {
                //bind parameters
                ParamOwner::Param(param_id) => {
                    if let Some(parent) = params_guard.get(&param_id) {
                        //TODO cycle detection
                        //TODO error handling
                        if let Some(param) = params_guard.get(&param_id) {
                            let _r = parent.params().try_bind(param_name, param.clone());
                            //get handle and self.server.trigger(handle);
                        }
                    }
                }
                ParamOwner::GraphItem(item_id) => {
                    if let Ok(graph_guard) = self.graph.lock() {
                        if let Some(item) = graph_guard.get(&item_id) {
                            //TODO cycle detection
                            //TODO error handling
                            if let Some(param) = params_guard.get(&param_id) {
                                let _r = item.params().try_bind(param_name, param.clone());
                                //get handle and self.server.trigger(handle);
                            }
                        }
                    }
                }
            }
        }
    }

    fn param_create(
        &self,
        uuid: &Option<uuid::Uuid>,
        type_name: &str,
        args: JsonValue,
        params: &Option<HashMap<String, uuid::Uuid>>,
    ) {
        let uuid = uuid.map_or_else(|| uuid::Uuid::new_v4(), |u| u.clone());
        match crate::param::factory::create_param(&uuid, type_name, args) {
            Ok(inst) => {
                self.add_param(Arc::new(inst));
                if let Some(params) = params {
                    let owner = ParamOwner::Param(uuid);
                    for (name, id) in params.iter() {
                        self.param_bind(&owner, name, id);
                    }
                }
            }
            Err(e) => println!("error creating instance {}", e),
        }
    }

    fn graph_node_create(
        &self,
        uuid: &Option<uuid::Uuid>,
        type_name: &str,
        args: Option<JsonValue>,
        children: &Option<GraphNodeChildren>,
        params: &Option<HashMap<String, uuid::Uuid>>,
    ) {
        let uuid = uuid.map_or_else(|| uuid::Uuid::new_v4(), |u| u.clone());
        match crate::graph::factory::create_instance(&uuid, &type_name, args, &self.queue_sources) {
            Ok(item) => {
                self.add_graph_item(item);
                if let Some(children) = children {
                    self.graph_node_set_children(&uuid, children);
                }
                if let Some(params) = params {
                    let owner = ParamOwner::GraphItem(uuid.clone());
                    for (name, id) in params.iter() {
                        self.param_bind(&owner, name, id);
                    }
                }
            }
            Err(e) => println!("error creating instance {}", e),
        }
    }

    fn graph_node_set_children(&self, parent_id: &uuid::Uuid, children: &GraphNodeChildren) {
        if let Ok(guard) = self.graph.lock() {
            if let Some(parent) = guard.get(parent_id) {
                match children {
                    GraphNodeChildren::None => {
                        let _ = parent.children_swap((Arc::new(Children::None), vec![]));
                    }
                    GraphNodeChildren::Indexed(children_ids) => {
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
                        } else {
                            eprintln!("cannot find graph children with ids {:?}", children_ids);
                        }
                    }
                    GraphNodeChildren::NChild(child_id) => {
                        if let Some(child) = guard.get(&child_id).map(|c| c.get_node()).flatten() {
                            let _ = parent.children_swap((
                                Arc::new(Children::NChild { child }),
                                vec![child_id.clone()],
                            ));
                        } else {
                            eprintln!("cannot find graph child with id {:?}", child_id);
                        }
                    }
                }
            } else {
                eprintln!("cannot find graph parent with id {}", parent_id);
            }
        }
    }

    fn handle_command(&self, cmd: &Command) {
        match cmd {
            Command::Batch(b) => {
                for c in b.iter() {
                    self.handle_command(c);
                }
            }
            Command::ParamBind {
                owner,
                param_name,
                param_id,
            } => self.param_bind(&owner, param_name.as_str(), &param_id),
            Command::ParamUnbind { owner, param_name } => {
                self.param_unbind(owner, param_name.as_str())
            }
            Command::ParamCreate {
                id,
                type_name,
                args,
                params,
            } => self.param_create(id, type_name, args.clone(), params),
            Command::GraphItemCreate {
                id,
                type_name,
                args,
                children,
                params,
            } => self.graph_node_create(id, type_name, args.clone(), children, params),
            Command::GraphNodeSetChildren {
                parent_id,
                children,
            } => self.graph_node_set_children(parent_id, children),
        }
    }

    //TODO timeout?
    pub fn process(&mut self) {
        while let Ok(cmd) = self.command_receiver.try_recv() {
            self.handle_command(&cmd);
        }
    }
}

//pull in the codegen
include!(concat!(env!("OUT_DIR"), "/oscquery.rs"));
